import { createServer } from 'node:https';
import { request } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startProcess, testEnvironment, type ManagedProcess } from './lib/process.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const children: ManagedProcess[] = [];
const proxy = createServer(
  {
    key: await readFile(resolve(root, 'tests/fixtures/tls/key.pem')),
    cert: await readFile(resolve(root, 'tests/fixtures/tls/cert.pem'))
  },
  (incoming, response) => {
    const upstream = request(
      {
        host: '127.0.0.1',
        port: 5186,
        method: incoming.method,
        path: incoming.url,
        headers: incoming.headers
      },
      (reply) => {
        response.writeHead(reply.statusCode ?? 502, reply.headers);
        reply.pipe(response);
      }
    );
    upstream.on('error', () => {
      if (!response.headersSent) response.writeHead(502);
      response.end('Fixture proxy unavailable.');
    });
    incoming.on('aborted', () => upstream.destroy());
    response.on('close', () => {
      if (!response.writableFinished) upstream.destroy();
    });
    incoming.pipe(upstream);
  }
);
let stopping = false;
async function stop(code: number) {
  if (stopping) return;
  stopping = true;
  proxy.closeAllConnections();
  await new Promise<void>((resolve) => {
    if (proxy.listening) proxy.close(() => resolve());
    else resolve();
  });
  await Promise.all(children.map((child) => child.stop()));
  process.exitCode = code;
}
for (const signal of ['SIGINT', 'SIGTERM'] as const)
  process.once(signal, () => {
    void stop(0);
  });
try {
  await new Promise<void>((resolve, reject) => {
    proxy.once('error', reject);
    proxy.listen(5185, '127.0.0.1', resolve);
  });
  const env = testEnvironment();
  children.push(
    startProcess(resolve(root, 'release/branch-backend'), [], {
      cwd: root,
      env: { ...env, APP_ORIGIN: 'http://127.0.0.1:5183', PORT: '5183' }
    })
  );
  children.push(
    startProcess(
      'cargo',
      [
        'test',
        '--manifest-path',
        'backend/Cargo.toml',
        '--locked',
        '--test',
        'browser_server',
        'serve_browser_fixtures',
        '--',
        '--ignored',
        '--nocapture'
      ],
      { cwd: root, env }
    )
  );
  for (const child of children)
    void child.exited
      .then((code) => {
        if (!stopping) void stop(code || 1);
      })
      .catch(() => {
        void stop(1);
      });
  console.log('Launched isolated Rust production and provider-fixture servers.');
} catch {
  await stop(1);
}
