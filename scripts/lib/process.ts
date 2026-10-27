import { spawn, type ChildProcess } from 'node:child_process';
export type ManagedProcess = {
  child: ChildProcess;
  exited: Promise<number>;
  stop: () => Promise<void>;
};
export function startProcess(
  command: string,
  args: string[],
  options: { env: NodeJS.ProcessEnv; cwd: string; singleProcess?: boolean }
): ManagedProcess {
  const child = spawn(command, args, {
    env: options.env,
    cwd: options.cwd,
    stdio: 'inherit',
    detached: !options.singleProcess
  });
  const exited = new Promise<number>((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code) => resolve(code ?? 1));
  });
  let stopping: Promise<void> | undefined;
  function groupAlive(pid: number) {
    try {
      process.kill(-pid, 0);
      return true;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ESRCH') return false;
      throw error;
    }
  }
  function send(pid: number, signal: NodeJS.Signals) {
    try {
      process.kill(-pid, signal);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error;
    }
  }
  return {
    child,
    exited,
    stop: () =>
      (stopping ??= (async () => {
        const pid = child.pid;
        if (!pid || pid <= 0 || pid === process.pid) return;
        if (options.singleProcess) {
          if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
          let timer: ReturnType<typeof setTimeout> | undefined;
          const finished = await Promise.race([
            exited.then(
              () => true,
              () => true
            ),
            new Promise<false>((resolve) => {
              timer = setTimeout(() => resolve(false), 130_000);
              timer.unref();
            })
          ]);
          if (timer) clearTimeout(timer);
          if (!finished) child.kill('SIGKILL');
          await exited.catch(() => {});
          return;
        }
        send(pid, 'SIGTERM');
        const deadline = Date.now() + 130000;
        while (groupAlive(pid) && Date.now() < deadline)
          await new Promise((resolve) => setTimeout(resolve, 25));
        if (groupAlive(pid)) send(pid, 'SIGKILL');
        await exited.catch(() => {});
      })())
  };
}
export function testEnvironment(): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = {
    APP_ENV_FILE: '',
    HOST: '127.0.0.1',
    RUST_LOG: 'warn',
    OPENAI_API_KEY: '',
    TAVILY_API_KEY: '',
    APP_ACCESS_KEY: 'fixture-access-key-'.repeat(4)
  };
  for (const name of [
    'PATH',
    'HOME',
    'TMPDIR',
    'TMP',
    'TEMP',
    'CARGO_HOME',
    'RUSTUP_HOME',
    'RUSTUP_TOOLCHAIN',
    'SYSTEMROOT',
    'SSL_CERT_FILE',
    'NIX_SSL_CERT_FILE'
  ])
    if (process.env[name] !== undefined) env[name] = process.env[name];
  return env;
}
