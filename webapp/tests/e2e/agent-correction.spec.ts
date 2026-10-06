import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

test('recovers a rejected agent reply and failed requirement without changing the original request', async ({ page }, testInfo) => {
  test.setTimeout(90_000);
  await page.addInitScript(() => {
    const state = { text: '', blocked: false };
    Object.defineProperty(window, '__correctionClipboard', { value: state });
    Object.defineProperty(navigator.clipboard, 'writeText', { value: async (text: string) => {
      if (state.blocked) throw new Error('Clipboard blocked');
      state.text = text;
    } });
  });
  await page.goto('/');
  await expect(page.getByRole('navigation', { name: 'Main navigation' }).getByRole('link', { name: 'Tools', exact: true })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Start with a calculation. Keep the circuit.' })).toHaveCount(0);
  await expect(page.getByRole('navigation', { name: 'Footer navigation' }).getByRole('link', { name: 'Calculators' })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('landing-desktop.png'), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: testInfo.outputPath('landing-mobile.png'), fullPage: true });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  const original = await page.locator('.view-lines').innerText();
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
  const request = 'Design a loaded divider from 12 V with OUT between 2.95 V and 3.05 V, RL = 10kOhm.';
  await dialog.getByLabel('Your circuit requirements').fill(request);
  await dialog.getByRole('button', { name: 'Copy task for AI', exact: true }).click();
  const copied = async () => JSON.parse(await page.evaluate(() =>
    (window as unknown as { __correctionClipboard: { text: string } }).__correctionClipboard.text));
  await expect(dialog.getByRole('status')).toContainText('Task copied');
  const task = await copied();
  expect(task.requirements).toBe(request);
  const valid = task.language.example_source as string;
  const reply = (source: string) => JSON.stringify({ ...task.expected_response, proposed_source: source, summary: 'Nominal divider under the original fixed load and supply.' });
  const invalid = reply(valid.replace('net OUT', 'et OUT'));
  await dialog.getByLabel('Agent reply JSON').fill(invalid);
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  const recovery = dialog.getByRole('alert', { name: 'Correct the agent reply' });
  await expect(recovery).toContainText('Line 3');
  await expect(recovery).toContainText('et OUT');
  await expect(dialog.getByRole('button', { name: 'Apply to editor', exact: true })).toBeDisabled();
  await recovery.getByRole('button', { name: 'Copy correction task', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Correction task copied');
  const correction = await copied();
  expect(correction.requirements).toBe(request);
  expect(correction.circuit).toEqual(task.circuit);
  expect(correction.expected_response.base_source_sha256).toBe(task.expected_response.base_source_sha256);
  expect(correction.correction.rejected_reply).toBe(invalid);
  expect(correction.correction.diagnostics[0]).toMatchObject({ code: 'KES-P001', stage: 'parse', line: 3 });
  expect(correction.correction.assertions).toBeNull();
  await recovery.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('correction-desktop.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  await recovery.scrollIntoViewIfNeeded();
  expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
  await page.screenshot({ path: testInfo.outputPath('correction-mobile.png') });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.evaluate(() => { (window as unknown as { __correctionClipboard: { blocked: boolean } }).__correctionClipboard.blocked = true; });
  await recovery.getByRole('button', { name: 'Copy correction task', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Clipboard access was blocked');
  await recovery.getByText('Copy correction text manually', { exact: true }).click();
  await expect(recovery.getByLabel('Correction task JSON')).toContainText('KES-P001');
  const download = page.waitForEvent('download');
  await recovery.getByRole('button', { name: 'Download correction file', exact: true }).click();
  const file = await download;
  const downloaded = JSON.parse(readFileSync((await file.path())!, 'utf8'));
  expect(downloaded.correction.rejected_reply).toBe(invalid);
  await expect(page.locator('.view-lines')).toHaveText(original, { useInnerText: true });

  // Closing abandons a late clipboard completion rather than repopulating a reopened session.
  await page.evaluate(() => {
    Object.defineProperty(navigator.clipboard, 'writeText', { value: () => new Promise<void>(resolve => {
      (window as unknown as { __finishCorrectionCopy(): void }).__finishCorrectionCopy = resolve;
    }) });
  });
  await recovery.getByRole('button', { name: 'Copy correction task', exact: true }).click();
  await expect(recovery.getByRole('button', { name: 'Preparing feedback…', exact: true })).toBeDisabled();
  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  await page.evaluate(() => (window as unknown as { __finishCorrectionCopy(): void }).__finishCorrectionCopy());
  await expect(dialog.getByLabel('Correction task JSON')).toHaveCount(0);
  await expect(dialog.getByRole('status')).not.toContainText('Correction task copied');

  await dialog.getByLabel('Agent reply JSON').fill(reply(valid.replace('resistor R1 15k', 'resistor R1 20k')));
  await expect(recovery).toBeHidden();
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toContainText('1/2 passed', { timeout: 30_000 });
  await recovery.getByRole('button', { name: 'Download correction file', exact: true }).click();
  const second = await dialog.getByLabel('Correction task JSON').inputValue();
  const failedFeedback = JSON.parse(second);
  expect(failedFeedback.requirements).toBe(request);
  expect(failedFeedback.correction.assertions.assertions[0]).toMatchObject({ status: 'FAIL', threshold: 2.95 });
  expect(failedFeedback.correction.assertions.assertions[0].actual).toBeCloseTo(2.4, 5);
  await dialog.getByLabel('Agent reply JSON').fill(reply(valid));
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toContainText('2/2 passed', { timeout: 30_000 });
  await expect(recovery).toBeHidden();
  await dialog.getByRole('button', { name: 'Apply tested proposal', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toContainText('resistor R1 15k');
});
