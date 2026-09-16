import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { encodeShareFragment } from '../../src/share';

test('parameter source compiles, simulates and edits without changing fixed acceptance limits', async ({ page }) => {
  const source = readFileSync(new URL('../../../core/tests/fixtures/parameters/loaded_divider.kess', import.meta.url), 'utf8');
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Parameterized divider');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByRole('img', { name: 'Kessetsu schematic' })).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 30_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(2);
  await expect(page.locator('.op-grid > div').filter({ hasText: 'V(out)' })).toContainText('3.000 V');

  await page.locator('.monaco-editor .view-lines').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(source.replace('param target: V = 3V', 'param target: V = 2V'));
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run simulation', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 30_000 });
  await expect(page.locator('.op-grid > div').filter({ hasText: 'V(out)' })).toContainText('2.000 V');
  await expect(summary.locator('.assertion-fail')).toHaveCount(1);
  await expect(summary.locator('.assertion-pass')).toHaveCount(1);
});
