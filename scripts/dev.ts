import { access } from 'node:fs/promises';
import { resolve, dirname, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startProcess, type ManagedProcess } from './lib/process.ts';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const children: ManagedProcess[] = [];
if (!process.env.APP_DATA_DIR || !isAbsolute(process.env.APP_DATA_DIR))
  throw new Error(
    'Set APP_DATA_DIR to an absolute private directory outside the checkout before starting development.'
  );
const portFlag = process.argv.indexOf('--port');
const port = portFlag < 0 ? 5173 : Number(process.argv[portFlag + 1]);
if (!Number.isInteger(port) || port < 1 || port > 65535 || port === 3001)
  throw new Error('Choose a valid frontend port other than backend port 3001.');
await access(resolve(root, 'release/frontend-manifest.json'));
let stopping = false;
async function stop(code: number) {
  if (stopping) return;
  stopping = true;
  await Promise.all(children.map((child) => child.stop()));
  process.exitCode = code;
}
for (const signal of ['SIGINT', 'SIGTERM'] as const)
  process.once(signal, () => {
    void stop(0);
  });
const origin = 'http://127.0.0.1:' + port;
try {
  children.push(
    startProcess('cargo', ['run', '--manifest-path', 'api/Cargo.toml', '--locked'], {
      cwd: root,
      env: {
        ...process.env,
        APP_ORIGIN: origin,
        PORT: '3001',
        HOST: '127.0.0.1',
        FRONTEND_DIR: resolve(root, 'release/public')
      }
    })
  );
  children.push(
    startProcess(
      process.execPath,
      [
        resolve(root, 'app/node_modules/vite-plus/bin/vp'),
        'dev',
        '--host',
        '127.0.0.1',
        '--port',
        String(port),
        '--strictPort'
      ],
      { cwd: resolve(root, 'app'), env: { ...process.env } }
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
} catch {
  await stop(1);
}
