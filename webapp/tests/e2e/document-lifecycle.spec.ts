import { expect, test } from '@playwright/test';

async function openFileMenu(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: 'File', exact: true }).click();
}

test('names, saves, restores, opens and creates local Kessetsu documents', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();

  await openFileMenu(page);
  await page.getByRole('menuitem', { name: 'Rename…' }).click();
  const rename = page.getByRole('dialog', { name: 'Rename circuit' });
  await rename.getByLabel('Circuit name').fill('Precision / Filter');
  await rename.getByRole('button', { name: 'Rename', exact: true }).click();
  await expect(page.locator('.document-title')).toContainText('Precision / Filter');

  await openFileMenu(page);
  const sourceDownloadPromise = page.waitForEvent('download');
  await page.getByRole('menuitem', { name: 'Save source' }).click();
  const sourceDownload = await sourceDownloadPromise;
  expect(sourceDownload.suggestedFilename()).toBe('precision-filter.kess');
  const sourceStream = await sourceDownload.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of sourceStream) chunks.push(Buffer.from(chunk));
  expect(Buffer.concat(chunks).toString('utf8')).toContain('Canonical first-order RC low-pass');

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
});
