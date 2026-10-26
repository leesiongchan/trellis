// Explicit setup only. Application startup never executes this script.
import { mkdir, lstat, readFile, writeFile, rename, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, randomUUID } from 'node:crypto';
import type { ModelAssetsManifest } from './package-release.ts';
const PINNED_MODEL = 'Qdrant/bge-small-en-v1.5-onnx-Q';
const PINNED_REVISION = 'aa8f8b060edb00e03bfdd08813a2949946c8ba55';
export async function fetchEmbeddingModel(
  destination: string,
  fetcher: typeof fetch = fetch
): Promise<void> {
  const root = resolve(destination);
  const manifest: ModelAssetsManifest = JSON.parse(
    await readFile(
      new URL('../tests/fixtures/library/model-manifest.json', import.meta.url),
      'utf8'
    )
  );
  if (
    root === resolve('/') ||
    manifest.modelId !== PINNED_MODEL ||
    manifest.revision !== PINNED_REVISION
  )
    throw new Error('Invalid pinned model setup.');
  await mkdir(root, { recursive: true, mode: 0o700 });
  const stat = await lstat(root);
  if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Model directory is unsafe.');
  for (const [name, expected] of Object.entries(manifest.artifactHashes)) {
    if (
      !/^(model_optimized\.onnx|tokenizer\.json|config\.json|tokenizer_config\.json|special_tokens_map\.json)$/.test(
        name
      ) ||
      !/^[a-f0-9]{64}$/.test(expected)
    )
      throw new Error('Invalid pinned model artifact.');
    try {
      const metadata = await lstat(join(root, name));
      if (
        metadata.isFile() &&
        !metadata.isSymbolicLink() &&
        metadata.size <= 128 * 1024 * 1024 &&
        createHash('sha256')
          .update(await readFile(join(root, name)))
          .digest('hex') === expected
      )
        continue;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }
    const response = await fetcher(
      `https://huggingface.co/${PINNED_MODEL}/resolve/${PINNED_REVISION}/${name}?download=true`,
      { signal: AbortSignal.timeout(120_000) }
    );
    if (!response.ok || !response.body) throw new Error('Pinned model download failed.');
    const chunks: Uint8Array[] = [];
    let length = 0;
    const hash = createHash('sha256');
    for await (const chunk of response.body) {
      length += chunk.byteLength;
      if (length > 128 * 1024 * 1024) throw new Error('Model download is too large.');
      hash.update(chunk);
      chunks.push(chunk);
    }
    if (hash.digest('hex') !== expected)
      throw new Error('Pinned model download checksum mismatch.');
    const temporary = join(root, '.download-' + randomUUID());
    try {
      await writeFile(temporary, Buffer.concat(chunks), { flag: 'wx', mode: 0o644 });
      await rename(temporary, join(root, name));
    } finally {
      await rm(temporary, { force: true });
    }
  }
  const manifestTemporary = join(root, '.manifest-' + randomUUID());
  await writeFile(manifestTemporary, JSON.stringify(manifest, null, 2) + '\n', {
    flag: 'wx',
    mode: 0o644
  });
  await rename(manifestTemporary, join(root, 'model-manifest.json'));
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await fetchEmbeddingModel(process.argv[2] ?? '.artifacts/embedding-model');
  console.log('Verified pinned local model assets.');
}
