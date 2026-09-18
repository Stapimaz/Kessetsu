import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

const productVersion = readFileSync(new URL('../../../VERSION', import.meta.url), 'utf8').trim();

test('opens the landing page, enters Web Hub, initializes WASM and compiles the canonical example', async ({ page }) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.stack ?? error.message));

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Circuit engineering you can execute.' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  const landingWordmark = page.locator('.landing-header').getByTestId('brand-wordmark');
  await expect(landingWordmark).toHaveText('kessetsu');
  await expect(landingWordmark).toHaveCSS('font-size', '22px');
  await expect(landingWordmark.locator('.brand-wordmark-signal')).toBeVisible();
  await expect(landingWordmark.locator('.brand-wordmark-node')).toBeVisible();
  await expect.poll(() => page.evaluate(async () => (await document.fonts.load('650 22px "Kessetsu Wordmark"', 'kessetsu')).length)).toBe(1);
  const nodeAlignmentError = await landingWordmark.evaluate((wordmark) => {
    const text = wordmark.querySelector<HTMLElement>('.brand-wordmark-text');
    const node = wordmark.querySelector<HTMLElement>('.brand-wordmark-node');
    if (!(text?.firstChild instanceof Text) || !node) throw new Error('Wordmark geometry is incomplete');

    const characterCenter = (index: number) => {
      const range = document.createRange();
      range.setStart(text.firstChild!, index);
      range.setEnd(text.firstChild!, index + 1);
      const bounds = range.getBoundingClientRect();
      return bounds.left + bounds.width / 2;
    };
    const previousSCenter = characterCenter(3);
    const followingECenter = characterCenter(4);
    const nodeBounds = node.getBoundingClientRect();
    const nodeCenter = nodeBounds.left + nodeBounds.width / 2;
    return Math.abs(nodeCenter - (previousSCenter + followingECenter) / 2);
  });
  // Chromium's platform font rasterizer can move the measured glyph midpoint by a subpixel.
  expect(nodeAlignmentError).toBeLessThanOrEqual(0.5);
  expect(await page.locator('.landing-hero .eyebrow').evaluate((element) => (element as HTMLElement).innerText))
    .toBe('EXECUTABLE CIRCUIT ENGINEERING');
  await expect(page.getByText('First-release scope:')).toHaveCount(0);
  await expect(page.locator('.preview-generated-schematic [data-component="VIN"].reference')).toBeVisible();
  await expect(page.getByRole('contentinfo')).toContainText(`Kessetsu ${productVersion}`);
  await expect(page.getByRole('navigation', { name: 'Footer navigation' }).getByRole('link')).toHaveCount(4);
  await expect(page.locator('.monaco-editor')).toHaveCount(0);
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await page.screenshot({ path: 'test-results/landing-desktop.png', fullPage: true });
  }
  await page.locator('.landing-hero').getByRole('link', { name: 'Open Web Hub' }).click();
  await expect(page).toHaveURL(/#editor$/);

  await expect(page.getByTestId('compile-success')).toBeVisible();
  const editorWordmark = page.locator('.app-menubar').getByTestId('brand-wordmark');
  await expect(editorWordmark).toHaveText('kessetsu');
  await expect(editorWordmark).toHaveCSS('font-size', '15px');
  const headerActionSizes = await page.locator('.global-actions .header-action-button').evaluateAll((buttons) => (
    buttons.map((button) => {
      const bounds = button.getBoundingClientRect();
      return { width: Math.round(bounds.width), height: Math.round(bounds.height) };
    })
  ));
  expect(headerActionSizes).toEqual([
    { width: 76, height: 30 },
    { width: 76, height: 30 },
  ]);
  await expect(page.getByRole('button', { name: 'Run simulation' })).toBeVisible();
  await expect(page.locator('.monaco-editor')).toBeVisible();
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Circuit details…' }).click();
  await expect(page.getByText('Generated SPICE Netlist', { exact: true })).toBeVisible();
  await expect(page.getByText('Unsupported compile report schema')).toHaveCount(0);
  await expect(page.locator('svg polyline')).not.toHaveCount(0);
  expect(pageErrors).toEqual([]);
});

test('keeps the landing page readable and actionable on mobile', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Circuit engineering you can execute.' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.locator('.landing-header').getByRole('link', { name: /Open Web Hub/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Give your agent requirements, not blind trust.' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'The complete workflow, in your browser.' })).toBeVisible();
  if (process.env.KESSETSU_E2E_SCREENSHOTS) {
    await page.screenshot({ path: 'test-results/landing-mobile.png', fullPage: true });
  }
});
