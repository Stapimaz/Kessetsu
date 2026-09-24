import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import type { DataComparison } from '../../src/research';

test.setTimeout(60_000);

const observed = `time,out
0,0
0.001,0.630
0.002,0.860
0.003,0.950
0.004,0.982
0.005,0.993
`;
const reference = `time,out
0,0
0.001,0.632
0.002,0.865
0.003,0.950
0.004,0.982
0.005,0.993
`;

test('maps two local CSV files, compares every point and downloads reproducible evidence', async ({ page }, testInfo) => {
  const nonReadRequests: string[] = [];
  page.on('request', (request) => {
    if (!['GET', 'HEAD'].includes(request.method())) nonReadRequests.push(`${request.method()} ${request.url()}`);
  });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  const sourceBefore = await page.locator('.view-lines').innerText();
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Research data…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Research data', exact: true });

  await dialog.getByLabel('Choose Observed data CSV').setInputFiles({ name: 'observed.csv', mimeType: 'text/csv', buffer: Buffer.from(observed) });
  await expect(dialog).toContainText('6 data records');
  await dialog.screenshot({ path: testInfo.outputPath('research-mapping.png') });
  await dialog.getByLabel('Origin').selectOption('measured');
  await dialog.getByRole('button', { name: 'Import mapped dataset' }).click();
  await expect(dialog.getByRole('button', { name: /1 · Observed/ })).toContainText('Observed');

  await dialog.getByLabel('Choose Reference data CSV').setInputFiles({ name: 'reference.csv', mimeType: 'text/csv', buffer: Buffer.from(reference) });
  await dialog.getByLabel('Origin').selectOption('published_simulation');
  await dialog.getByRole('button', { name: 'Import mapped dataset' }).click();
  await expect(dialog.getByRole('button', { name: '3 · Compare' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByRole('button', { name: 'Compare datasets' }).click();

  await expect(dialog).toContainText('Comparison complete');
  const row = dialog.locator('.research-metrics tbody tr').first();
  await expect(row).toContainText('out');
  await expect(row).toContainText('6/6');
  await expect(dialog.locator('.research-plot polyline')).toHaveCount(2);
  await dialog.screenshot({ path: testInfo.outputPath('research-comparison.png') });

  const download = page.waitForEvent('download');
  await dialog.getByRole('button', { name: 'Download evidence JSON' }).click();
  const artifact = await download;
  expect(artifact.suggestedFilename()).toBe('observed-data-versus-reference.kesscompare.json');
  const report = JSON.parse(readFileSync((await artifact.path())!, 'utf8')) as DataComparison;
  expect(report.schema_version).toBe('kessetsu.data-comparison.v1');
  expect(report.data_origin).toBe('measured');
  expect(report.reference_origin).toBe('published_simulation');
  expect(report.signals[0].metrics).toMatchObject({ total: 6, matched: 6, unmatched: 0, excluded_by_window: 0 });
  expect(report.signals[0].metrics.mae).toBeCloseTo(0.0011666667, 8);
  expect(report.signals[0].points.map((point) => point.source_record)).toEqual([2, 3, 4, 5, 6, 7]);
  expect(nonReadRequests).toEqual([]);
  await dialog.getByRole('button', { name: 'Close research data' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toHaveText(sourceBefore, { useInnerText: true });
});

test('compares observed CSV directly with the current typed simulation projection', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await page.getByRole('button', { name: 'Run simulation' }).click();
  await expect(page.getByText(/requirements passed/)).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Research data…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Research data', exact: true });
  const measured = `frequency,out
10,0.9999
1000,0.7071
100000,0.0100
`;
  await dialog.getByLabel('Choose Observed data CSV').setInputFiles({ name: 'bench.csv', mimeType: 'text/csv', buffer: Buffer.from(measured) });
  await dialog.getByLabel('Origin').selectOption('measured');
  await dialog.getByLabel('Quantity').nth(0).selectOption('Hertz');
  await dialog.getByRole('button', { name: 'Import mapped dataset' }).click();

  await expect(dialog.getByText('Use the current simulation')).toBeVisible();
  await expect(dialog.getByLabel('Analysis')).toHaveValue('0');
  const vector = dialog.getByLabel('Simulation vector');
  const options = await vector.locator('option').allTextContents();
  const output = options.find((option) => ['V(OUT)', 'OUT'].includes(option.toUpperCase()));
  expect(output).toBeTruthy();
  await vector.selectOption({ label: output! });
  await dialog.getByLabel('Logical name').last().fill('out');
  await dialog.getByRole('button', { name: 'Use simulation as reference' }).click();
  await dialog.getByLabel('Interpolation').selectOption('log_axis');
  await dialog.getByRole('button', { name: 'Compare datasets' }).click();
  await expect(dialog).toContainText('Comparison complete');

  const download = page.waitForEvent('download');
  await dialog.getByRole('button', { name: 'Download evidence JSON' }).click();
  const report = JSON.parse(readFileSync((await (await download).path())!, 'utf8')) as DataComparison;
  expect(report.data_origin).toBe('measured');
  expect(report.reference_origin).toBe('simulation');
  expect(report.signals[0].metrics).toMatchObject({ total: 3, matched: 3, unmatched: 0 });
});
