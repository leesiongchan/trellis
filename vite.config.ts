import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    ignorePatterns: ['api/vendor/**', 'docs/superpowers/**', 'app/styled-system/**'],
    jsPlugins: [
      { name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' },
      { name: '@pandacss', specifier: './app/panda.oxlint.mjs' }
    ],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error' },
    options: { typeAware: true, typeCheck: true },
    overrides: [
      {
        files: ['app/src/**/*.ts'],
        rules: {
          '@pandacss/no-invalid-token-paths': 'error',
          '@pandacss/no-invalid-nesting': 'error',
          '@pandacss/file-not-included': 'error',
          '@pandacss/no-deprecated': 'error',
          '@pandacss/no-debug': 'error',
          '@pandacss/extraction-diagnostics': 'error',
          '@pandacss/prefer-token': ['error', { categories: ['colors', 'spacing', 'radii'] }]
        }
      },
      {
        // Fetch fixtures inspect the string/URL arguments produced by our client.
        files: ['app/tests/**/*.ts', 'tests/build/**/*.ts'],
        rules: { 'typescript/no-base-to-string': 'off' }
      }
    ]
  },
  fmt: {
    singleQuote: true,
    trailingComma: 'none',
    printWidth: 100,
    tabWidth: 2,
    sortPackageJson: false,
    svelte: {},
    ignorePatterns: [
      'node_modules',
      '.svelte-kit',
      '.superpowers',
      'build',
      'app/styled-system',
      'pnpm-lock.yaml',
      'test-results',
      'playwright-report',
      '.cargo/**',
      'api/Cargo.*',
      'api/src/**',
      'api/tests/**',
      'api/vendor/**',
      'docs/superpowers/**',
      'tests/fixtures/**'
    ]
  }
});
