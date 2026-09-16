import { defineConfig, devices } from '@playwright/test';

// Release-only checks hit the deployed site, not the local Vite preview.
// Keep them separate from canonical CI so network availability is not a unit gate.
export default defineConfig({
  testDir: './tests',
  testMatch: [
    '**/e2e/browser-smoke.spec.ts',
    '**/e2e/install.spec.ts',
    '**/e2e/discovery.spec.ts',
    '**/e2e/benchmark-parity.spec.ts',
    '**/e2e/exports.spec.ts',
    '**/e2e/share.spec.ts',
    '**/e2e/simulation-parity.spec.ts',
    '**/production/live-boundary.spec.ts',
  ],
  fullyParallel: false,
  workers: 2,
  forbidOnly: true,
  retries: 0,
  reporter: 'list',
  outputDir: 'test-results/production',
  expect: { timeout: 15_000 },
  use: {
    baseURL: 'https://kessetsu.com',
    actionTimeout: 15_000,
    navigationTimeout: 45_000,
    trace: 'retain-on-failure',
  },
  projects: [{
    name: 'production-chromium',
    use: { ...devices['Desktop Chrome'] },
  }],
});
