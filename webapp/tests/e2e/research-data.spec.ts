import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import type { DataComparison } from '../../src/research';

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
