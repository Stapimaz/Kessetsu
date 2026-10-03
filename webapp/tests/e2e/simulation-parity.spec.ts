import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { SimulationResult } from '../../src/simulation/types';

async function moveOverPlot(page: Page, plot: ReturnType<Page['locator']>, fraction: number) {
  const point = await plot.evaluate((element, fraction) => {
    const svg = element as SVGSVGElement;
    const matrix = svg.getScreenCTM();
    if (!matrix) throw new Error('result plot has no screen transform');
    const point = svg.createSVGPoint();
    point.x = 42 + 536 * fraction;
    point.y = 96;
    const screen = point.matrixTransform(matrix);
    return { x: screen.x, y: screen.y };
  }, fraction);
  await page.mouse.move(point.x, point.y);
}

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
  await page.addInitScript(() => {
    const NativeWorker = Worker;
    window.Worker = class extends NativeWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        this.addEventListener('message', (event) => {
          if (event.data.type === 'result') (window as unknown as { recordedSimulation: SimulationResult }).recordedSimulation = event.data.result;
        });
      }
    };
  });
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
  await expect(summary.locator('.threshold-line')).toHaveCount(0);
  await expect(summary).toContainText('not drawn as instantaneous waveform limits');
  const transientPlot = summary.locator('.result-plot');
  const recorded = await page.evaluate(() => (window as unknown as { recordedSimulation: SimulationResult }).recordedSimulation);
  const transient = recorded.datasets.find((item) => item.data.kind === 'transient')!.data;
  if (transient.kind !== 'transient') throw new Error('missing real transient data');
  // Independent oracle from real solver samples, not the frontend's lookup helper.
  const axis = transient.axis.values;
  const fraction = 0.37;
  const target = Math.min(...axis) + fraction * (Math.max(...axis) - Math.min(...axis));
  const nearest = axis.reduce((best, value, index) => Math.abs(value - target) < Math.abs(axis[best] - target) ? index : best, 0);
  expect(axis[Math.round(fraction * (axis.length - 1))]).not.toBe(axis[nearest]);
  await moveOverPlot(page, transientPlot, fraction);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', String(axis[nearest]));
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-signal-value', String(transient.signals.out[nearest]));
  expect(Number(await summary.locator('.cursor-line').getAttribute('x1'))).toBeCloseTo(Number(await summary.locator('.cursor-dot').getAttribute('cx')), 8);
  await moveOverPlot(page, transientPlot, 0);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', String(Math.min(...axis)));
  await moveOverPlot(page, transientPlot, 1);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', String(Math.max(...axis)));
  await moveOverPlot(page, transientPlot, 0.75);
  await page.mouse.wheel(0, -100);
  await expect(summary.getByRole('button', { name: 'Reset zoom' })).toBeVisible();
  await expect(transientPlot.locator('.axis-label').first()).toContainText('µs');
  await summary.getByRole('button', { name: 'Reset zoom' }).click();
  await summary.getByRole('tab', { name: 'ac' }).click();
  await expect(summary.locator('.result-plot')).toHaveCount(2);
  await expect(summary).toContainText('not output/input gain');
  await expect(summary.locator('.result-plot').first().locator('.axis-label').last()).toContainText('dBV');
  const ac = recorded.datasets.find((item) => item.data.kind === 'ac')!.data;
  if (ac.kind !== 'ac') throw new Error('missing real AC data');
  const frequencies = ac.frequency_hz;
  const logTarget = Math.log10(frequencies[0]) + 0.4 * Math.log10(frequencies.at(-1)! / frequencies[0]);
  const acNearest = frequencies.reduce((best, value, index) => Math.abs(Math.log10(value) - logTarget) < Math.abs(Math.log10(frequencies[best]) - logTarget) ? index : best, 0);
  await moveOverPlot(page, summary.locator('.result-plot').first(), 0.4);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', String(frequencies[acNearest]));
  await summary.getByRole('tab', { name: 'dc sweep' }).click();
  await expect(summary.locator('.result-plot')).toHaveCount(1);
  await expect(summary).toContainText('Sweep of VIN (V)');
  if (process.env.KESSETSU_E2E_SCREENSHOTS) await page.screenshot({ path: 'test-results/dc-plot-units.png' });
});

test('labels a descending current-source sweep in amperes and selects its physical endpoints', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await replaceSource(page, 'net GND\nnet OUT\ncurrent_source I1 1mA\nresistor R1 1k\nconnect I1.plus, R1.p1 to OUT\nconnect I1.minus, R1.p2 to GND\nsimulate dc I1 2mA -2mA -1mA\n');
  await page.getByRole('button', { name: 'Run simulation' }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded');
  await expect(summary).toContainText('Sweep of I1 (A)');
  const plot = summary.locator('.result-plot');
  await expect(plot.locator('.axis-label').first()).toHaveText('-2.000 mA');
  await expect(plot.locator('.axis-label').nth(1)).toHaveText('2.000 mA');
  await moveOverPlot(page, plot, 0);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', '-0.002');
  await moveOverPlot(page, plot, 1);
  await expect(summary.locator('.cursor-readout')).toHaveAttribute('data-axis-value', '0.002');
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await summary.getByRole('button', { name: 'Maximize simulation panel' }).click();
    await page.screenshot({ path: 'test-results/current-sweep-plot.png' });
  }
});
