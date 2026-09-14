import { expect, test } from '@playwright/test';

async function replaceSource(page: import('@playwright/test').Page, source: string) {
  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(source);
}

test('supports edit, inline diagnostic navigation, fix, simulation, assertion and schematic update', async ({ page }) => {
  test.setTimeout(120_000);
  page.on('pageerror', (error) => console.log(`[workspace:error] ${error.stack ?? error.message}`));
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByLabel('Simulation results')).toContainText('Run a simulation to inspect results.');
  await replaceSource(page, 'resistor R1 nope\n');
  const diagnostic = page.getByRole('button', { name: /KES-C001/ });
  await expect(diagnostic).toContainText('L1:');
  await diagnostic.click();
  await expect(page.locator('.monaco-editor').getByRole('textbox').first()).toBeFocused();

  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'RC Low-pass', exact: true }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-quality', 'pass');

  await page.getByRole('button', { name: 'Run' }).click();
  const results = page.getByTestId('simulation-summary');
  await expect(results).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(results.locator('.assertion-pass')).toHaveCount(5);
  await results.getByRole('tab', { name: 'ac' }).click();
  await expect(results.locator('.result-plot')).toHaveCount(2);
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    const path = process.env.KESSETSU_UPDATE_DOCS_ASSETS
      ? '../docs/assets/web-hub-workspace.png'
      : 'test-results/web-workspace.png';
    await page.screenshot({ path, fullPage: true });
  }
});

test('offers corresponding source and license from the interactive Web Hub', async ({ page }) => {
  await page.goto('/#editor');
  await page.getByRole('button', { name: 'Help', exact: true }).click();
  const sourceLink = page.getByRole('menuitem', { name: /source code and AGPL license/i });
  await expect(sourceLink).toHaveAttribute('href', 'https://github.com/Stapimaz/Kessetsu');
  await expect(sourceLink).toContainText('Source and license');
  await page.getByRole('button', { name: 'Help', exact: true }).click();
  await page.getByLabel('Circuit details').click();
  await page.getByText('Legal', { exact: true }).click();
  await expect(page.getByText('AGPL-3.0-only free software, provided without warranty.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Full license' })).toHaveAttribute('href', '/LICENSE.txt');
});

test('exposes keyboard controls and a usable mobile workspace', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#editor');
  await expect(page.getByLabel('Kessetsu source editor')).toBeVisible();
  await expect(page.getByLabel('Canonical schematic')).toBeVisible();
  await expect(page.getByLabel('Simulation results')).toBeVisible();
  await page.getByLabel('Canonical schematic').locator('.schematic-surface').focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('button', { name: 'Run' })).toBeEnabled();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.getByRole('button', { name: /theme/i })).toHaveCount(0);
});
