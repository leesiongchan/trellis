import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { packageRelease } from '../../scripts/package-release.ts';

void test('release contains only static assets with exact script hashes',async()=>{
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
    assert.deepEqual(JSON.parse(await readFile(join(outputDir,'release-manifest.json'),'utf8')), {protocolVersion:2,storageSchemaVersion:1,engineVersion:'3.3.4',executableSha256:createHash('sha256').update('fixture executable').digest('hex')});
    assert.equal(manifest.files.length,2);assert.equal(manifest.files.find((f:{path:string})=>f.path.endsWith('chunk.js')).immutable,true);
    await assert.rejects(readFile(join(outputDir,'public','.env')));await assert.rejects(readFile(join(outputDir,'public','chunk.js.map')));
    assert.equal(await readFile(join(outputDir,'branch-backend'),'utf8'),'fixture executable');
    await symlink(executable,join(frontendDir,'escaped.js'));await assert.rejects(packageRelease({frontendDir,outputDir,executable}));
  } finally {await rm(root,{recursive:true,force:true});}
});

void test('models are checksum verified and packaged outside public assets',async()=>{
 const root=await mkdtemp(join(tmpdir(),'branch-model-package-'));
 try{
  const frontendDir=join(root,'build'),outputDir=join(root,'release'),executable=join(root,'binary'),modelDir=join(root,'models');
  await mkdir(frontendDir);await mkdir(modelDir);await writeFile(join(frontendDir,'index.html'),'<html>Library</html>');await writeFile(executable,'native fixture');
  const manifest={modelId:'test-public-model',revision:'pinned-revision',dimensions:384,maxTokens:512,preprocessingVersion:'fixture-v1',artifactHashes:{'model_optimized.onnx':createHash('sha256').update('fixture model').digest('hex'),'tokenizer.json':createHash('sha256').update('{}').digest('hex')}};
  await writeFile(join(modelDir,'model_optimized.onnx'),'fixture model');await writeFile(join(modelDir,'tokenizer.json'),'{}');
  await packageRelease({frontendDir,outputDir,executable,modelDir,modelManifest:manifest});
  assert.equal(await readFile(join(outputDir,'models/model_optimized.onnx'),'utf8'),'fixture model');
  assert.deepEqual(JSON.parse(await readFile(join(outputDir,'models/model-manifest.json'),'utf8')),manifest);
  await assert.rejects(readFile(join(outputDir,'public/model_optimized.onnx')));
  const runtime=JSON.parse(await readFile(join(outputDir,'native-runtime-manifest.json'),'utf8'));assert.equal(runtime.linkage,'static');assert.equal(runtime.onnxRuntimeVersion,'1.31.0');
  await writeFile(join(modelDir,'model_optimized.onnx'),'tampered');await assert.rejects(packageRelease({frontendDir,outputDir,executable,modelDir,modelManifest:manifest}));
 }finally{await rm(root,{recursive:true,force:true});}
});

void test('explicit model setup requests pinned files and rejects tampered downloads',async()=>{
 const {fetchEmbeddingModel}=await import('../../api/scripts/fetch-embedding-model.ts');
 const root=await mkdtemp(join(tmpdir(),'branch-model-setup-'));
 try{
  const calls:string[]=[];
  const fetcher=(async(input:RequestInfo|URL)=>{calls.push(String(input));return new Response('tampered model');})as typeof fetch;
  await assert.rejects(fetchEmbeddingModel(root,fetcher),/checksum mismatch/);
  assert.equal(calls.length,1);assert.match(calls[0],/^https:\/\/huggingface.co\/Qdrant\/bge-small-en-v1.5-onnx-Q\/resolve\/aa8f8b060edb00e03bfdd08813a2949946c8ba55\//);
  await assert.rejects(readFile(join(root,'model-manifest.json')));
 }finally{await rm(root,{recursive:true,force:true});}
});
