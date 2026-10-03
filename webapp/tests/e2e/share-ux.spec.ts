import { expect, test } from '@playwright/test';
import { decodeShareFragment } from '../../src/share';

type ControlledWindow = Window & {
  holdCompression: boolean;
  compressionStarted: number;
  compressionCompleted: number;
  releaseCompression?: () => void;
  holdClipboard: boolean;
  clipboardCalls: number;
  releaseClipboard?: () => void;
};

test('named sharing supports manual copy, name changes and a recipient reopening the exact snapshot', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: async () => { throw new Error('Clipboard denied'); },
    } });
  });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Share circuit' }).click();
  const dialog = page.getByRole('dialog', { name: 'Share circuit' });
  await expect(dialog).toContainText('Later edits do not update');
  await expect(dialog).toContainText('not simulation results or local model/data files');
  await dialog.getByLabel('Circuit name').fill('My filter');
  await dialog.getByRole('button', { name: 'Copy link' }).click();
  await expect(dialog.getByRole('status')).toHaveText('Link created. Select and copy it below.');
  const firstLink = await dialog.getByLabel('Share link').inputValue();
  const first = await decodeShareFragment(new URL(firstLink).hash, 'kessetsu.compile.v9');
  expect(first?.name).toBe('My filter');
  expect(first?.source).toContain('resistor R1 1k');

  await dialog.getByLabel('Circuit name').fill('Bench filter');
  await expect(dialog.getByLabel('Share link')).toHaveCount(0);
  await dialog.getByLabel('Circuit name').press('Enter');
  await expect(dialog.getByRole('status')).toHaveText('Link created. Select and copy it below.');
  const finalLink = await dialog.getByLabel('Share link').inputValue();
  expect(finalLink).not.toBe(firstLink);
  await dialog.getByRole('button', { name: 'Close share' }).click();
  await page.getByRole('button', { name: 'Share circuit' }).click();
  await expect(dialog.getByLabel('Circuit name')).toHaveValue('Bench filter');
  await expect(dialog.getByLabel('Share link')).toHaveCount(0);
  await expect(dialog.getByRole('status')).toBeEmpty();
  await page.goto(finalLink);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('.document-title')).toHaveText('Bench filter');
  await expect(page.locator('.view-lines')).toContainText('resistor R1 1k');
});

test('closing abandons real link compression and late clipboard completion cannot repopulate a new session', async ({ page }) => {
  await page.addInitScript(() => {
    const controlled = window as ControlledWindow;
    controlled.holdCompression = false;
    controlled.compressionStarted = 0;
    controlled.compressionCompleted = 0;
    controlled.holdClipboard = true;
    controlled.clipboardCalls = 0;
    const NativeCompression = CompressionStream;
    // Delay the stream boundary, not the compressed output or Core result.
    Object.defineProperty(window, 'CompressionStream', { value: class {
      readable: ReadableStream;
      writable: WritableStream;
      constructor(format: CompressionFormat) {
        const delayed = new TransformStream({ transform: async (chunk, controller) => {
          controlled.compressionStarted += 1;
          if (controlled.holdCompression) await new Promise<void>((resolve) => { controlled.releaseCompression = resolve; });
          controller.enqueue(chunk);
          controlled.compressionCompleted += 1;
        } });
        this.readable = delayed.readable.pipeThrough(new NativeCompression(format));
        this.writable = delayed.writable;
      }
    } });
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: async () => {
        controlled.clipboardCalls += 1;
        if (controlled.holdClipboard) await new Promise<void>((resolve) => { controlled.releaseClipboard = resolve; });
      },
    } });
  });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const originalName = await page.locator('.document-title').textContent();
  const dialog = page.getByRole('dialog', { name: 'Share circuit' });
  await page.getByRole('button', { name: 'Share circuit' }).click();
  await dialog.getByLabel('Circuit name').fill('Abandoned rename');
  await page.evaluate(() => { (window as ControlledWindow).holdCompression = true; });
  await dialog.getByRole('button', { name: 'Copy link' }).click();
  await expect.poll(() => page.evaluate(() => (window as ControlledWindow).compressionStarted)).toBe(1);
  await expect(dialog.getByLabel('Circuit name')).toBeDisabled();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.getByRole('button', { name: 'Share circuit' }).click();
  await page.evaluate(() => {
    const controlled = window as ControlledWindow;
    controlled.holdCompression = false;
    controlled.releaseCompression?.();
  });
  await expect.poll(() => page.evaluate(() => (window as ControlledWindow).compressionCompleted)).toBe(1);
  await expect(dialog.getByLabel('Circuit name')).toHaveValue(originalName!);
  await expect(dialog.getByRole('status')).toBeEmpty();
  await expect(page).toHaveURL(/#editor$/);
  await expect(page.locator('.document-title')).toHaveText(originalName!);
  await expect(dialog.getByRole('button', { name: 'Copy link' })).toBeEnabled();

  await dialog.getByLabel('Circuit name').fill('Committed snapshot');
  await dialog.getByRole('button', { name: 'Copy link' }).click();
  await expect.poll(() => page.evaluate(() => (window as ControlledWindow).clipboardCalls)).toBe(1);
  await expect(dialog.getByLabel('Share link')).toBeVisible();
  await expect(page.locator('.document-title')).toHaveText('Committed snapshot');
  await dialog.getByRole('button', { name: 'Close share' }).click();
  await page.getByRole('button', { name: 'Share circuit' }).click();
  await page.evaluate(() => {
    const controlled = window as ControlledWindow;
    controlled.holdClipboard = false;
    controlled.releaseClipboard?.();
  });
  await expect(dialog.getByRole('status')).toBeEmpty();
  await expect(dialog.getByLabel('Share link')).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Copy link' }).click();
  await expect(dialog.getByRole('status')).toHaveText('Link copied to clipboard.');
  await expect.poll(() => page.evaluate(() => (window as ControlledWindow).clipboardCalls)).toBe(2);
  if (process.env.KESSETSU_E2E_SCREENSHOTS) await page.screenshot({ path: 'test-results/share-snapshot.png' });
});

test('export presents all nine formats by intent and a real download with its complete notices', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Export circuit' });
  await expect(dialog.locator('[data-export-format]')).toHaveCount(9);
  await expect(dialog.getByRole('region', { name: 'Editable schematics' }).locator('button')).toHaveCount(2);
  await expect(dialog.getByRole('region', { name: 'Images and documents' }).locator('button')).toHaveCount(3);
  await expect(dialog).toContainText('save a .kess file from the File menu');
  const pending = page.waitForEvent('download');
  await dialog.locator('[data-export-format="ltspice"]').click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe('rc-low-pass.asc');
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream) chunks.push(Buffer.from(chunk));
  expect(Buffer.concat(chunks).toString('utf8')).toContain('Version 4');
  await expect(dialog.getByRole('status')).toContainText('Download requested: rc-low-pass.asc');
  await expect(dialog.getByRole('status')).toContainText('LTspice');
  await expect(dialog.getByRole('status')).toHaveCSS('white-space', 'normal');
  if (process.env.KESSETSU_E2E_SCREENSHOTS) await page.screenshot({ path: 'test-results/export-intents-desktop.png' });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await dialog.locator('.export-buttons').first().evaluate((element) =>
    getComputedStyle(element).gridTemplateColumns.split(' ').length)).toBe(1);
  if (process.env.KESSETSU_E2E_SCREENSHOTS) await page.screenshot({ path: 'test-results/export-intents-narrow.png' });
  await dialog.getByRole('button', { name: 'Close export' }).click();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await expect(dialog.getByRole('status')).toHaveCount(0);
});
