import { mkdir, readdir, readFile, writeFile, chmod, rename, lstat } from 'node:fs/promises';
import { resolve, join, dirname, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, randomUUID } from 'node:crypto';

const assetExtensions = new Set([
  '.html',
  '.js',
  '.css',
  '.json',
  '.svg',
  '.ico',
  '.png',
  '.jpg',
  '.jpeg',
  '.webp',
  '.avif',
  '.gif',
  '.woff',
  '.woff2',
  '.ttf',
  '.eot',
  '.txt'
]);
async function directory(path: string): Promise<void> {
  try {
    const stat = await lstat(path);
    if (!stat.isDirectory() || stat.isSymbolicLink())
      throw new Error('Release directory is unsafe.');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    await mkdir(path, { recursive: true });
  }
}
async function collect(root: string, path = ''): Promise<{ path: string; bytes: Buffer }[]> {
  const files: { path: string; bytes: Buffer }[] = [];
  for (const entry of await readdir(join(root, path), { withFileTypes: true })) {
    if (entry.isSymbolicLink()) throw new Error('Frontend build contains a symlink.');
    if (entry.name.startsWith('.')) continue;
    const relative = path ? path + '/' + entry.name : entry.name;
    if (entry.isDirectory()) files.push(...(await collect(root, relative)));
    else if (entry.isFile() && assetExtensions.has(extname(entry.name))) {
      const stat = await lstat(join(root, relative));
      if (stat.size > 32 * 1024 * 1024) throw new Error('Frontend asset is too large.');
      files.push({ path: relative, bytes: await readFile(join(root, relative)) });
    }
  }
  return files;
}
export interface ModelAssetsManifest {
  modelId: string;
  revision: string;
  dimensions: number;
  maxTokens: number;
  artifactHashes: Record<string, string>;
  preprocessingVersion: string;
}
export async function packageModelAssets(
  modelDir: string,
  outputDir: string,
  manifest: ModelAssetsManifest
): Promise<void> {
  const allowed = new Set([
    'model_optimized.onnx',
    'tokenizer.json',
    'config.json',
    'tokenizer_config.json',
    'special_tokens_map.json'
  ]);
  const files = [];
  for (const [name, hash] of Object.entries(manifest.artifactHashes)) {
    if (!allowed.has(name) || !/^[0-9a-f]{64}$/.test(hash))
      throw new Error('Invalid model artifact manifest.');
    const path = join(modelDir, name),
      stat = await lstat(path);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 128 * 1024 * 1024)
      throw new Error('Model artifact is unsafe.');
    const bytes = await readFile(path);
    if (createHash('sha256').update(bytes).digest('hex') !== hash)
      throw new Error('Model artifact checksum mismatch.');
    files.push({ name, bytes });
  }
  const destination = join(outputDir, 'models');
  await directory(destination);
  for (const file of files) {
    const temp = join(destination, '.model-' + randomUUID());
    await writeFile(temp, file.bytes, { flag: 'wx', mode: 0o644 });
    await rename(temp, join(destination, file.name));
  }
  const temp = join(destination, '.manifest-' + randomUUID());
  await writeFile(temp, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  await rename(temp, join(destination, 'model-manifest.json'));
}
export async function packageRelease(input: {
  frontendDir: string;
  executable: string;
  outputDir: string;
  modelDir?: string;
  modelManifest?: ModelAssetsManifest;
}): Promise<void> {
  const frontendDir = resolve(input.frontendDir),
    outputDir = resolve(input.outputDir);
  if (frontendDir === outputDir || outputDir === dirname(outputDir))
    throw new Error('Release output must be a dedicated directory.');
  const files = (await collect(frontendDir)).sort((a, b) => a.path.localeCompare(b.path));
  const index = files.find((f) => f.path === 'index.html');
  if (
    !index ||
    files.length > 4096 ||
    files.reduce((n, f) => n + f.bytes.length, 0) > 128 * 1024 * 1024
  )
    throw new Error('A bounded static frontend build is required.');
  const inlineScriptHashes: string[] = [];
  for (const match of index.bytes
    .toString('utf8')
    .matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script\s*>/gi)) {
    if (!/(?:^|\s)src\s*=/i.test(match[1]))
      inlineScriptHashes.push('sha256-' + createHash('sha256').update(match[2]).digest('base64'));
  }
  if (inlineScriptHashes.length > 32) throw new Error('Frontend has too many inline scripts.');
  const executable = await readFile(input.executable);
  await directory(outputDir);
  await directory(join(outputDir, 'public'));
  if (input.modelDir) {
    const modelManifest =
      input.modelManifest ??
      JSON.parse(
        await readFile(
          new URL('../tests/fixtures/library/model-manifest.json', import.meta.url),
          'utf8'
        )
      );
    await packageModelAssets(resolve(input.modelDir), outputDir, modelManifest);
  }
  const runtime = await readFile(
    new URL('../tests/fixtures/library/native-runtime-manifest.json', import.meta.url)
  );
  const runtimeTemp = join(outputDir, '.runtime-' + randomUUID());
  await writeFile(runtimeTemp, runtime, { flag: 'wx' });
  await rename(runtimeTemp, join(outputDir, 'native-runtime-manifest.json'));

  for (const file of files) {
    const parts = file.path.split('/');
    let parent = join(outputDir, 'public');
    for (const part of parts.slice(0, -1)) {
      parent = join(parent, part);
      await directory(parent);
    }
    const destination = join(outputDir, 'public', file.path);
    const temporary = join(parent, '.asset-' + randomUUID());
    await writeFile(temporary, file.bytes, { flag: 'wx', mode: 0o644 });
    await rename(temporary, destination);
  }
  const temporaryBinary = join(outputDir, '.binary-' + randomUUID());
  await writeFile(temporaryBinary, executable, { flag: 'wx', mode: 0o755 });
  await chmod(temporaryBinary, 0o755);
  await rename(temporaryBinary, join(outputDir, 'branch-backend'));
  const manifest = {
    version: 1,
    files: files.map((file) => ({
      path: file.path,
      sha256: createHash('sha256').update(file.bytes).digest('hex'),
      immutable: file.path.startsWith('_app/immutable/')
    })),
    inlineScriptHashes
  };
  const temporaryManifest = join(outputDir, '.manifest-' + randomUUID());
  await writeFile(temporaryManifest, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  await rename(temporaryManifest, join(outputDir, 'frontend-manifest.json'));
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const defaultModelDir = resolve('.artifacts/embedding-model');
  const modelDir =
    process.argv[5] ??
    (await lstat(defaultModelDir)
      .then(() => defaultModelDir)
      .catch((error: NodeJS.ErrnoException) => {
        if (error.code === 'ENOENT') return undefined;
        throw error;
      }));
  await packageRelease({
    frontendDir: process.argv[2] ?? 'build',
    executable: process.argv[3] ?? 'backend/target/release/branch-backend',
    outputDir: process.argv[4] ?? 'release',
    modelDir
  });
  console.log('Packaged Rust executable and verified frontend assets in release/.');
}
