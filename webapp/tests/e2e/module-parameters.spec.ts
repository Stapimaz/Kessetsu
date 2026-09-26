import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { encodeShareFragment } from '../../src/share';

test('independent filter instances simulate in parity and remain independently editable', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(new URL('../../../core/tests/fixtures/parameters/rc_instances.kess', import.meta.url), 'utf8');
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const native = JSON.parse(execFileSync(binary, ['test', '-', '--format', 'json'], {
    input: source, encoding: 'utf8', timeout: 60_000,
  }));
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Independent RC filters');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await expect(page.getByRole('img', { name: 'Kessetsu schematic' })).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(8);
  for (const assertion of native.assertions.assertions) {
    expect(assertion.status).toBe('PASS');
    const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
    expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
  }
  await page.locator('.monaco-editor .view-lines').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(source.replace('base_cutoff * 4', 'base_cutoff * 2'));
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run simulation', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  // Fixed limits remain fixed: the modified fast filter fails its original
  // 2 kHz target, while the slow filter's independent 500 Hz bounds still pass.
  const requirementRow = (code: string) => summary.locator('tr').filter({
    has: page.locator(`[data-assertion-code="${code}"]`),
  });
  await expect(requirementRow('KES-T001').locator('.assertion-pass')).toHaveText('PASS');
  await expect(requirementRow('KES-T002').locator('.assertion-pass')).toHaveText('PASS');
  await expect(requirementRow('KES-T003').locator('.assertion-fail')).toHaveText('FAIL');
});
