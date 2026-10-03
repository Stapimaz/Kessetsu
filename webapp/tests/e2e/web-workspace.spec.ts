import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

const productVersion = readFileSync(new URL('../../../VERSION', import.meta.url), 'utf8').trim();

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
  await expect(page.getByTestId('compile-status')).toHaveText('Source valid');
  await expect(page.getByLabel('Circuit simulation')).toContainText('Run the simulation to inspect plots and requirements.');
  await expect(page.getByLabel('First simulation steps')).toBeVisible();
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'New circuit', exact: true }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByLabel('Circuit simulation')).toContainText('This starter circuit includes an operating-point analysis. Press Run to simulate it.');
  await replaceSource(page, 'resistor R1 nope\n');
  await expect(page.getByTestId('compile-status')).toHaveText('1 error');
  const diagnostic = page.getByRole('button', { name: /KES-C001/ });
  await expect(diagnostic).toContainText('L1:');
  await expect(page.getByLabel('Circuit simulation')).toContainText('Fix source errors');
  await expect(page.getByRole('button', { name: 'Run simulation', exact: true })).toBeDisabled();
  await diagnostic.click();
  await expect(page.locator('.monaco-editor').getByRole('textbox').first()).toBeFocused();
  await page.getByRole('button', { name: 'Minimize source panel', exact: true }).click();
  await expect(page.getByLabel('Kessetsu source editor')).toBeHidden();
  await page.getByLabel('Circuit simulation').getByRole('button', { name: 'Show source errors' }).click();
  await expect(page.getByLabel('Kessetsu source editor')).toBeVisible();
  await expect(page.locator('.monaco-editor').getByRole('textbox').first()).toBeFocused();

  page.once('dialog', (dialog) => void dialog.accept());
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
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

  await page.getByRole('button', { name: 'Export', exact: true }).click();
  const exported = page.waitForEvent('download');
  await page.locator('[data-export-format="svg"]').click();
  const download = await exported;
  expect(download.suggestedFilename()).toBe('rc-low-pass.svg');
  expect(readFileSync((await download.path())!, 'utf8')).toContain('<svg');
  await page.getByRole('button', { name: 'Close export', exact: true }).click();

  const source = readFileSync(new URL('../../../core/tests/fixtures/benchmarks/rc_filter.kess', import.meta.url), 'utf8');
  await replaceSource(page, source.replace('resistor R1 1k', 'resistor R1 2k'));
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(results).toContainText('Simulation results are out of date');
  await expect(results.locator('.result-plot')).toHaveCount(0);
  await results.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(results).toHaveAttribute('data-state', 'succeeded', { timeout: 90_000 });
  await expect(results.locator('.assertion-fail')).toHaveCount(2);
  await replaceSource(page, source);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await results.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(results.locator('.assertion-pass')).toHaveCount(5, { timeout: 90_000 });
});

test('explains missing analyses without blocking schematic export and distinguishes an unchecked run', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const source = 'net GND\nnet OUT\nsource V1 5V\nresistor R1 1k\nconnect V1.plus, R1.p1 to OUT\nconnect V1.minus, R1.p2 to GND\n';
  await replaceSource(page, source);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const simulation = page.getByLabel('Circuit simulation');
  await expect(simulation).toContainText('Add an analysis to simulate');
  await expect(simulation.getByRole('button', { name: 'Run simulation', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Export', exact: true })).toBeEnabled();
  await expect(page.getByTestId('canonical-schematic')).toBeVisible();
  await replaceSource(page, `${source}simulate op\n`);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await simulation.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(simulation).toHaveAttribute('data-state', 'succeeded', { timeout: 90_000 });
  await expect(simulation).toContainText('Simulation completed without requirement checks');
  await expect(simulation.locator('.op-grid')).toContainText('5.000 V');
});

test('guides missing and mismatched local model files to the binding dialog', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
  await page.getByRole('menuitem', { name: 'External Comparator', exact: true }).click();
  const simulation = page.getByLabel('Circuit simulation');
  await expect(simulation).toContainText('Model file needed');
  await simulation.getByRole('button', { name: 'Choose model files' }).click();
  const details = page.getByRole('dialog', { name: 'Circuit details' });
  await details.getByLabel('Model file for CMP').setInputFiles({ name: 'wrong.lib', mimeType: 'text/plain', buffer: Buffer.from('* wrong file\n') });
  await expect(page.getByLabel('Diagnostics')).toContainText('KES-C016');
  await details.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(simulation).toContainText('Model file needed');
  await simulation.getByRole('button', { name: 'Choose model files' }).click();
  await details.getByLabel('Model file for CMP').setInputFiles({ name: 'comparator.lib', mimeType: 'text/plain', buffer: readFileSync(new URL('../../../examples/models/comparator.lib', import.meta.url)) });
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await details.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(simulation.getByRole('button', { name: 'Run simulation', exact: true })).toBeEnabled();
});

test('offers corresponding source and license from the interactive Web Hub', async ({ page }) => {
  await page.goto('/#editor');
  await page.getByRole('button', { name: 'Help', exact: true }).click();
  const sourceLink = page.getByRole('menuitem', { name: /corresponding source code/i });
  await expect(sourceLink).toHaveAttribute('href', 'https://github.com/Stapimaz/Kessetsu');
  await expect(sourceLink).toContainText('Corresponding source');
  await expect(page.getByRole('menuitem', { name: `What’s new in ${productVersion}` })).toHaveAttribute('href', '/changelog/');
  await expect(page.locator('.menu-version')).toHaveText(`Kessetsu ${productVersion}`);
  await expect(page.getByRole('menuitem', { name: 'License', exact: true })).toHaveAttribute('href', '/LICENSE.txt');
  await expect(page.getByRole('menuitem', { name: 'First circuit tutorial', exact: true })).toHaveAttribute('href', '/docs/guides/tutorial/');
  await expect(page.getByRole('menuitem', { name: 'Troubleshooting', exact: true })).toHaveAttribute('href', '/docs/guides/troubleshooting/');
});

test('exposes keyboard controls and a usable mobile workspace', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#editor');
  await expect(page.getByLabel('Kessetsu source editor')).toBeVisible();
  await expect(page.getByLabel('Canonical schematic')).toBeVisible();
  await expect(page.getByLabel('Circuit simulation')).toBeVisible();
  await page.getByLabel('Canonical schematic').locator('.schematic-surface').focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('button', { name: 'Run' })).toBeEnabled();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.getByRole('button', { name: /theme/i })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Check', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Simulation', exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: 'test-results/editor-mobile.png', fullPage: true });
  }
});
