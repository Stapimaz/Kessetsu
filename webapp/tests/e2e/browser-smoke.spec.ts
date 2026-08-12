import { expect, test } from '@playwright/test';

test('initializes WASM and compiles the canonical example', async ({ page }) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.stack ?? error.message));

  await page.goto('/');

  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('.monaco-editor')).toBeVisible();
  await expect(page.getByText('Generated SPICE Netlist', { exact: true })).toBeVisible();
  await expect(page.getByText('Unsupported compile report schema')).toHaveCount(0);
  await expect(page.locator('svg polyline')).not.toHaveCount(0);
  expect(pageErrors).toEqual([]);
});
