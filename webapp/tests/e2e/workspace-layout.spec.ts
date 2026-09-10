import { expect, test } from '@playwright/test';

test('resizes, restores and persists panels without losing the circuit or results', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const source = page.getByLabel('Kessetsu source editor');
  const original = (await source.boundingBox())!;
  const horizontal = page.getByRole('separator', { name: 'Resize source panel' });
  const handle = (await horizontal.boundingBox())!;
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await page.mouse.down();
  await page.mouse.move(handle.x + 100, handle.y + handle.height / 2, { steps: 8 });
  await page.mouse.up();
  expect((await source.boundingBox())!.width).toBeGreaterThan(original.width + 80);
  await horizontal.focus();
  await page.keyboard.press('ArrowLeft');
  const saved = await horizontal.getAttribute('aria-valuenow');
  await page.reload();
  await expect(horizontal).toHaveAttribute('aria-valuenow', saved!);
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded');
  const circuit = await page.locator('.view-lines').innerText();
  for (const panel of ['source', 'schematic', 'results']) {
    await page.getByRole('button', { name: `Minimize ${panel} panel` }).click();
    await page.getByRole('button', { name: `Restore ${panel} panel` }).click();
  }
  await expect(page.locator('.view-lines')).toHaveText(circuit, { useInnerText: true });
  await expect(page.getByRole('table', { name: 'Engineering requirements' })).toBeVisible();
  await expect(page.locator('.assertion-pass')).toHaveCount(5);
  const vertical = page.getByRole('separator', { name: 'Resize schematic and results' });
  await vertical.focus();
  await page.keyboard.press('ArrowUp');
  await expect(vertical).toHaveAttribute('aria-valuenow', '53');
  await page.getByRole('button', { name: 'Reset layout' }).click();
  await expect(horizontal).toHaveAttribute('aria-valuenow', '40');
  await expect(vertical).toHaveAttribute('aria-valuenow', '55');
  await page.screenshot({ path: 'test-results/workspace-revision-desktop.png', fullPage: true });
  await page.getByRole('button', { name: 'Minimize schematic panel' }).click();
  await page.screenshot({ path: 'test-results/workspace-requirements.png', fullPage: true });
  await page.getByRole('button', { name: 'Restore schematic panel' }).click();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.screenshot({ path: 'test-results/workspace-export-dialog.png' });
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Export', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Change theme' }).click();
  await page.screenshot({ path: 'test-results/workspace-revision-light.png', fullPage: true });
});

test('keeps at least one panel open and fits mobile toolbar and export dialog', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Minimize source panel' }).click();
  await page.getByRole('button', { name: 'Minimize schematic panel' }).click();
  await expect(page.getByRole('button', { name: 'Minimize results panel' })).toBeDisabled();
  await page.getByRole('button', { name: 'Reset layout' }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  const bounds = (await page.getByRole('dialog').boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
  await page.screenshot({ path: 'test-results/workspace-revision-mobile.png' });
});
