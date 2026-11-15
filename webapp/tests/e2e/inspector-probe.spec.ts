import { expect, test } from '@playwright/test';
import { encodeShareFragment } from '../../src/share';

test('inspects AC component ΔV/I readout, validates & auto-reruns quick-tweak, and plots V/I signals', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Split schematic and simulation' }).click();

  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });

  await page.getByRole('button', { name: 'Find and inspect schematic' }).click();
  const navigator = page.locator('.schematic-navigator');
  await expect(navigator).toBeVisible();

  await navigator.getByPlaceholder('Find component or net').fill('R1');
  await navigator.getByRole('button', { name: /\bR1\b/ }).first().click();
  await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-selected-component', 'R1');

  const simBlock = navigator.locator('.inspector-sim-block');
  await expect(simBlock).toBeVisible();
  await expect(simBlock).toContainText('Simulated');
  await expect(simBlock).toContainText('ΔV');
  await expect(simBlock).toContainText('I');

  const valueInput = navigator.getByLabel('Component value');
  const applyButton = navigator.getByRole('button', { name: 'Apply', exact: true });
  await valueInput.fill('5V');
  await expect(valueInput).toHaveClass(/is-invalid/);
  await expect(applyButton).toBeDisabled();
  await valueInput.fill('1k');
  await expect(valueInput).not.toHaveClass(/is-invalid/);
  await expect(applyButton).toBeEnabled();

  await navigator.getByRole('button', { name: '+10%', exact: true }).click();
  await expect(page.locator('.view-lines')).toContainText('resistor R1 1.1k');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(valueInput).toHaveValue('1.1k');

  await navigator.getByRole('button', { name: 'Plot V', exact: true }).click();
  await expect(page.getByLabel('Signal', { exact: true })).toHaveValue(/v\(in,\s*out\)/i);

  await navigator.getByRole('button', { name: 'Plot I', exact: true }).click();
  await expect(page.getByLabel('Signal', { exact: true })).toHaveValue(/i\(r1\)/i);
});

test('recovers cleanly from malformed #kessetsu= hash and handles dynamic hashchange navigation', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/#kessetsu=malformed-corrupt-fragment');

  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 30_000 });
  const shareAlert = page.locator('.global-error', { hasText: 'Could not open shared circuit link' });
  await expect(shareAlert).toBeVisible();
  await expect(page.locator('.view-lines')).toContainText('resistor R1 1k');

  await shareAlert.getByRole('button', { name: 'Dismiss', exact: true }).click();
  await expect(shareAlert).toHaveCount(0);

  const sharedSource = [
    'net GND',
    'net OUT',
    'source VIN 3.3V',
    'resistor RLOAD 1k',
    'connect VIN.plus, RLOAD.p1 to OUT',
    'connect VIN.minus, RLOAD.p2 to GND',
    'simulate op',
    'assert value(V(OUT)) > 3.2V',
  ].join('\n');
  const fragment = await encodeShareFragment(sharedSource, 'kessetsu.compile.v9', null, 'Hashchange Test');
  await page.evaluate((nextHash) => {
    window.location.hash = nextHash;
  }, fragment);

  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('.document-title')).toHaveText('Hashchange Test');
  await expect(page.locator('.view-lines')).toContainText('resistor RLOAD 1k');
  await expect(shareAlert).toHaveCount(0);
});

test('loads LM358 & LED Driver preset example and passes all browser simulation assertions', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
  await page.getByRole('menuitem', { name: 'LM358 & LED Driver', exact: true }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Split schematic and simulation' }).click();

  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(6);
  await expect(summary.locator('.assertion-fail, .assertion-error, .assertion-skipped')).toHaveCount(0);
});
