import { access } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
for (const path of [
  'release/branch-backend',
  'release/public/index.html',
  'release/frontend-manifest.json'
])
  await access(resolve(root, path));
console.log(
  'Release artifact files exist. Runtime verification follows in the release-check task.'
);
