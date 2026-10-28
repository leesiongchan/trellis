import { test } from 'node:test';
import assert from 'node:assert/strict';
import { startProcess } from '../../scripts/lib/process.ts';
void test('single native process mode waits for graceful termination without group signalling', async () => {
  const child = startProcess(process.execPath, ['-e', 'process.on("SIGTERM",()=>setTimeout(()=>process.exit(0),10));setInterval(()=>{},1000)'], { env: {}, cwd: process.cwd(), singleProcess: true });
  await new Promise((resolve) => setTimeout(resolve, 100));
  await child.stop();
  assert.equal(await child.exited, 0);
  await child.stop();
});
