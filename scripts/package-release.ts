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
export async function packageRelease(input: {
  frontendDir: string;
  executable: string;
  outputDir: string;
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
  await packageRelease({
    frontendDir: process.argv[2] ?? 'build',
    executable: process.argv[3] ?? 'backend/target/release/branch-backend',
    outputDir: process.argv[4] ?? 'release'
  });
  console.log('Packaged Rust executable and verified frontend assets in release/.');
}
