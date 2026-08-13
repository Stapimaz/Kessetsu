import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { encodeShareFragment } from '../../src/share';

test('opens the final power-amplifier source from a versioned URL and runs the full product path', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(
    new URL('../../../core/tests/fixtures/benchmarks/power_amplifier.kess', import.meta.url),
    'utf8',
  );
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v3', null);
  await page.goto(`/${fragment}`);

  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('.share-status')).toContainText('Shared circuit loaded');
  await expect(page.getByRole('img', { name: 'Kessetsu schematic' })).toBeVisible();
  await expect(page.locator('.view-lines')).toContainText('Four-stage amplifier');

  await page.getByRole('button', { name: 'Run' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(12);
  await expect(summary.locator('.assertion-fail, .assertion-error, .assertion-skipped')).toHaveCount(0);
  await page.screenshot({ path: 'test-results/power-amplifier-result.png', fullPage: true });

  const downloadPromise = page.waitForEvent('download');
  await page.locator('[data-export-format="svg"]').click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe('circuit.svg');

  await page.getByRole('button', { name: 'Share circuit' }).click();
  await expect(page.locator('.share-status')).toContainText(/source and package versions embedded/);
  expect(page.url()).toContain('#kessetsu=1.');
});
