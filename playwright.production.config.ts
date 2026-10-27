import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  use: { ...devices['Desktop Chrome'], trace: 'retain-on-failure' },
  webServer: {
    command: 'node scripts/test-server.ts',
    url: 'https://127.0.0.1:5185/health/ready',
    ignoreHTTPSErrors: true,
    reuseExistingServer: false,
    timeout: 120000,
    gracefulShutdown: { signal: 'SIGTERM', timeout: 135000 }
  },
  projects: [
    {
      name: 'production',
      testIgnore: /(rust-backend|library-graph)\.spec\.ts/,
      use: { baseURL: 'http://127.0.0.1:5184' }
    },
    {
      name: 'graph-fixture',
      testMatch: /library-graph\.spec\.ts/,
      use: { baseURL: 'http://127.0.0.1:5187' }
    },
    {
      name: 'fixture-http',
      testMatch: /rust-backend\.spec\.ts/,
      use: { baseURL: 'http://127.0.0.1:5184' }
    },
    {
      name: 'fixture-https',
      testMatch: /(rust-backend|upgrade)\.spec\.ts/,
      use: { baseURL: 'https://127.0.0.1:5185', ignoreHTTPSErrors: true }
    }
  ],
  reporter: 'list'
});
