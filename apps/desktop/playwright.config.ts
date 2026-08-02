import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  expect: {
    timeout: 5_000,
  },
  fullyParallel: false,
  reporter: 'line',
  testDir: './e2e',
  testMatch: '**/*.e2e.ts',
  use: {
    baseURL: 'http://localhost:1420',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'bun run dev',
    reuseExistingServer: true,
    timeout: 120_000,
    url: 'http://localhost:1420',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})
