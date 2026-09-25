import { expect, test } from '@playwright/test';

test('navigates reusable blocks, components, pins and complete nets', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Examples', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Reusable Filters', exact: true }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();

  await page.getByRole('button', { name: 'Find and inspect schematic' }).click();
  const inspector = page.getByRole('complementary', { name: 'Schematic inspector' });
  await expect(inspector).toBeVisible();
  await expect(inspector).toContainText('FAST');
  await expect(inspector).toContainText('SLOW');

  await inspector.getByRole('button', { name: /FAST.*2 components/ }).click();
  await expect(page.getByTestId('canonical-schematic').locator('g.component[data-component="FAST_R1"]')).toHaveClass(/is-group-active/);

  const search = inspector.getByPlaceholder('Find component or net');
  await search.fill('SLOW_R1');
  await inspector.getByRole('button', { name: /SLOW_R1/ }).click();
  await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-selected-component', 'SLOW_R1');
  await expect(inspector).toContainText('Pins');

  const firstNet = page.getByTestId('canonical-schematic').locator('.wire[data-net]').first();
  const netId = await firstNet.getAttribute('data-net');
  expect(netId).not.toBeNull();
  await firstNet.click({ force: true });
  await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-selected-net', netId!);
  await expect(firstNet).toHaveClass(/is-net-active/);
  await expect(inspector).toContainText('Connected pins');
});
