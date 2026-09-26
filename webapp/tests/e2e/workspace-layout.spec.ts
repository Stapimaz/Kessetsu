import { expect, test } from '@playwright/test';

test('resizes, minimizes, maximizes and persists panels without losing circuit state', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
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
  await page.getByRole('button', { name: 'Minimize source panel' }).click();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Restore minimized source panel' })).toBeVisible();
  await page.getByRole('button', { name: 'Restore minimized source panel' }).click();

  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded', { timeout: 30_000 });
  const circuit = await page.locator('.view-lines').innerText();

  for (const panel of ['source', 'schematic', 'simulation']) {
    await page.getByRole('button', { name: `Minimize ${panel} panel` }).click();
    await expect(page.getByRole('button', { name: `Restore minimized ${panel} panel` })).toBeVisible();
    await page.getByRole('button', { name: `Restore minimized ${panel} panel` }).click();
  }
  await expect(page.locator('.view-lines')).toHaveText(circuit, { useInnerText: true });
  await expect(page.getByRole('table', { name: 'Engineering requirements' })).toBeVisible();
  await expect(page.locator('.assertion-pass')).toHaveCount(5);

  await page.getByLabel('Canonical schematic').locator('.header-title').dblclick();
  await expect(page.locator('.resizable-workspace')).toHaveAttribute('data-maximized-panel', 'schematic');
  await expect(page.getByLabel('Canonical schematic')).toBeVisible();
  await expect(page.getByLabel('Kessetsu source editor')).toBeHidden();
  await page.getByRole('button', { name: 'Restore schematic panel from full workspace' }).click();
  await expect(page.locator('.resizable-workspace')).not.toHaveAttribute('data-maximized-panel');
  await expect(page.getByLabel('Kessetsu source editor')).toBeVisible();
  await page.getByRole('button', { name: 'Maximize simulation panel' }).click();
  await page.keyboard.press('Escape');
  await expect(page.locator('.resizable-workspace')).not.toHaveAttribute('data-maximized-panel');

  const vertical = page.getByRole('separator', { name: 'Resize schematic and results' });
  await vertical.focus();
  await page.keyboard.press('ArrowUp');
  await expect(vertical).toHaveAttribute('aria-valuenow', '53');
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: 'Power Amplifier', exact: true })).toBeVisible();
  await page.screenshot({ path: 'test-results/workspace-file-menu.png', fullPage: true });
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Reset panel layout' }).click();
  await expect(horizontal).toHaveAttribute('aria-valuenow', '40');
  await expect(vertical).toHaveAttribute('aria-valuenow', '55');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.screenshot({ path: 'test-results/workspace-revision-desktop.png', fullPage: true });

  await page.getByRole('button', { name: 'Minimize schematic panel' }).click();
  await page.screenshot({ path: 'test-results/workspace-requirements.png', fullPage: true });
  await page.getByRole('button', { name: 'Restore minimized schematic panel' }).click();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.screenshot({ path: 'test-results/workspace-export-dialog.png' });
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Export', exact: true })).toBeFocused();
});

test('allows every panel to minimize and keeps menus and restore dock usable on mobile', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  for (const panel of ['source', 'schematic', 'simulation']) {
    await page.getByRole('button', { name: `Minimize ${panel} panel` }).click();
  }
  await expect(page.getByText('All panels are minimized.')).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Minimized panels' }).getByRole('button')).toHaveCount(3);
  await page.getByRole('button', { name: 'Restore minimized source panel' }).click();
  await expect(page.getByLabel('Kessetsu source editor')).toBeVisible();

  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: 'Power Amplifier', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  const bounds = (await page.getByRole('dialog').boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
  await page.screenshot({ path: 'test-results/workspace-revision-mobile.png' });
});
