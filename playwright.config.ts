import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir:'./tests/e2e', fullyParallel:false, workers:1,
  use:{baseURL:'http://127.0.0.1:5173',...devices['Desktop Chrome'],trace:'retain-on-failure'},
  webServer:{command:'pnpm dev --port 5173',url:'http://127.0.0.1:5173',reuseExistingServer:true},
  reporter:'list'
});
