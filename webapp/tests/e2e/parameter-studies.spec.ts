import { expect, test, type Page } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { StudyResults } from '../../src/experiment';

test.setTimeout(120_000);
const specPath = (name: string) => resolve(`../examples/studies/${name}.kessstudy.json`);
async function openStudy(page: Page) {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Parameter study…', exact: true }).click();
  return page.getByRole('dialog', { name: 'Parameter study', exact: true });
}
async function downloadReport(page: Page): Promise<StudyResults> {
  const dialog = page.getByRole('dialog', { name: 'Parameter study', exact: true });
  await dialog.getByLabel('Export current study').selectOption('json');
  const downloaded = page.waitForEvent('download');
  await dialog.getByRole('button', { name: 'Download', exact: true }).click();
  const file = await downloaded;
  return JSON.parse(readFileSync((await file.path())!, 'utf8'));
}

test('loaded-filter configuration, all outcomes, native parity, reports and source continuation', async ({ page }, testInfo) => {
  const dialog = await openStudy(page);
  await dialog.getByLabel('Open study file').setInputFiles(specPath('loaded-filter'));
  await expect(dialog.getByLabel('Sweep values 1')).toHaveValue('820Ohm, 1kOhm, 1.2kOhm');
  await dialog.getByLabel('Sweep parameter 1').selectOption('capacitance');
  await expect(dialog.getByLabel('Sweep values 1')).toHaveValue(/F/);
  const overBudget = JSON.parse(readFileSync(specPath('loaded-filter'), 'utf8'));
  overBudget.axes[0].values = { kind: 'linear', start: '820Ohm', stop: '1200Ohm', points: 33 };
  await dialog.getByLabel('Open study file').setInputFiles({ name: 'large-study.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(overBudget)) });
  await dialog.getByRole('button', { name: 'Run study', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Browser limit: 32');
  await dialog.getByLabel('Open study file').setInputFiles(specPath('loaded-filter'));
  const sourceBefore = await page.locator('.view-lines').innerText();
  await dialog.getByRole('button', { name: 'Run study', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Study complete', { timeout: 90_000 });
  await expect(dialog.getByTestId('study-summary')).toContainText('6 total · 3 passed');
  await expect(dialog.locator('.study-case-failed')).toHaveCount(3);
  await expect(dialog.getByTestId('study-plot').locator('polyline')).toHaveCount(6);
  await expect(dialog).toContainText('Best feasible evaluated case:');
  const browser = await downloadReport(page);
  mkdirSync(testInfo.outputDir, { recursive: true });
  const binary = process.env.KESSETSU_TEST_BINARY ?? resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const nativeOutput = testInfo.outputPath('native-results.json');
  const ran = spawnSync(binary, ['study', 'run', specPath('loaded-filter'), '--output', nativeOutput, '--format', 'json'], { encoding: 'utf8', timeout: 60_000 });
  expect(ran.status, `${ran.stdout} ${ran.stderr}`).toBe(1);
  const native = JSON.parse(readFileSync(nativeOutput, 'utf8')) as StudyResults;
  expect(browser.plan.cases).toEqual(native.plan.cases);
  for (let i = 0; i < browser.cases.length; i++) {
    expect(browser.cases[i].status).toBe(native.cases[i].status);
    for (const [name, measurement] of Object.entries(browser.cases[i].measurements)) {
      const reference = native.cases[i].measurements[name].value!;
      expect(Math.abs(measurement.value! - reference)).toBeLessThan(Math.max(1e-7, Math.abs(reference) * 1e-4));
    }
  }
  for (const format of ['csv', 'data_csv', 'svg', 'html']) {
    await dialog.getByLabel('Export current study').selectOption(format);
    const download = page.waitForEvent('download');
    await dialog.getByRole('button', { name: 'Download', exact: true }).click();
    const text = readFileSync((await (await download).path())!, 'utf8');
    expect(text.length).toBeGreaterThan(100);
    if (format === 'data_csv') expect(text.split('\n').length).toBeGreaterThan(1000);
  }
  await dialog.evaluate(element => { element.scrollTop = 0; });
  await page.screenshot({ path: testInfo.outputPath('study-results.png') });
  await dialog.getByLabel('Compare study results file').setInputFiles(nativeOutput);
  await expect(dialog.getByTestId('study-plot').locator('polyline')).toHaveCount(12);
  await dialog.getByRole('button', { name: 'Close parameter study' }).click();
  await expect(page.locator('.view-lines')).toHaveText(sourceBefore, { useInnerText: true });
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Parameter study…', exact: true }).click();
  await expect(dialog.getByTestId('study-summary')).toContainText('6 total');
  page.once('dialog', event => event.accept());
  await dialog.getByRole('button', { name: 'Apply parameters', exact: true }).nth(5).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toContainText('1200Ohm');
});

test('cancel/reopen/resume preserves completed evidence and rejects changed constraints', async ({ page }, testInfo) => {
  const dialog = await openStudy(page);
  const spec = JSON.parse(readFileSync(specPath('loaded-filter'), 'utf8'));
  spec.axes = []; spec.objective = null;
  spec.tolerances = { mode: 'monte_carlo', seed: 42, samples: 8, parameters: [{ parameter: 'resistance', relative: 0.05, distribution: 'uniform', group: null }] };
  await dialog.getByLabel('Open study file').setInputFiles({ name: 'seeded.kessstudy.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(spec)) });
  await dialog.getByRole('button', { name: 'Run study', exact: true }).click();
  await expect(dialog.locator('.study-case-passed').first()).toBeVisible({ timeout: 60_000 });
  await dialog.getByRole('button', { name: 'Stop study', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Study stopped');
  const partial = await downloadReport(page);
  expect(partial.cases.some(c => c.status === 'pending' || c.status === 'cancelled')).toBe(true);
  await dialog.getByRole('button', { name: 'Close parameter study' }).click();
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Parameter study…', exact: true }).click();
  await dialog.getByRole('button', { name: 'Resume', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Study complete', { timeout: 90_000 });
  const completed = await downloadReport(page);
  expect(completed.summary.passed).toBe(9);
  for (const row of partial.cases.filter(c => c.status === 'passed')) {
    expect(completed.cases.find(c => c.case_id === row.case_id)?.content_sha256).toBe(row.content_sha256);
  }
  await dialog.getByLabel('Open study file').setInputFiles({ name: 'partial-results.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(partial)) });
  await dialog.getByRole('button', { name: 'Configure', exact: true }).click();
  await dialog.getByText('Conditions and portable specification', { exact: true }).click();
  await dialog.getByText('Advanced JSON: revisions and combined studies', { exact: true }).click();
  const changed = { ...spec, source: spec.source.replace('> 0.8', '> 0.7') };
  await dialog.getByLabel('Advanced study specification').fill(JSON.stringify(changed));
  await dialog.getByRole('button', { name: 'Apply specification', exact: true }).click();
  await dialog.getByRole('button', { name: 'Resume', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Cannot resume');
  await page.setViewportSize({ width: 760, height: 900 });
  await dialog.evaluate(element => { element.scrollTop = 0; });
  await page.screenshot({ path: testInfo.outputPath('study-config.png') });
});

test('browser driver conditions preserve failures, temperature behavior and native parity', async ({ page }, testInfo) => {
  const dialog = await openStudy(page);
  await dialog.getByLabel('Open study file').setInputFiles(specPath('transistor-driver'));
  await dialog.getByRole('button', { name: 'Run study', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Study complete', { timeout: 90_000 });
  const report = await downloadReport(page);
  expect(report.summary.total).toBe(12);
  expect(report.summary.errors).toBe(0);
  expect(report.summary.passed).toBeGreaterThan(0);
  expect(report.summary.failed).toBeGreaterThan(0);
  expect(report.cases[4].measurements.transistor_power.value).not.toBe(report.cases[5].measurements.transistor_power.value);
  mkdirSync(testInfo.outputDir, { recursive: true });
  const binary = process.env.KESSETSU_TEST_BINARY ?? resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const output = testInfo.outputPath('native-driver-results.json');
  const ran = spawnSync(binary, ['study', 'run', specPath('transistor-driver'), '--output', output, '--format', 'json'], { encoding: 'utf8', timeout: 60_000 });
  expect(ran.status, `${ran.stdout} ${ran.stderr}`).toBe(1);
  const native = JSON.parse(readFileSync(output, 'utf8')) as StudyResults;
  expect(report.plan.cases).toEqual(native.plan.cases);
  for (let i = 0; i < report.cases.length; i++) {
    expect(report.cases[i].status).toBe(native.cases[i].status);
    for (const [name, measurement] of Object.entries(report.cases[i].measurements)) {
      const reference = native.cases[i].measurements[name].value!;
      expect(Math.abs(measurement.value! - reference)).toBeLessThan(Math.max(1e-7, Math.abs(reference) * 1e-3));
    }
  }
});
