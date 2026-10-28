import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const script=fileURLToPath(new URL('../../scripts/verify-release.ts',import.meta.url));
void test('release verifier exercises runtime with no Node on its child PATH',()=>{
  const result=spawnSync(process.execPath,[script],{encoding:'utf8',timeout:90000});assert.equal(result.status,0,result.stderr);
  const summary=JSON.parse(result.stdout.trim().split('\n').at(-1)!);assert.equal(summary.ok,true);for(const check of ['no-node-runtime','research-disabled','private-metrics','asset-integrity','secret-redaction','sigterm-exit','durable-library','legacy-upgrade','exclusive-writer','restart-persistence','backup-restore','model-degradation','native-model','private-storage-assets','epoch-fencing','zero-paid-starts'])assert.ok(summary.checks.includes(check),check);
});
