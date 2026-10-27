import { access, readFile, mkdtemp, rm, mkdir, writeFile } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { platform } from 'node:process';
import { createServer } from 'node:net';
import { spawn, type ChildProcess } from 'node:child_process';
import { randomBytes, randomUUID, createHash } from 'node:crypto';
import { canonicalLegacyJson, legacyIdentity } from '../src/lib/knowledge/legacy-identity.ts';
import type { Exploration } from '../src/lib/knowledge/types.ts';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const binaryFlag = process.argv.indexOf('--binary');
const binary = resolve(
  binaryFlag < 0 ? 'release/branch-backend' : (process.argv[binaryFlag + 1] ?? '')
);
function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
async function freePort() {
  const server = createServer();
  await new Promise<void>((yes, no) => {
    server.once('error', no);
    server.listen(0, '127.0.0.1', yes);
  });
  const address = server.address();
  check(address && typeof address !== 'string', 'No local test port.');
  await new Promise<void>((yes) => server.close(() => yes()));
  return address.port;
}
async function verify() {
  await access(binary);
  const releaseManifest = JSON.parse(
    await readFile(join(root, 'release/release-manifest.json'), 'utf8')
  );
  check(
    releaseManifest.protocolVersion === 2 &&
      releaseManifest.storageSchemaVersion === 1 &&
      releaseManifest.engineVersion === '3.3.4' &&
      releaseManifest.executableSha256 ===
        createHash('sha256')
          .update(await readFile(binary))
          .digest('hex'),
    'Executable/protocol manifest mismatch.'
  );
  const manifest = JSON.parse(await readFile(join(root, 'release/frontend-manifest.json'), 'utf8'));
  const cwd = await mkdtemp(join(tmpdir(), 'branch-release-check-'));
  const port = await freePort(),
    origin = 'http://127.0.0.1:' + port;
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
    FRONTEND_DIR: join(root, 'release/public'),
    APP_DATA_DIR: join(cwd, 'data'),
    APP_BACKUP_DIR: join(cwd, 'backups'),
    APP_MODEL_DIR: join(cwd, 'missing-model')
  };
  let logs = '',
    overflow = false;
  type OwnedProcess = { child: ChildProcess; closed: Promise<number> };
  const children: OwnedProcess[] = [];
  const cancellation = new AbortController();
  const abort = () => {
    cancellation.abort(new Error('Release verification interrupted or timed out.'));
    for (const owned of children) owned.child.kill('SIGKILL');
  };
  const overallDeadline = setTimeout(abort, 70000);
  process.once('SIGINT', abort);
  process.once('SIGTERM', abort);
  function launch(args: string[] = [], values = env): OwnedProcess {
    cancellation.signal.throwIfAborted();
    const child = spawn(binary, args, { cwd, env: values, stdio: ['ignore', 'pipe', 'pipe'] });
    for (const stream of [child.stdout, child.stderr])
      stream.on('data', (chunk) => {
        if (logs.length + chunk.length > 1024 * 1024) {
          overflow = true;
          child.kill('SIGKILL');
        } else logs += chunk.toString();
      });
    const closed = new Promise<number>((yes, no) => {
      child.once('error', () => no(new Error('Release executable could not start.')));
      child.once('close', (code) => yes(code ?? 1));
    });
    void closed.catch(() => {});
    const process = { child, closed };
    children.push(process);
    return process;
  }
  const boundedClose = (process: ReturnType<typeof launch>, milliseconds = 10000) =>
    Promise.race([
      process.closed,
      new Promise<never>((_, no) => {
        const timer = setTimeout(() => {
          process.child.kill('SIGKILL');
          no(new Error('Owned release subprocess timed out.'));
        }, milliseconds);
        timer.unref();
      })
    ]);
  const fetchLocal = (path: string, options: RequestInit = {}) =>
    fetch(origin + path, {
      ...options,
      signal: AbortSignal.any([cancellation.signal, AbortSignal.timeout(2000)])
    });
  async function ready(
    process: ReturnType<typeof launch>,
    discovery: 'unavailable' | 'ready' = 'unavailable'
  ) {
    const deadline = Date.now() + 10000;
    while (Date.now() < deadline) {
      cancellation.signal.throwIfAborted();
      check(
        process.child.exitCode === null && process.child.signalCode === null,
        'Release exited before readiness.'
      );
      try {
        const r = await fetchLocal('/health/ready');
        const v = await r.json();
        if (
          r.ok &&
          v.ready &&
          v.libraryReady &&
          v.storageReady &&
          !v.researchConfigured &&
          v.discovery === discovery
        )
          return;
      } catch {
        /* Wait only for this owned child. */
      }
      await new Promise((yes) => setTimeout(yes, 25));
    }
    throw new Error('Release did not become ready.');
  }
  async function stop(process: ReturnType<typeof launch>) {
    process.child.kill('SIGTERM');
    check((await boundedClose(process)) === 0, 'Unclean release shutdown.');
  }
  const checks: string[] = [];
  try {
    let process = launch();
    await ready(process);
    checks.push('no-node-runtime', 'unrelated-cwd', 'research-disabled');
    const html = await fetchLocal('/'),
      body = await html.text();
    check(
      html.status === 200 && html.headers.get('content-type')?.includes('text/html'),
      'Static shell unavailable.'
    );
    check(
      html.headers.get('x-content-type-options') === 'nosniff' &&
        html.headers.get('content-security-policy')?.includes('frame-ancestors'),
      'Security headers missing.'
    );
    check(
      (await fetchLocal('/access?key=' + encodeURIComponent(question), { redirect: 'manual' }))
        .status === 401,
      'Invalid key accepted.'
    );
    const access = await fetchLocal('/access?key=' + encodeURIComponent(key), {
      redirect: 'manual'
    });
    const setCookie = access.headers.get('set-cookie');
    check(
      access.status === 303 &&
        access.headers.get('location') === '/' &&
        setCookie?.includes('HttpOnly') &&
        setCookie.includes('SameSite=Strict'),
      'Private cookie incorrect.'
    );
    check(setCookie !== null, 'Missing private cookie.');
    const cookie = setCookie.split(';')[0];
    let info: any;
    const headers = () => ({
      cookie,
      origin,
      'content-type': 'application/json',
      'x-branch-protocol': '2',
      ...(info ? { 'x-branch-library': info.libraryId } : {})
    });
    async function api(path: string, options: RequestInit = {}, status = 200) {
      const response = await fetchLocal('/api' + path, {
        ...options,
        headers: { ...headers(), ...options.headers }
      });
      check(response.status === status, `Unexpected HTTP ${response.status} for ${path}.`);
      check(
        response.headers.get('cache-control')?.includes('no-store'),
        'Private response may be cached.'
      );
      return response.json();
    }
    const command = (payload: unknown, expectedRevision: number | null = null) => ({
      operationId: randomUUID(),
      libraryEpoch: info.libraryEpoch,
      expectedRevision,
      payload
    });
    check(
      (await fetchLocal('/api/library', { headers: { 'x-branch-protocol': '2' } })).status === 401,
      'Library is public.'
    );
    info = await api('/library');
    check(info.discovery === 'unavailable', 'Missing model not degraded.');
    const create = command({ question, locale: 'en' });
    const created = await api('/explorations', { method: 'POST', body: JSON.stringify(create) });
    check(created.state === 'completed', 'Create not durable.');
    const id = created.result.explorationId,
      appearance = created.result.appearanceId;
    const saved = await api('/explorations/' + id);
    check(saved.appearances[0].question === question, 'Question not stored.');
    check(
      (
        await fetchLocal('/api/explorations', {
          method: 'POST',
          headers: { ...headers(), origin: 'https://invalid.example' },
          body: JSON.stringify(create)
        })
      ).status === 403,
      'Foreign origin accepted.'
    );
    check(
      (await fetchLocal('/api/research', { method: 'POST', headers: headers(), body: '{}' }))
        .status === 426,
      'Legacy protocol admitted research.'
    );
    const research = command(
      { appearanceId: appearance, refresh: false, clarification: null },
      saved.exploration.revision
    );
    await api(
      '/appearances/' + appearance + '/research',
      { method: 'POST', body: JSON.stringify(research) },
      503
    );
    const failed = await api('/operations/' + research.operationId);
    check(
      failed.state === 'failed' && failed.error.code === 'SETUP_REQUIRED',
      'Provider absence receipt incorrect.'
    );
    checks.push('durable-library', 'private-access');
    const original = JSON.parse(
      await readFile(join(root, 'tests/fixtures/library/migration.json'), 'utf8')
    ) as Exploration;
    const encoded = JSON.stringify(original),
      digest = createHash('sha256')
        .update(canonicalLegacyJson(legacyIdentity(original)))
        .digest('hex');
    const begin = await api('/imports', {
      method: 'POST',
      body: JSON.stringify(
        command({
          legacyExplorationId: original.id,
          contentDigest: digest,
          expectedBytes: Buffer.byteLength(encoded),
          expectedNodes: Object.keys(original.nodes).length,
          explicitRestore: false
        })
      )
    });
    await api('/imports/' + begin.id + '/batches', {
      method: 'POST',
      body: JSON.stringify(command({ importId: begin.id, batchIndex: 0, records: [encoded] }))
    });
    const imported = await api('/imports/' + begin.id + '/completion', {
      method: 'POST',
      body: JSON.stringify(command({ importId: begin.id }))
    });
    check(imported.state === 'completed' && imported.mapping.root, 'Legacy import incomplete.');
    const legacy = await api('/explorations/' + imported.explorationId);
    check(
      legacy.answers.length === 3 &&
        legacy.answers.some(
          (a: any) => a.content.sources[0].url === original.nodes.root.content!.sources[0].url
        ),
      'Legacy answers/citations changed.'
    );
    checks.push('legacy-upgrade');
    const lockLogStart = logs.length;
    const second = launch([], { ...env, PORT: String(await freePort()) });
    check((await boundedClose(second)) !== 0, 'Second writer opened data.');
    check(
      logs.slice(lockLogStart).includes('STORAGE_LOCKED'),
      'Second writer did not fail on the data lock.'
    );
    checks.push('exclusive-writer');
    process.child.kill('SIGKILL');
    await boundedClose(process);
    process = launch();
    await ready(process);
    check(
      JSON.stringify(await api('/explorations/' + imported.explorationId)) ===
        JSON.stringify(legacy),
      'Acknowledged snapshot changed after restart.'
    );
    check(
      (await api('/operations/' + created.id)).state === 'completed' &&
        (await api('/operations/' + research.operationId)).state === 'failed',
      'Operation receipts lost.'
    );
    checks.push('restart-persistence');
    await stop(process);
    const backup = join(cwd, 'manual-backup');
    check(
      (await boundedClose(launch(['storage', 'backup', '--output', backup]), 20000)) === 0,
      'Native backup failed.'
    );
    const restored = join(cwd, 'restored');
    check(
      (await boundedClose(
        launch(['storage', 'restore', '--input', backup, '--data-dir', restored]),
        20000
      )) === 0,
      'Native restore failed.'
    );
    const restoredEnv = { ...env, APP_DATA_DIR: restored };
    process = launch([], restoredEnv);
    await ready(process);
    const next = await api('/library');
    check(
      next.libraryId === info.libraryId && next.libraryEpoch > info.libraryEpoch,
      'Restore identity/epoch incorrect.'
    );
    await api('/explorations', { method: 'POST', body: JSON.stringify(create) }, 409);
    info = next;
    const restoredSnapshot = await api('/explorations/' + imported.explorationId);
    check(
      JSON.stringify(restoredSnapshot.answers) === JSON.stringify(legacy.answers),
      'Restore changed immutable answers.'
    );
    checks.push('backup-restore', 'epoch-fencing');
    await stop(process);
    // The packaged pinned model is loaded by the native executable, without tools on PATH.
    process = launch([], { ...restoredEnv, APP_MODEL_DIR: join(root, 'release/models') });
    await ready(process, 'ready');
    check(
      (await api('/explorations/' + imported.explorationId)).answers.length === 3,
      'Model load lost library.'
    );
    checks.push('native-model');
    await stop(process);
    const corrupt = join(cwd, 'corrupt-model');
    await mkdir(corrupt);
    await writeFile(join(corrupt, 'model-manifest.json'), '{}');
    process = launch([], { ...restoredEnv, APP_MODEL_DIR: corrupt });
    await ready(process);
    check(
      (await api('/explorations/' + imported.explorationId)).answers.length === 3,
      'Corrupt model blocked library.'
    );
    checks.push('model-degradation');
    for (const path of [
      '/models/model_optimized.onnx',
      '/native-runtime-manifest.json',
      '/data/knowledge.db',
      '/backups/manifest.json',
      '/.branch-storage-owner'
    ])
      check((await fetchLocal(path)).status === 404, 'Private storage/model asset served.');
    checks.push('private-storage-assets');
    check((await fetchLocal('/metrics')).status === 401, 'Metrics public.');
    const metricsResponse = await fetchLocal('/metrics', { headers: { cookie } }),
      metrics = await metricsResponse.text();
    check(
      metricsResponse.ok &&
        metrics.includes('branch_active_jobs 0') &&
        !/^branch_research_stages_total\{/m.test(metrics),
      'Unexpected paid provider stages.'
    );
    const paidProviderRequests = [
      ...metrics.matchAll(/^branch_research_stages_total\{[^\n]+\} (\d+)/gm)
    ].reduce((n, m) => n + Number(m[1]), 0);
    check(
      paidProviderRequests === 0 && !logs.includes('research_stage_completed'),
      'Startup/recovery ran paid work.'
    );
    checks.push('private-metrics', 'zero-paid-starts');
    for (const file of manifest.files) {
      check(
        typeof file.path === 'string' &&
          !file.path.split('/').some((part: string) => part.startsWith('.')) &&
          !/\.(map|ts|svelte|pem|key)$/.test(file.path),
        'Unsafe release asset.'
      );
      const path = resolve(root, 'release/public', file.path);
      check(path.startsWith(resolve(root, 'release/public') + '/'), 'Escaped asset path.');
      const bytes = await readFile(path);
      check(
        createHash('sha256').update(bytes).digest('hex') === file.sha256,
        'Release asset digest mismatch.'
      );
      check(
        !bytes.includes(Buffer.from(key)) && !bytes.includes(Buffer.from(question)),
        'Private values leaked into assets.'
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
    await stop(process);
    checks.push('sigterm-exit');
    check(
      !overflow && !logs.includes(key) && !logs.includes(question),
      'Private value leaked into logs.'
    );
    check(logs.includes('shutdown_completed'), 'Shutdown not observable.');
    checks.push('secret-redaction');
    console.log(JSON.stringify({ ok: true, checks, platform }));
  } finally {
    clearTimeout(overallDeadline);
    globalThis.process.removeListener('SIGINT', abort);
    globalThis.process.removeListener('SIGTERM', abort);
    for (const process of children) {
      if (process.child.exitCode === null && process.child.signalCode === null)
        process.child.kill('SIGKILL');
    }
    await Promise.allSettled(children.map((process) => process.closed));
    await rm(cwd, { recursive: true, force: true });
  }
}
try {
  await verify();
} catch (error) {
  console.error(
    'Release verification failed: ' +
      (error instanceof Error ? error.message : 'check artifact and readiness.')
  );
  process.exitCode = 1;
}
