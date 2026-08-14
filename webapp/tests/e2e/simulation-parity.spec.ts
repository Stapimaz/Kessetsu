import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

async function replaceSource(page: Page, source: string) {
  await expect(page.locator('.monaco-editor')).toBeVisible();
  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Backspace');
  await expect(page.getByTestId('compile-success')).not.toBeVisible();
  await page.keyboard.insertText(source);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
}

test('runs the canonical RC filter in a worker and evaluates Core assertions', async ({ page }) => {
  test.setTimeout(120_000);
  page.on('console', (message) => console.log(`[browser:${message.type()}] ${message.text()}`));
  page.on('pageerror', (error) => console.log(`[browser:error] ${error.message}`));
  page.on('requestfailed', (request) => console.log(`[browser:requestfailed] ${request.url()} ${request.failure()?.errorText}`));
  page.on('worker', (worker) => console.log(`[browser:worker] ${worker.url()}`));
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('.view-lines')).toContainText('Canonical first-order RC low-pass');

  await page.getByRole('button', { name: 'Run' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary).toContainText('ngspice-45.2+');
  await expect(summary.locator('.assertion-pass')).toHaveCount(5);
  await expect(summary.locator('.assertion-fail, .assertion-error, .assertion-skipped')).toHaveCount(0);

  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const nativeOutput = resolve('test-results/native-rc.spice');
  const native = JSON.parse(execFileSync(binary, [
    'test',
    resolve('../core/tests/fixtures/benchmarks/rc_filter.kess'),
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
    new URL('../../../core/tests/fixtures/benchmarks/browser_analysis_matrix.kess', import.meta.url),
    'utf8',
  );
  await page.goto('/#editor');
  await replaceSource(page, source);

  await page.getByRole('button', { name: 'Run' }).click();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'cancelled');

  await page.getByRole('button', { name: 'Run' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 100_000 });
  await expect(summary.getByTestId('dataset-kind')).toHaveText([
    'operating point',
    'transient',
    'ac',
    'dc sweep',
  ]);
  await expect(summary.locator('.assertion-pass')).toHaveCount(5);
  await summary.getByRole('tab', { name: 'operating point' }).click();
  await expect(summary.locator('.op-grid')).toBeVisible();
  await summary.getByRole('tab', { name: 'transient' }).click();
  await expect(summary.locator('.result-plot')).toHaveCount(1);
  await expect(summary.locator('.threshold-line')).toHaveCount(1);
  const transientPlot = summary.locator('.result-plot');
  const plotPoints = await transientPlot.evaluate((element) => {
    const svg = element as SVGSVGElement;
    const matrix = svg.getScreenCTM();
    if (!matrix) throw new Error('result plot has no screen transform');
    const screenPoint = (x: number) => {
      const point = svg.createSVGPoint();
      point.x = x;
      point.y = 96;
      const screen = point.matrixTransform(matrix);
      return { x: screen.x, y: screen.y };
    };
    return { left: screenPoint(44), right: screenPoint(576) };
  });
  await page.mouse.move(plotPoints.left.x, plotPoints.left.y);
  await expect(summary.locator('.cursor-readout')).toBeVisible();
  expect(Math.abs(Number(await summary.locator('.cursor-line').getAttribute('x1')) - 44)).toBeLessThan(0.5);
  await page.mouse.move(plotPoints.right.x, plotPoints.right.y);
  expect(Math.abs(Number(await summary.locator('.cursor-line').getAttribute('x1')) - 576)).toBeLessThan(0.5);
  await summary.getByRole('tab', { name: 'ac' }).click();
  await expect(summary.locator('.result-plot')).toHaveCount(2);
  await summary.getByRole('tab', { name: 'dc sweep' }).click();
  await expect(summary.locator('.result-plot')).toHaveCount(1);
});
