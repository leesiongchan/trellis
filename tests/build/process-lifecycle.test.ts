import { test } from 'node:test';
import assert from 'node:assert/strict';
import { startProcess, testEnvironment } from '../../scripts/lib/process.ts';
void test('fixture environment preserves explicit tool trust without inheriting provider credentials', () => {
  const names = ['MISE_TRUSTED_CONFIG_PATHS', 'OPENAI_API_KEY', 'TAVILY_API_KEY'] as const;
  const saved = names.map((name) => process.env[name]);
  try {
    process.env.MISE_TRUSTED_CONFIG_PATHS = '/fixture/checkout';
    process.env.OPENAI_API_KEY = 'parent-openai-key';
    process.env.TAVILY_API_KEY = 'parent-tavily-key';
    const env = testEnvironment();
    assert.equal(env.MISE_TRUSTED_CONFIG_PATHS, '/fixture/checkout');
    assert.equal(env.OPENAI_API_KEY, '');
    assert.equal(env.TAVILY_API_KEY, '');
  } finally {
    names.forEach((name, index) => {
      if (saved[index] === undefined) delete process.env[name];
      else process.env[name] = saved[index];
    });
  }
});
void test('single native process mode waits for graceful termination without group signalling', async () => {
  const child = startProcess(process.execPath, ['-e', 'process.on("SIGTERM",()=>setTimeout(()=>process.exit(0),10));setInterval(()=>{},1000)'], { env: {}, cwd: process.cwd(), singleProcess: true });
  await new Promise((resolve) => setTimeout(resolve, 100));
  await child.stop();
  assert.equal(await child.exited, 0);
  await child.stop();
});
