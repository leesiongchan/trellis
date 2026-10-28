import { createServer } from 'node:https';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { request } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
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
const dataRoot = await mkdtemp(resolve(tmpdir(), 'branch-browser-release-'));
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
  await rm(dataRoot, { recursive: true, force: true });
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
      singleProcess: true,
      env: {
        ...env,
        APP_ORIGIN: 'http://127.0.0.1:5183',
        PORT: '5183',
        APP_DATA_DIR: resolve(dataRoot, 'data')
      }
    })
  );
  const build = await promisify(execFile)(
    'cargo',
    [
      'test',
      '--manifest-path',
      'api/Cargo.toml',
      '--locked',
      '--test',
      'browser_server',
      '--no-run',
      '--message-format=json'
    ],
    { cwd: root, env, maxBuffer: 16 * 1024 * 1024 }
  );
  const fixture = build.stdout
    .split('\n')
    .flatMap((line) => {
      if (!line.trim()) return [];
      const artifact = JSON.parse(line);
      return artifact.reason === 'compiler-artifact' &&
        artifact.target?.name === 'browser_server' &&
        artifact.profile?.test &&
        typeof artifact.executable === 'string'
        ? [artifact.executable as string]
        : [];
    })
    .at(-1);
  if (!fixture) throw new Error('Native browser fixture executable was not built.');
  children.push(
    startProcess(fixture, ['serve_browser_fixtures', '--ignored', '--nocapture'], {
      cwd: root,
      env,
      singleProcess: true
    })
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
