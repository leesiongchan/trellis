import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    ignorePatterns: ['api/vendor/**', 'docs/superpowers/**'],
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error' },
    options: { typeAware: true, typeCheck: true },
    overrides: [
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
