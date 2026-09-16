import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { encodeShareFragment } from '../../src/share';

test('builtin diodes and the default model simulate without replacement models', async ({ page }) => {
  const source = readFileSync(new URL('../../../core/tests/fixtures/models/builtin_diodes.kess', import.meta.url), 'utf8');
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Builtin diodes');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 30_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(6);
  await expect(summary.locator('.assertion-fail, .assertion-error')).toHaveCount(0);
});
