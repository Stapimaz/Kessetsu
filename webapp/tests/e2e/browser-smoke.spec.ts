import { expect, test } from '@playwright/test';

test('opens the landing page, enters Web Hub, initializes WASM and compiles the canonical example', async ({ page }) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.stack ?? error.message));

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Circuit engineering you can execute.' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  expect(await page.locator('.landing-hero .eyebrow').evaluate((element) => (element as HTMLElement).innerText))
    .toBe('EXECUTABLE CIRCUIT ENGINEERING');
  await expect(page.getByText('First-release scope:')).toBeVisible();
  await expect(page.getByRole('contentinfo')).toContainText('Kessetsu 1.0.0');
  await expect(page.getByRole('navigation', { name: 'Footer navigation' }).getByRole('link')).toHaveCount(5);
  await expect(page.locator('.monaco-editor')).toHaveCount(0);
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await page.screenshot({ path: 'test-results/landing-desktop.png', fullPage: true });
  }
  await page.locator('.landing-hero').getByRole('link', { name: 'Open Web Hub' }).click();
  await expect(page).toHaveURL(/#editor$/);

  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('.monaco-editor')).toBeVisible();
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Circuit details…' }).click();
  await expect(page.getByText('Generated SPICE Netlist', { exact: true })).toBeVisible();
  await expect(page.getByText('Unsupported compile report schema')).toHaveCount(0);
  await expect(page.locator('svg polyline')).not.toHaveCount(0);
  expect(pageErrors).toEqual([]);
});

test('keeps the landing page readable and actionable on mobile', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Circuit engineering you can execute.' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.locator('.landing-header').getByRole('link', { name: /Open Web Hub/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Give your agent requirements, not blind trust.' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'The complete workflow, in your browser.' })).toBeVisible();
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await page.screenshot({ path: 'test-results/landing-mobile.png', fullPage: true });
  }
});
