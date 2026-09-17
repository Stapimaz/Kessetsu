import { expect, test } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

for (const example of [
  { label: 'Reusable Filters', file: 'reusable_filters.kess', count: 4 },
  { label: 'Reusable Amplifiers', file: 'reusable_amplifiers.kess', count: 6 },
]) {
  test(`${example.label} opens from Examples and matches native simulation`, async ({ page }) => {
    const source = readFileSync(new URL(`../../../examples/${example.file}`, import.meta.url), 'utf8');
    const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
    const native = spawnSync(binary, ['test', '-', '--format', 'json'], { input: source, encoding: 'utf8', timeout: 60_000 });
    expect(native.status).toBe(0);
    const report = JSON.parse(native.stdout);
    await page.goto('/#editor');
    await expect(page.getByTestId('compile-success')).toBeVisible();
    await page.getByRole('button', { name: 'File', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
    await page.getByRole('menuitem', { name: example.label, exact: true }).click();
    await expect(page.locator('.document-title')).toHaveText(example.label);
    await expect(page.getByTestId('compile-success')).toBeVisible();
    await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-quality', 'pass');
    await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
    const summary = page.getByTestId('simulation-summary');
    await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
    await expect(summary.locator('.assertion-pass')).toHaveCount(example.count);
    for (const assertion of report.assertions.assertions) {
      expect(assertion.status).toBe('PASS');
      const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
      expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
    }
    await page.getByTestId('canonical-schematic').screenshot({ path: `test-results/${example.file}.png` });
    await page.getByRole('button', { name: 'Share circuit' }).click();
    const share = page.getByRole('dialog', { name: 'Share circuit' });
    await share.getByRole('button', { name: 'Copy link' }).click();
    await expect(share.getByRole('status')).toContainText(/Link (copied|created)/);
    expect(page.url()).toContain('#kessetsu=1.');
    await page.reload();
    await expect(page.getByTestId('compile-success')).toBeVisible();
    await expect(page.locator('.document-title')).toHaveText(example.label);
    await expect(page.locator('.monaco-editor .view-lines')).toContainText('module ');
  });
}
