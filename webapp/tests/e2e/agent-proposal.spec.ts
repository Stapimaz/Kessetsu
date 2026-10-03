import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { encodeShareFragment } from '../../src/share';

function replyFor(source: string) {
  return JSON.stringify({
    schema_version: 'kessetsu.agent-proposal.v1',
    base_source_sha256: createHash('sha256').update(source).digest('hex'),
    proposed_source: `${source.trimEnd()}\n// reviewed agent proposal\n`,
    summary: 'Preserve the circuit and its assertions.',
  });
}

test('invalidates completed proposal evidence when local model bindings change while closed', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(new URL('../../../examples/external_comparator.kess', import.meta.url), 'utf8');
  const modelPath = new URL('../../../examples/models/comparator.lib', import.meta.url);
  await page.goto(`/${await encodeShareFragment(source, 'kessetsu.compile.v9', null, 'Local comparator')}`);
  const details = page.getByRole('dialog', { name: 'Circuit details' });
  const openDetails = async () => {
    await page.getByRole('button', { name: 'View', exact: true }).click();
    await page.getByRole('menuitem', { name: /Circuit details/ }).click();
  };
  const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
  const openAgent = async () => {
    await page.getByRole('button', { name: 'Analyze', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  };
  await openDetails();
  await details.getByLabel('Model file for CMP').setInputFiles({
    name: 'comparator.lib', mimeType: 'text/plain', buffer: readFileSync(modelPath),
  });
  await expect(details).toContainText('File selected');
  await details.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await openAgent();
  const reply = replyFor(source);
  await dialog.locator('textarea').nth(1).fill(reply);
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toContainText('4/4 passed', { timeout: 90_000 });
  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
  await openAgent();
  await expect(dialog.getByRole('button', { name: 'Apply tested proposal', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();

  await openDetails();
  await details.getByRole('button', { name: 'Clear files', exact: true }).click();
  await expect(details).toContainText('File required');
  await details.getByRole('button', { name: 'Close circuit details' }).click();
  await openAgent();
  await expect(dialog.getByTestId('agent-verification')).toBeHidden();
  await expect(dialog.getByRole('button', { name: 'Apply to editor', exact: true })).toBeDisabled();
  await expect(dialog.locator('textarea').nth(1)).toHaveValue(reply);
  await expect(dialog).toContainText('Local model files changed');
  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();

  await openDetails();
  await details.getByLabel('Model file for CMP').setInputFiles({
    name: 'comparator.lib', mimeType: 'text/plain', buffer: readFileSync(modelPath),
  });
  await expect(details).toContainText('File selected');
  await details.getByRole('button', { name: 'Close circuit details' }).click();
  await openAgent();
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toContainText('4/4 passed', { timeout: 90_000 });
});

for (const stage of ['file', 'review'] as const) {
  test(`ignores a late ${stage} result after closing and reopening proposal review`, async ({ page }) => {
    const source = readFileSync(new URL('../../../core/tests/fixtures/benchmarks/rc_filter.kess', import.meta.url), 'utf8');
    await page.goto(`/${await encodeShareFragment(source, 'kessetsu.compile.v9', null, 'RC filter')}`);
    await expect(page.getByTestId('compile-success')).toBeVisible();
    const openAgent = async () => {
      await page.getByRole('button', { name: 'Analyze', exact: true }).click();
      await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
    };
    await openAgent();
    const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
    // Delay a real file read/hash deterministically; the production component and Core are unchanged.
    await page.evaluate((stage) => {
      const controls = globalThis as unknown as {
        agentWorkStarted: boolean; releaseAgentWork(): Promise<void>;
      };
      controls.agentWorkStarted = false;
      if (stage === 'file') {
        const original = File.prototype.text;
        File.prototype.text = function () {
          const result = original.call(this);
          controls.agentWorkStarted = true;
          return new Promise<string>((resolve, reject) => {
            controls.releaseAgentWork = async () => {
              File.prototype.text = original;
              await result.then(resolve, reject);
            };
          });
        };
      } else {
        const original = crypto.subtle.digest.bind(crypto.subtle);
        crypto.subtle.digest = (...args) => {
          const result = original(...args);
          controls.agentWorkStarted = true;
          return new Promise<ArrayBuffer>((resolve, reject) => {
            controls.releaseAgentWork = async () => {
              crypto.subtle.digest = original;
              await result.then(resolve, reject);
            };
          });
        };
      }
    }, stage);
    const reply = replyFor(source);
    if (stage === 'file') {
      await dialog.getByLabel('Open agent proposal JSON').setInputFiles({
        name: 'proposal.json', mimeType: 'application/json', buffer: Buffer.from(reply),
      });
    } else {
      await dialog.locator('textarea').nth(1).fill(reply);
      await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
    }
    await expect.poll(() => page.evaluate(() => (globalThis as unknown as { agentWorkStarted: boolean }).agentWorkStarted)).toBe(true);
    await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
    await openAgent();
    await page.evaluate(async () => {
      await (globalThis as unknown as { releaseAgentWork(): Promise<void> }).releaseAgentWork();
      await new Promise(requestAnimationFrame);
    });
    await expect(dialog.getByText('Core compile and connectivity', { exact: true })).toBeHidden();
    await expect(dialog.getByRole('button', { name: 'Apply to editor', exact: true })).toBeDisabled();
    await expect(dialog.locator('textarea').nth(1)).toHaveValue(stage === 'file' ? '' : reply);
    await expect(page.locator('.view-lines')).not.toContainText('reviewed agent proposal');
    // A deliberate new review still works after the abandoned operation settles.
    await dialog.locator('textarea').nth(1).fill(reply);
    await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
    await expect(dialog.getByText('Core compile and connectivity', { exact: true })).toBeVisible();
  });
}

test('closing an active proposal run requires a deliberate new test after reopening', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(new URL('../../../core/tests/fixtures/benchmarks/rc_filter.kess', import.meta.url), 'utf8');
  await page.goto(`/${await encodeShareFragment(source, 'kessetsu.compile.v9', null, 'RC filter')}`);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const openAgent = async () => {
    await page.getByRole('button', { name: 'Analyze', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  };
  await openAgent();
  const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
  await dialog.locator('textarea').nth(1).fill(replyFor(source));
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  // Hold only the initial worker fetch so cancellation is deterministic, not a race with solver speed.
  let releaseWorker!: () => void;
  const blockedWorker = new Promise<void>((resolve) => { releaseWorker = resolve; });
  await page.route('**/simulation.worker-*.js', async (route) => {
    await blockedWorker;
    // Closing terminates the worker; its request may already have been aborted.
    await route.continue().catch(() => {});
  }, { times: 1 });
  const workerRequest = page.waitForRequest(/simulation\.worker-/);
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await workerRequest;
  await expect(dialog.getByRole('button', { name: 'Stop test', exact: true })).toBeVisible();
  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
  releaseWorker();
  await openAgent();
  await expect(dialog).toContainText('Simulation was cancelled. Test again before applying.');
  await expect(dialog.getByTestId('agent-verification')).toBeHidden();
  await expect(dialog.getByRole('button', { name: 'Apply to editor', exact: true })).toBeDisabled();
  await dialog.getByRole('button', { name: 'Test again', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toContainText('5/5 passed', { timeout: 90_000 });
});

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

  const replyText = dialog.locator('textarea').nth(1);
  const serializedProposal = await replyText.inputValue();
  await replyText.fill(`${serializedProposal}\n`);
  await expect(dialog.getByText('Core compile and connectivity')).toBeHidden();
  await expect(dialog.getByRole('button', { name: 'Apply to editor', exact: true })).toBeDisabled();
  await replyText.fill(serializedProposal);
  await dialog.getByRole('button', { name: 'Check returned circuit', exact: true }).click();
  await expect(dialog.getByText('Core compile and connectivity')).toBeVisible();

  await dialog.getByRole('button', { name: 'Close AI agent workflow' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toHaveText(originalSource, { useInnerText: true });

  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  await expect(dialog.getByTestId('agent-verification')).toBeVisible({ timeout: 90_000 });
  await expect(dialog.getByTestId('agent-verification')).toContainText('5/5 passed');
  await dialog.getByRole('button', { name: 'Apply tested proposal', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.view-lines')).toContainText('reviewed agent proposal');

  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: /Undo AI agent change/ }).click();
  await expect(page.locator('.view-lines')).toHaveText(originalSource, { useInnerText: true });
});

test('does not present a simulation without assertions as verified', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Work with an AI agent…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Work with an AI agent', exact: true });
  await dialog.getByPlaceholder(/Design for 2 W RMS/).fill('Return a simulated candidate and preserve every stated requirement.');
  const taskDownload = page.waitForEvent('download');
  await dialog.getByRole('button', { name: 'Download task file', exact: true }).click();
  const taskFile = await taskDownload;
  const task = JSON.parse(readFileSync((await taskFile.path())!, 'utf8')) as {
    circuit: { source: string; source_sha256: string };
  };
  const proposal = {
    schema_version: 'kessetsu.agent-proposal.v1',
    base_source_sha256: task.circuit.source_sha256,
    proposed_source: task.circuit.source.split('\n').filter((line) => !line.trimStart().startsWith('assert ')).join('\n'),
    summary: 'Removed every assertion while retaining the simulation.',
  };
  await dialog.getByLabel('Open agent proposal JSON').setInputFiles({
    name: 'unchecked-proposal.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(proposal)),
  });
  await dialog.getByRole('button', { name: 'Test proposed circuit', exact: true }).click();
  const verification = dialog.getByTestId('agent-verification');
  await expect(verification).toBeVisible({ timeout: 90_000 });
  await expect(verification).toContainText('without requirement checks');
  await expect(verification).toContainText('0/0 passed');
  await dialog.getByRole('button', { name: 'Apply anyway…', exact: true }).click();
  await expect(dialog.getByText('Apply an unverified outcome?')).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Apply unchecked changes', exact: true })).toBeVisible();
});
