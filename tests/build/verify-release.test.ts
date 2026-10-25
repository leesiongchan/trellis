import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,chmod,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
const script=fileURLToPath(new URL('../../scripts/verify-release.ts',import.meta.url));
test('release verifier exercises runtime with no Node on its child PATH',()=>{
  const result=spawnSync(process.execPath,[script],{encoding:'utf8',timeout:20000});assert.equal(result.status,0,result.stderr);
  const summary=JSON.parse(result.stdout.trim().split('\n').at(-1)!);assert.equal(summary.ok,true);for(const check of ['no-node-runtime','research-disabled','private-metrics','asset-integrity','secret-redaction','sigterm-exit'])assert.ok(summary.checks.includes(check),check);
});
test('release verifier rejects a missing executable',()=>{const result=spawnSync(process.execPath,[script,'--binary','/private/tmp/branch-deliberately-missing-binary'],{encoding:'utf8',timeout:5000});assert.notEqual(result.status,0);});
test('release verifier rejects an executable that requires Node',async()=>{
  const root=await mkdtemp(join(tmpdir(),'branch-node-required-'));try{const binary=join(root,'node-required');await writeFile(binary,'#!/usr/bin/env node\nprocess.exit(0)\n');await chmod(binary,0o755);const result=spawnSync(process.execPath,[script,'--binary',binary],{encoding:'utf8',timeout:10000});assert.notEqual(result.status,0);}finally{await rm(root,{recursive:true,force:true});}
});
