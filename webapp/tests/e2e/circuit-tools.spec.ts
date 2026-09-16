import { expect, test } from '@playwright/test';

test('calculates the loaded divider, downloads source and restores the prior workspace', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('kessetsu.workspace.draft.v1', JSON.stringify({
    schema_version: 'kessetsu.web-draft.v1', saved_at: new Date().toISOString(),
    name: 'My prior circuit', source: 'net GND\nnet OUT\nsource V1 5V\nresistor R1 1000\nconnect V1.plus, R1.p1 to OUT\nconnect V1.minus, R1.p2 to GND\nsimulate op\n', dirty: true,
  })));
  await page.getByRole('navigation', { name: 'Main navigation' }).getByRole('link', { name: 'Tools', exact: true }).click();
  await page.getByRole('link', { name: /Loaded voltage divider/ }).click();
  await page.getByLabel('Component values', { exact: true }).selectOption('exact');
  await page.getByRole('button', { name: 'Calculate circuit' }).click();
  const output = page.getByRole('region', { name: 'Calculation results' });
  await expect(output.locator('dl > div').filter({ has: page.locator('dt', { hasText: /^Loaded output$/ }) })).toContainText('3 V');
  await expect(output).toContainText('15 kΩ');
  await page.screenshot({ path: 'test-results/circuit-tool-desktop.png', fullPage: true });
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download .kess' }).click();
  expect((await download).suggestedFilename()).toBe('voltage-divider.kess');
  await page.getByRole('button', { name: 'Open in editor' }).click();
  await expect(page.getByTestId('compile-status')).toContainText('Checked');
  await expect(page.locator('.document-title')).toContainText('Loaded voltage divider');
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded', { timeout: 20_000 });
  await expect(page.locator('.op-grid > div').filter({ hasText: 'V(out)' })).toContainText('3.000 V');
  await expect(page.locator('.run-status')).toContainText('Simulation complete');
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  const svgDownload = page.waitForEvent('download');
  await page.locator('[data-export-format="svg"]').click();
  expect((await svgDownload).suggestedFilename()).toBe('loaded-voltage-divider.svg');
  await page.getByRole('button', { name: 'Close export' }).click();
  await page.getByRole('button', { name: 'File', exact: true }).click();
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('menuitem', { name: 'Restore previous circuit' }).click();
  await expect(page.locator('.document-title')).toContainText('My prior circuit');
  await expect(page.getByTestId('compile-status')).toContainText('Checked');
});

test('RC rounding changes achieved cutoff, rejects wrong units and stays usable on mobile', async ({ page, browser, baseURL }) => {
  await page.goto('/tools/rc-lowpass/');
  await page.getByLabel('Component values', { exact: true }).selectOption('e12');
  await page.getByRole('button', { name: 'Calculate circuit' }).focus();
  await page.keyboard.press('Enter');
  const output = page.getByRole('region', { name: 'Calculation results' });
  await expect(output).toContainText('150 nF');
  await expect(output).toContainText('1.06103 kHz');
  await page.getByLabel('Target cutoff frequency', { exact: true }).fill('1V');
  await expect(page.getByRole('button', { name: 'Open in editor' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Calculate circuit' }).click();
  await expect(page.getByRole('alert')).toContainText('unit mismatch');
  await page.getByLabel('Target cutoff frequency', { exact: true }).fill('2kHz');
  await page.getByRole('button', { name: 'Calculate circuit' }).click();
  await expect(page.getByRole('alert')).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'test-results/circuit-tool-mobile.png', fullPage: true });
  const noJs = await browser.newContext({ javaScriptEnabled: false, baseURL });
  try {
    const staticPage = await noJs.newPage();
    await staticPage.goto('/tools/rc-lowpass/');
    await expect(staticPage.getByRole('heading', { name: 'RC low-pass filter', exact: true })).toBeVisible();
    await expect(staticPage.getByText('C = 1 / (2π × R × cutoff frequency)', { exact: true })).toBeVisible();
  } finally { await noJs.close(); }
  await page.getByRole('button', { name: 'Open in editor' }).click();
  await expect(page.getByTestId('compile-status')).toContainText('Checked');
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded', { timeout: 20_000 });
  await expect(page.getByLabel('Signal', { exact: true })).toHaveValue('out');
  await expect(page.locator('.result-plot')).toHaveCount(2);
});
