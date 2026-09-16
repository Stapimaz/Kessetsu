import { expect, test } from '@playwright/test';

test('guides installation, copies commands, and opens direct installation URLs', async ({ page, context, request }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/');
  await page.getByRole('link', { name: 'Install the CLI' }).click();
  await expect(page).toHaveURL(/\/install\/$/);
  await expect(page.getByRole('heading', { name: 'A few steps. Your first circuit.' })).toBeVisible();
  await page.getByRole('button', { name: 'Windows', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Windows', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Copy installation command', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('irm https://kessetsu.com/install.ps1 | iex');
  await expect(page.getByText('Ngspice, the simulator, is included')).toBeVisible();
  await page.getByRole('button', { name: 'macOS', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Simulation needs Ngspice' })).toBeVisible();
  await expect(page.getByText('brew install ngspice', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Linux', exact: true }).click();
  await expect(page.getByText('sudo apt-get update && sudo apt-get install ngspice', { exact: true })).toBeVisible();
  await page.screenshot({ path: 'test-results/install-desktop.png', fullPage: true });
  await page.reload();
  await expect(page.getByRole('heading', { name: 'A few steps. Your first circuit.' })).toBeVisible();
  for (const script of ['install.ps1', 'install.sh']) {
    const response = await request.get(`/${script}`);
    expect(response.ok()).toBeTruthy();
    expect(await response.text()).toContain('Kessetsu');
    expect(await response.text()).not.toContain('<!doctype html>');
  }
  await page.getByRole('navigation', { name: 'Installation navigation' }).getByRole('link', { name: 'Web Hub', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Kessetsu source editor' })).toBeVisible();
});

test('keeps mobile installation readable and offers manual copy on clipboard failure', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/install/');
  await page.getByRole('button', { name: 'Windows', exact: true }).click();
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { value: { writeText: () => Promise.reject(new Error('denied')) }, configurable: true }));
  await page.getByRole('button', { name: 'Copy installation command', exact: true }).click();
  await expect(page.getByText('Select the command above and copy it manually.')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/install-mobile.png', fullPage: true });
});
