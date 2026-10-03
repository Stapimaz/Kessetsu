import { expect, test } from '@playwright/test';

test('keyboard menus and a complete study preserve the newcomer circuit', async ({ page }, testInfo) => {
  test.setTimeout(60_000);
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const original = await page.locator('.view-lines').innerText();
  const file = page.getByRole('button', { name: 'File', exact: true });
  await file.focus(); await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'New circuit', exact: true })).toBeFocused();
  await page.keyboard.press('End'); await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('menu', { name: 'Examples menu' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menuitem', { name: 'Examples', exact: true })).toBeFocused();
  await page.keyboard.press('Escape'); await expect(file).toBeFocused();
  const analyze = page.getByRole('button', { name: 'Analyze', exact: true });
  await analyze.focus(); await page.keyboard.press('Enter'); await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('dialog', { name: 'Parameter study', exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Close parameter study' }).focus();
  await page.keyboard.press('Shift+Tab');
  expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBeTruthy();
  await page.keyboard.press('Tab');
  expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBeTruthy();
  const initialState = await page.getByTestId('simulation-summary').getAttribute('data-state');
  await page.keyboard.press('Control+Enter');
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', initialState!);
  await dialog.getByRole('button', { name: 'Try loaded-filter study' }).click();
  await expect(dialog.getByLabel('Sweep values 1')).toHaveValue('820Ohm, 1kOhm, 1.2kOhm');
  await dialog.evaluate(element => { element.scrollTop = 0; });
  await expect(dialog.getByRole('heading', { name: 'Parameter study', exact: true })).toBeInViewport();
  await page.screenshot({ path: testInfo.outputPath('study-configuration.png') });
  await dialog.getByRole('button', { name: 'Run study', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Study complete', { timeout: 45_000 });
  await expect(dialog.getByTestId('study-summary')).toContainText('6 total · 3 passed');
  await expect(dialog.getByTestId('study-summary')).toBeInViewport();
  await expect(dialog.getByRole('heading', { name: 'Parameter study', exact: true })).toBeInViewport();
  await expect(dialog.locator('.study-case-failed')).toHaveCount(3);
  await page.screenshot({ path: testInfo.outputPath('study-first-run.png') });
  await dialog.getByRole('button', { name: 'Configure', exact: true }).click();
  page.once('dialog', confirmation => confirmation.dismiss());
  await dialog.getByRole('button', { name: 'Try loaded-filter study' }).click();
  await dialog.getByRole('button', { name: 'Results', exact: true }).click();
  await expect(dialog.getByTestId('study-summary')).toContainText('6 total · 3 passed');
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden(); await expect(analyze).toBeFocused();
  await expect(page.locator('.view-lines')).toHaveText(original, { useInnerText: true });
});

test('failed page download recovers on reload, and research failure leaves the editor usable', async ({ page }) => {
  await page.route(/\/WorkspaceApp-[^/]+\.js$/, route => route.abort());
  await page.goto('/#editor');
  await expect(page.getByRole('heading', { name: 'Could not open the circuit editor' })).toBeVisible();
  await page.unroute(/\/WorkspaceApp-[^/]+\.js$/);
  await page.getByRole('button', { name: 'Reload page' }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  const original = await page.locator('.view-lines').innerText();
  await page.route(/\/ResearchDataDialog-[^/]+\.js$/, route => route.abort());
  await page.getByRole('button', { name: 'Analyze', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Compare research data…', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Could not open research data' })).toBeVisible();
  await page.getByRole('button', { name: 'Return to editor' }).click();
  await expect(page.getByRole('heading', { name: 'Could not open research data' })).toBeHidden();
  await expect(page.locator('.view-lines')).toHaveText(original, { useInnerText: true });
  await expect(page.getByRole('button', { name: 'Analyze', exact: true })).toBeFocused();
});

test('cold informational pages avoid engineering modules; first editor and simulation timing', async ({ page, browser, baseURL, request }, testInfo) => {
  const entry = await request.get('/llms.txt');
  expect(entry.ok()).toBeTruthy();
  const text = await entry.text();
  expect(text).toContain('kess capabilities --format json');
  expect(text).not.toContain('<!doctype html>');
  const engineering: string[] = [];
  const timings: Record<string, number> = {};
  for (const [url, heading] of [['/', 'Circuit engineering you can execute.'], ['/install/', 'A few steps. Your first circuit.'], ['/docs/guides/tutorial/', 'Tutorial: From Source to Verified Circuit']]) {
    const context = await browser.newContext({ baseURL });
    try {
      const informational = await context.newPage();
      informational.on('request', request => { if (/WorkspaceApp-|ResearchDataDialog-|\.wasm(?:\?|$)|editor\.worker|eecircuit/.test(request.url())) engineering.push(request.url()); });
      await informational.goto(url); await expect(informational.getByRole('heading', { name: heading, exact: true })).toBeVisible();
      timings[url] = await informational.evaluate(() => performance.now());
      await informational.waitForLoadState('networkidle');
    } finally { await context.close(); }
  }
  expect(engineering).toEqual([]);
  await page.goto('/#editor'); await expect(page.getByTestId('compile-success')).toBeVisible();
  timings.editor_ready_ms = await page.evaluate(() => performance.now());
  const start = await page.evaluate(() => performance.now());
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded', { timeout: 25_000 });
  timings.first_simulation_ms = await page.evaluate(() => performance.now()) - start;
  await expect(page.locator('.assertion-pass')).toHaveCount(5);
  await testInfo.attach('local-cold-timings.json', { body: JSON.stringify({ environment: 'Fresh Playwright Chromium context; local production preview over loopback; no CPU/network throttle; includes automation wait overhead; not live-user latency', timings }, null, 2), contentType: 'application/json' });
  console.log('Local cold timings (ms):', JSON.stringify(timings));
});
