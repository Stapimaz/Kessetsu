import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { encodeShareFragment } from '../../src/share';

test('typed assertion times, frequency and threshold match native and retain fixed bounds', async ({ page }) => {
  const source = readFileSync(new URL('../../../core/tests/fixtures/parameters/assertions.kess', import.meta.url), 'utf8');
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const native = JSON.parse(execFileSync(binary, ['test', '-', '--format', 'json'], { input: source, encoding: 'utf8', timeout: 60_000 }));
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Parameterized measurements');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  const run = page.getByRole('button', { name: 'Run simulation', exact: true });
  const summary = page.getByTestId('simulation-summary');
  await run.click();
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(4);
  for (const assertion of native.assertions.assertions) {
    expect(assertion.status).toBe('PASS');
    const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
    expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
  }
  await page.locator('.monaco-editor .view-lines').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(source.replace('param amplitude: V = 1V', 'param amplitude: V = 2V'));
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(run).toBeEnabled();
  await run.click();
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  const row = (code: string) => summary.locator('tr').filter({ has: page.locator(`[data-assertion-code="${code}"]`) });
  await expect(row('KES-T001').locator('.assertion-pass')).toHaveText('PASS');
  await expect(row('KES-T004').locator('.assertion-fail')).toHaveText('FAIL');
});
