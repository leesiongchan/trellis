import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { packageRelease } from '../../scripts/package-release.ts';

test('release contains only static assets with exact script hashes',async()=>{
  const root=await mkdtemp(join(tmpdir(),'branch-package-'));
  try {
    const frontendDir=join(root,'build'),outputDir=join(root,'release'),executable=join(root,'binary');
    await mkdir(join(frontendDir,'_app','immutable'),{recursive:true});
    const script='window.fixture=true;';const html='<html><script>'+script+'</script></html>';
    await writeFile(join(frontendDir,'index.html'),html);await writeFile(join(frontendDir,'_app','immutable','chunk.js'),'export{}');await writeFile(join(frontendDir,'.env'),'private-package-secret');await writeFile(join(frontendDir,'chunk.js.map'),'private-source-map');await writeFile(executable,'fixture executable');
    await packageRelease({frontendDir,outputDir,executable});
    assert.equal(await readFile(join(outputDir,'public','index.html'),'utf8'),html);
    const manifest=JSON.parse(await readFile(join(outputDir,'frontend-manifest.json'),'utf8'));
    assert.deepEqual(manifest.inlineScriptHashes,['sha256-'+createHash('sha256').update(script).digest('base64')]);
    assert.equal(manifest.files.length,2);assert.equal(manifest.files.find((f:{path:string})=>f.path.endsWith('chunk.js')).immutable,true);
    await assert.rejects(readFile(join(outputDir,'public','.env')));await assert.rejects(readFile(join(outputDir,'public','chunk.js.map')));
    assert.equal(await readFile(join(outputDir,'branch-backend'),'utf8'),'fixture executable');
    await symlink(executable,join(frontendDir,'escaped.js'));await assert.rejects(packageRelease({frontendDir,outputDir,executable}));
  } finally {await rm(root,{recursive:true,force:true});}
});
