import { expect, test } from '@playwright/test';

async function openFileMenu(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: 'File', exact: true }).click();
}

test('uses clear browser-local Save and explicit download when native file handles are unavailable', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, 'showOpenFilePicker', { configurable: true, value: undefined });
    Object.defineProperty(window, 'showSaveFilePicker', { configurable: true, value: undefined });
  });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });

  await openFileMenu(page);
  await page.getByRole('menuitem', { name: 'Rename…' }).click();
  const rename = page.getByRole('dialog', { name: 'Rename circuit' });
  await rename.getByLabel('Circuit name').fill('Precision / Filter');
  await rename.getByRole('button', { name: 'Rename', exact: true }).click();
  await expect(page.locator('.document-title')).toContainText('Precision / Filter');

  await openFileMenu(page);
  await expect(page.getByText('Saved projects stay in this browser.')).toBeVisible();
  await page.getByRole('menuitem', { name: 'Save in browser', exact: true }).click();
  await expect(page.getByText('Saved in this browser', { exact: true })).toBeVisible();
  await expect(page.locator('.document-title')).not.toHaveAttribute('aria-label', /unsaved changes/);
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('kessetsu.workspace.draft.v1') ?? '{}').dirty)).toBe(false);

  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.insertText('\n// saved edit');
  await expect(page.locator('.document-title')).toHaveAttribute('aria-label', /unsaved changes/);
  await page.keyboard.press('ControlOrMeta+s');
  await expect(page.getByText('Saved in this browser', { exact: true })).toBeVisible();
  await expect(page.locator('.document-title')).not.toHaveAttribute('aria-label', /unsaved changes/);

  await openFileMenu(page);
  const sourceDownloadPromise = page.waitForEvent('download');
  await page.getByRole('menuitem', { name: 'Download .kess', exact: true }).click();
  const sourceDownload = await sourceDownloadPromise;
  expect(sourceDownload.suggestedFilename()).toBe('precision-filter.kess');
  const sourceStream = await sourceDownload.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of sourceStream) chunks.push(Buffer.from(chunk));
  expect(Buffer.concat(chunks).toString('utf8')).toContain('// saved edit');
  await expect(page.getByText('Downloaded precision-filter.kess', { exact: true })).toBeVisible();

  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.insertText('\n// recovered edit');
  await expect(page.locator('.document-title')).toHaveAttribute('aria-label', /unsaved changes/);
  await page.waitForFunction(() => localStorage.getItem('kessetsu.workspace.draft.v1')?.includes('recovered edit'));
  await page.reload();
  await expect(page.getByText('Unsaved browser draft restored.')).toBeVisible();
  await expect(page.locator('.view-lines')).toContainText('recovered edit');

  page.once('dialog', (dialog) => void dialog.accept());
  await openFileMenu(page);
  const chooserPromise = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'Open .kess…' }).click();
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: 'Loaded Circuit.kess',
    mimeType: 'text/plain',
    buffer: Buffer.from('net GND\nsource V1 1V\nconnect V1.minus to GND\n'),
  });
  await expect(page.locator('.document-title')).toHaveText('Loaded Circuit');

  await openFileMenu(page);
  await page.getByRole('menuitem', { name: 'New circuit' }).click();
  await expect(page.locator('.document-title')).toHaveText('Untitled circuit');
  await expect(page.locator('.view-lines')).toContainText('New Kessetsu circuit');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('svg g.component[data-component="V1"]')).toBeVisible();
  await expect(page.locator('svg g.component[data-component="R1"]')).toBeVisible();
});

test('imports the supported SPICE subset as an unsaved editable circuit', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });

  await openFileMenu(page);
  const chooserPromise = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'Import SPICE netlist...' }).click();
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: 'Imported Filter.cir',
    mimeType: 'text/plain',
    buffer: Buffer.from('V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 159.154943n\n.ac dec 40 10 100k\n.end\n'),
  });

  await expect(page.locator('.document-title')).toHaveText('Imported Filter');
  await expect(page.locator('.document-title')).toHaveAttribute('aria-label', /unsaved changes/);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await expect(page.locator('.view-lines')).toContainText('Original source: sha256:');
  await expect(page.locator('svg g.component[data-component="R1"]')).toBeVisible();

  page.once('dialog', (dialog) => void dialog.accept());
  await openFileMenu(page);
  const invalidChooserPromise = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'Import SPICE netlist...' }).click();
  const invalidChooser = await invalidChooserPromise;
  await invalidChooser.setFiles({
    name: 'unsafe.cir',
    mimeType: 'text/plain',
    buffer: Buffer.from('.include ../outside.lib\n.end\n'),
  });
  await expect(page.locator('.global-error')).toContainText('KES-N003 line 1');
  await expect(page.locator('.document-title')).toHaveText('Imported Filter');
});

test('Save retains a native file handle while Save As selects a new destination', async ({ page, browserName }) => {
  test.skip(browserName !== 'chromium', 'Native File System Access save is a Chromium capability.');
  await page.addInitScript(() => {
    const state = { pickerCalls: 0, writes: [] as string[], suggestedNames: [] as string[] };
    Object.defineProperty(window, '__kessetsuFsTest', { configurable: true, value: state });
    Object.defineProperty(window, 'showSaveFilePicker', {
      configurable: true,
      value: async (options: { suggestedName: string }) => {
        state.pickerCalls += 1;
        state.suggestedNames.push(options.suggestedName);
        const handleId = state.pickerCalls;
        return {
          name: options.suggestedName,
          getFile: async () => new File([], options.suggestedName, { type: 'text/plain' }),
          createWritable: async () => ({
            write: async (data: Blob) => { state.writes.push(`${handleId}:${await data.text()}`); },
            close: async () => undefined,
          }),
        };
      },
    });
  });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await openFileMenu(page);
  await expect(page.getByRole('menuitem', { name: 'Save', exact: true })).toBeVisible();
  await expect(page.getByRole('menuitem', { name: 'Save As', exact: true })).toBeVisible();
  await expect(page.getByText('Saved projects stay in this browser.')).toHaveCount(0);
  await page.getByRole('button', { name: 'File', exact: true }).click();

  await openFileMenu(page);
  await page.getByRole('menuitem', { name: 'Rename…' }).click();
  const rename = page.getByRole('dialog', { name: 'Rename circuit' });
  await rename.getByLabel('Circuit name').fill('Native Filter');
  await rename.getByRole('button', { name: 'Rename', exact: true }).click();

  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.insertText('\n// native save one');
  await page.keyboard.press('ControlOrMeta+s');
  await expect(page.locator('.document-title')).not.toHaveAttribute('aria-label', /unsaved changes/);
  await expect(page.getByText('Saved to native-filter.kess', { exact: true })).toBeVisible();

  await page.keyboard.insertText('\n// native save two');
  await page.keyboard.press('ControlOrMeta+s');
  await expect(page.locator('.document-title')).not.toHaveAttribute('aria-label', /unsaved changes/);

  await openFileMenu(page);
  await page.getByRole('menuitem', { name: 'Save As', exact: true }).click();
  await expect(page.locator('.document-title')).not.toHaveAttribute('aria-label', /unsaved changes/);

  const nativeState = await page.evaluate(() => (
    window as unknown as { __kessetsuFsTest: { pickerCalls: number; writes: string[]; suggestedNames: string[] } }
  ).__kessetsuFsTest);
  expect(nativeState.pickerCalls).toBe(2);
  expect(nativeState.suggestedNames).toEqual(['native-filter.kess', 'native-filter.kess']);
  expect(nativeState.writes).toHaveLength(3);
  expect(nativeState.writes[0]).toContain('// native save one');
  expect(nativeState.writes[1]).toContain('// native save two');
  expect(nativeState.writes[2]).toContain('// native save two');
});
