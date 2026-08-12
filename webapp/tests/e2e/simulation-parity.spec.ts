import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

async function replaceSource(page: Page, source: string) {
  await expect(page.locator('.monaco-editor')).toBeVisible();
  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(source);
  await expect(page.getByTestId('compile-success')).toBeVisible();
}

test('runs the canonical RC filter in a worker and evaluates Core assertions', async ({ page }) => {
  test.setTimeout(120_000);
  page.on('console', (message) => console.log(`[browser:${message.type()}] ${message.text()}`));
  page.on('pageerror', (error) => console.log(`[browser:error] ${error.message}`));
  page.on('requestfailed', (request) => console.log(`[browser:requestfailed] ${request.url()} ${request.failure()?.errorText}`));
  page.on('worker', (worker) => console.log(`[browser:worker] ${worker.url()}`));
  const source = readFileSync(
    new URL('../../../core/tests/fixtures/benchmarks/rc_filter.nl', import.meta.url),
    'utf8',
  );
  await page.goto('/');
  await replaceSource(page, source);

  await page.getByRole('button', { name: 'Simüle Et' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary).toContainText('ngspice-45.2+');
  await expect(summary.locator('.assertion-pass')).toHaveCount(5);
  await expect(summary.locator('.assertion-fail, .assertion-error, .assertion-skipped')).toHaveCount(0);

  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'netlang.exe' : 'netlang');
  const nativeOutput = resolve('test-results/native-rc.spice');
  const native = JSON.parse(execFileSync(binary, [
    'test',
    resolve('../core/tests/fixtures/benchmarks/rc_filter.nl'),
    '--output',
    nativeOutput,
    '--force',
    '--format',
    'json',
  ], { encoding: 'utf8' }));
  expect(native.assertions.summary).toMatchObject({ passed: 5, failed: 0, errors: 0, skipped: 0 });
  for (const assertion of native.assertions.assertions) {
    const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
    expect(Math.abs(actual - assertion.actual)).toBeLessThanOrEqual(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
  }
});

test('normalizes OP, transient, AC and DC sweep results and restarts after cancellation', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(
    new URL('../../../core/tests/fixtures/benchmarks/browser_analysis_matrix.nl', import.meta.url),
    'utf8',
  );
  await page.goto('/');
  await replaceSource(page, source);

  await page.getByRole('button', { name: 'Simüle Et' }).click();
  await page.getByRole('button', { name: 'İptal' }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'cancelled');

  await page.getByRole('button', { name: 'Simüle Et' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary.getByTestId('dataset-kind')).toHaveText([
    'operating_point',
    'transient',
    'ac',
    'dc_sweep',
  ]);
  await expect(summary.locator('.assertion-pass')).toHaveCount(4);
});
