import { access, readFile, mkdtemp, rm } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { createServer } from 'node:net';
import { spawn } from 'node:child_process';
import { randomBytes, createHash } from 'node:crypto';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const binaryFlag = process.argv.indexOf('--binary');
const binary = resolve(
  binaryFlag < 0 ? 'release/branch-backend' : (process.argv[binaryFlag + 1] ?? '')
);
function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  check(address && typeof address !== 'string', 'No local test port.');
  await new Promise<void>((resolve) => server.close(() => resolve()));
  return address.port;
}
async function verify() {
  await access(binary);
  const manifest = JSON.parse(
    await readFile(resolve(root, 'release/frontend-manifest.json'), 'utf8')
  );
  const port = await freePort(),
    origin = 'http://127.0.0.1:' + port;
  const cwd = await mkdtemp(resolve(tmpdir(), 'branch-release-check-'));
  const key = 'verify-' + randomBytes(32).toString('hex'),
    question = 'private-verification-' + randomBytes(12).toString('hex');
  const env = {
    PATH: '',
    APP_ENV_FILE: '',
    APP_ORIGIN: origin,
    PORT: String(port),
    HOST: '127.0.0.1',
    APP_ACCESS_KEY: key,
    OPENAI_API_KEY: '',
    TAVILY_API_KEY: '',
    RUST_LOG: 'info',
    FRONTEND_DIR: resolve(root, 'release/public')
  };
  const child = spawn(binary, [], { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let logs = '',
    overflow = false;
  for (const stream of [child.stdout, child.stderr])
    stream.on('data', (chunk) => {
      if (logs.length + chunk.length > 1024 * 1024) {
        overflow = true;
        child.kill('SIGTERM');
      } else logs += chunk.toString();
    });
  const closed = new Promise<number>((resolve, reject) => {
    child.once('error', () => reject(new Error('Release executable could not start.')));
    child.once('close', (code) => resolve(code ?? 1));
  });
  // Attach an early handler so a missing interpreter cannot become an unhandled rejection.
  void closed.catch(() => {});
  const checks: string[] = [];
  const fetchLocal = (path: string, options: RequestInit = {}) =>
    fetch(origin + path, { ...options, signal: AbortSignal.timeout(2000) });
  try {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      check(
        child.exitCode === null && child.signalCode === null,
        'Release executable exited before readiness.'
      );
      try {
        const response = await fetchLocal('/health/ready');
        if (response.ok) {
          const value = await response.json();
          check(
            value.ready === true && value.researchConfigured === false,
            'Read-only readiness is incorrect.'
          );
          ready = true;
          break;
        }
      } catch {
        /* Wait for this owned child only. */
      }
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    check(ready, 'Release did not become ready.');
    checks.push('no-node-runtime', 'unrelated-cwd', 'research-disabled');
    const html = await fetchLocal('/');
    const body = await html.text();
    check(
      html.status === 200 && html.headers.get('content-type')?.includes('text/html'),
      'Static app shell is unavailable.'
    );
    check(
      html.headers.get('x-content-type-options') === 'nosniff' &&
        html.headers.get('content-security-policy')?.includes('frame-ancestors'),
      'Security headers are missing.'
    );
    const invalid = await fetchLocal('/access?key=' + encodeURIComponent(question), {
      redirect: 'manual'
    });
    check(invalid.status === 401, 'Invalid access key was accepted.');
    const access = await fetchLocal('/access?key=' + encodeURIComponent(key), {
      redirect: 'manual'
    });
    check(
      access.status === 303 && access.headers.get('location') === '/',
      'Access redirect is incorrect.'
    );
    const setCookie = access.headers.get('set-cookie');
    check(
      setCookie !== null && setCookie.includes('HttpOnly') && setCookie.includes('SameSite=Strict'),
      'Access cookie flags are incorrect.'
    );
    const cookie = setCookie.split(';')[0];
    const input = {
      explorationId: 'verify',
      nodeId: 'root',
      question,
      clarification: null,
      ancestors: []
    };
    const anonymous = await fetchLocal('/api/research', {
      method: 'POST',
      headers: { origin, 'content-type': 'application/json' },
      body: JSON.stringify(input)
    });
    check(anonymous.status === 401, 'Research is not private.');
    const research = await fetchLocal('/api/research', {
      method: 'POST',
      headers: {
        origin,
        cookie,
        'content-type': 'application/json',
        'x-request-id': 'release-check'
      },
      body: JSON.stringify(input)
    });
    const error = await research.json();
    check(
      research.status === 503 &&
        error.error?.code === 'SETUP_REQUIRED' &&
        error.error.requestId === research.headers.get('x-request-id'),
      'Missing providers did not fail safely.'
    );
    check((await fetchLocal('/metrics')).status === 401, 'Metrics are not private.');
    const metricsResponse = await fetchLocal('/metrics', { headers: { cookie } });
    const metrics = await metricsResponse.text();
    check(
      metricsResponse.status === 200 &&
        metrics.includes('branch_http_requests') &&
        metrics.includes('branch_active_jobs 0'),
      'Private metrics are unavailable.'
    );
    checks.push('private-access', 'private-metrics');
    for (const file of manifest.files) {
      check(
        typeof file.path === 'string' &&
          !file.path.split('/').some((part: string) => part.startsWith('.')) &&
          !/\.(map|ts|svelte|pem|key)$/.test(file.path),
        'Unsafe release asset.'
      );
      const path = resolve(root, 'release/public', file.path);
      check(path.startsWith(resolve(root, 'release/public') + '/'), 'Escaped release asset path.');
      const bytes = await readFile(path);
      check(
        createHash('sha256').update(bytes).digest('hex') === file.sha256,
        'Release asset digest mismatch.'
      );
      check(
        !bytes.includes(Buffer.from(key)) && !bytes.includes(Buffer.from(question)),
        'Private values leaked into release assets.'
      );
    }
    const csp = html.headers.get('content-security-policy') ?? '';
    for (const match of body.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script\s*>/gi))
      if (!/(?:^|\s)src\s*=/i.test(match[1]))
        check(
          csp.includes('sha256-' + createHash('sha256').update(match[2]).digest('base64')),
          'Inline script CSP hash missing.'
        );
    checks.push('asset-integrity');
    child.kill('SIGTERM');
    const timeout = new Promise<never>((_, reject) => {
      const timer = setTimeout(
        () => reject(new Error('Idle release did not stop after SIGTERM.')),
        5000
      );
      timer.unref();
    });
    check((await Promise.race([closed, timeout])) === 0, 'Release did not exit cleanly.');
    checks.push('sigterm-exit');
    check(
      !overflow && !logs.includes(key) && !logs.includes(question),
      'Private values leaked into runtime logs.'
    );
    check(logs.includes('shutdown_completed'), 'Shutdown was not observable.');
    checks.push('secret-redaction');
    console.log(JSON.stringify({ ok: true, checks, platform: process.platform }));
  } finally {
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    await closed.catch(() => {});
    await rm(cwd, { recursive: true, force: true });
  }
}
try {
  await verify();
} catch {
  console.error(
    'Release verification failed; check the artifact, readiness, access, integrity and shutdown checks.'
  );
  process.exitCode = 1;
}
