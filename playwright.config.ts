import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for AnchorKit UI end-to-end tests.
 *
 * Tests run against the Storybook build so that AnchorPlayground can be
 * exercised in isolation (tab navigation, form submission, theme switching
 * and the request panel).
 *
 * See https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: './ui/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: process.env.STORYBOOK_URL ?? 'http://127.0.0.1:6006',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: process.env.STORYBOOK_URL
    ? undefined
    : {
        command: 'npm run storybook -- --ci --quiet',
        url: 'http://127.0.0.1:6006',
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      },
});
