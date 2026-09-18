import { expect, test } from '@playwright/test';

test('reads the product, installation and public tutorial without JavaScript', async ({ browser, baseURL }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, baseURL });
  try {
    const page = await context.newPage();
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Circuit engineering you can execute.' })).toBeVisible();
    await page.getByRole('link', { name: 'Install the CLI' }).click();
    await expect(page).toHaveTitle('Install Kessetsu CLI — Kessetsu');
    await expect(page.getByText('irm https://kessetsu.com/install.ps1 | iex', { exact: true })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Without JavaScript' })).toBeVisible();
    await page.getByRole('navigation', { name: 'Installation navigation' }).getByRole('link', { name: 'Docs', exact: true }).click();
    await page.locator('article').getByRole('link', { name: 'Tutorial', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Tutorial: From Source to Verified Circuit' })).toBeVisible();
    await expect(page.locator('article')).toContainText('assert cutoff(V(OUT),V(IN)) > 990Hz');
    await page.screenshot({ path: 'test-results/docs-desktop.png', fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: 'test-results/docs-mobile.png', fullPage: true });
    await page.goto('/docs/reference/language/');
    await page.getByRole('link', { name: 'model cookbook', exact: true }).click();
    await expect(page).toHaveURL(/\/docs\/guides\/cookbook\/#choose-and-verify-a-component-model$/);
    await expect(page.locator('#choose-and-verify-a-component-model')).toBeVisible();
    await page.getByRole('navigation', { name: 'Documentation navigation' }).getByRole('link', { name: 'Changelog' }).click();
    await expect(page).toHaveURL(/\/changelog\/$/);
    await expect(page.getByRole('heading', { name: 'Changelog', exact: true })).toBeVisible();
    await expect(page.locator('article')).toContainText('1.2.0');
    await expect(page.getByRole('navigation', { name: 'Documentation topics' })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
    await page.screenshot({ path: 'test-results/changelog-mobile.png', fullPage: true });
  } finally { await context.close(); }
});
