import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

test('keeps agent proposals separate until verified, accepted and explicitly reverted', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  const healthySource = readFileSync(new URL('../../../core/tests/fixtures/benchmarks/rc_filter.kess', import.meta.url), 'utf8');
  const failingSource = healthySource.replace('159.154943nF', '300nF');
  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.insertText(failingSource);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const failingResults = page.getByTestId('simulation-summary');
  await expect(failingResults).toHaveAttribute('data-state', 'succeeded', { timeout: 90_000 });
  await expect(failingResults.locator('.assertion-fail')).not.toHaveCount(0);
  const originalSource = await page.locator('.view-lines').innerText();

  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
  await dialog.getByPlaceholder(/Design for 2 W RMS/).fill('Restore the intended 1 kHz cutoff so every existing assertion passes.');
  const taskDownload = page.waitForEvent('download');
  await dialog.getByRole('button', { name: 'Download task file', exact: true }).click();
  const taskFile = await taskDownload;
  const task = JSON.parse(readFileSync((await taskFile.path())!, 'utf8')) as {
    circuit: { source: string; source_sha256: string };
  };
  const proposal = {
    schema_version: 'kessetsu.agent-proposal.v1',
    base_source_sha256: task.circuit.source_sha256,
    proposed_source: `${task.circuit.source.replace('300nF', '159.154943nF').trimEnd()}\n// reviewed agent proposal\n`,
    summary: 'Restored the capacitor value required for the existing 1 kHz assertions.',
  };
  await dialog.getByLabel('Open agent proposal JSON').setInputFiles({
    name: 'proposal.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(proposal)),
  });
  await expect(dialog.getByText('Core compile and connectivity')).toBeVisible();
  await expect(dialog.getByText('3 changed lines')).toBeVisible();

  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toHaveText(originalSource, { useInnerText: true });

  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toBeVisible({ timeout: 90_000 });
  await expect(dialog.getByTestId('agent-verification')).toContainText('5/5 assertions passed');
  await dialog.getByRole('button', { name: 'Apply to editor', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toContainText('reviewed agent proposal');

  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: /Undo AI agent change/ }).click();
  await expect(page.locator('.view-lines')).toHaveText(originalSource, { useInnerText: true });
});
