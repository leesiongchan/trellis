import { fileURLToPath } from 'node:url';
import panda from '@pandacss/eslint-plugin';
import svelteParser from 'svelte-eslint-parser';
import tsParser from '@typescript-eslint/parser';

export default [
  await panda.configs.recommended({
    cwd: fileURLToPath(new URL('.', import.meta.url)),
    configPath: fileURLToPath(new URL('./panda.config.ts', import.meta.url)),
    files: ['src/**/*.svelte']
  }),
  {
    files: ['src/**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: { parser: tsParser }
    },
    rules: {
      '@pandacss/extraction-diagnostics': 'error',
      '@pandacss/no-deprecated': 'error',
      '@pandacss/no-debug': 'error',
      '@pandacss/prefer-token': ['error', { categories: ['colors', 'spacing', 'radii'] }]
    }
  }
];
