import { expect, test } from '@playwright/test';

test('guides MCP and CLI installation, copies commands, and supports hash deep-links', async ({ page, context, request }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/');
  await page.locator('.landing-hero').getByRole('link', { name: 'Install CLI & MCP' }).click();
  await expect(page).toHaveURL(/\/install\/$/);
  await expect(page.getByRole('heading', { name: 'Install Kessetsu', exact: true })).toBeVisible();

  const modeTabs = page.getByRole('tablist', { name: 'Setup mode' });
  await expect(modeTabs.getByRole('tab', { name: 'MCP (AI Agents)', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(modeTabs.getByRole('tab', { name: 'CLI (Terminal & CI)', exact: true })).toHaveAttribute('aria-selected', 'false');

  const osTabs = page.getByRole('tablist', { name: 'Operating system' });
  await osTabs.getByRole('tab', { name: 'Windows', exact: true }).click();
  await expect(osTabs.getByRole('tab', { name: 'Windows', exact: true })).toHaveAttribute('aria-selected', 'true');
  await page.getByRole('button', { name: 'Copy installation command', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('irm https://kessetsu.com/install.ps1 | iex');
  await expect(page.getByText('Includes Kessetsu CLI, MCP server, and Ngspice 46 — no admin rights needed.')).toBeVisible();

  await osTabs.getByRole('tab', { name: 'macOS', exact: true }).click();
  await expect(page.getByText('brew install ngspice', { exact: true })).toBeVisible();

  await osTabs.getByRole('tab', { name: 'Linux', exact: true }).click();
  await expect(page.getByText('sudo apt install ngspice', { exact: true })).toBeVisible();

  // MCP mode steps
  await expect(page.getByRole('heading', { name: 'Register the MCP server', exact: true })).toBeVisible();
  const clientTabs = page.getByRole('tablist', { name: 'Choose your AI application' });
  await expect(clientTabs).toBeVisible();
  await clientTabs.getByRole('tab', { name: 'Cursor', exact: true }).click();
  await expect(clientTabs.getByRole('tab', { name: 'Cursor', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByText('.cursor/mcp.json', { exact: false })).toBeVisible();
  await page.getByRole('button', { name: 'Copy Cursor MCP config', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain('"command": "kess"');

  await clientTabs.getByRole('tab', { name: 'VS Code', exact: true }).click();
  await expect(page.getByText('.vscode/mcp.json', { exact: false })).toBeVisible();

  await clientTabs.getByRole('tab', { name: 'Claude Code', exact: true }).click();
  await expect(page.getByText('claude mcp add kessetsu -- kess mcp', { exact: true })).toBeVisible();

  await expect(page.getByRole('heading', { name: 'Verify in your AI chat', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Copy verification prompt', exact: true }).click();
  const verifyPrompt = await page.evaluate(() => navigator.clipboard.readText());
  expect(verifyPrompt).toContain('1 kHz RC low-pass filter');
  expect(verifyPrompt).toContain('-3 dB cutoff');

  // Switch to CLI mode
  await modeTabs.getByRole('tab', { name: 'CLI (Terminal & CI)', exact: true }).click();
  await expect(modeTabs.getByRole('tab', { name: 'CLI (Terminal & CI)', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('heading', { name: 'Verify in your terminal', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Register the MCP server', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Copy version command', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('kess --version');

  await page.getByText('Try a 30-second RC low-pass check in the terminal', { exact: true }).click();
  await expect(page.getByText('kess check my-circuit.kess', { exact: true })).toBeVisible();
  await expect(page.getByText('kess test my-circuit.kess', { exact: true })).toBeVisible();

  // Bottom accordions
  await page.getByText('Prefer not to pipe to a shell? Inspect or install manually', { exact: true }).click();
  await expect(page.getByText('view install.sh', { exact: true })).toBeVisible();
  await page.getByText('Update, uninstall, or fix PATH issues', { exact: true }).click();
  await expect(page.getByText('Do not delete the surrounding bin/share folders.', { exact: false })).toBeVisible();

  await page.screenshot({ path: 'test-results/install-desktop.png', fullPage: true });

  // Hash deep-linking
  await page.goto('/install/#cli-verify');
  await expect(modeTabs.getByRole('tab', { name: 'CLI (Terminal & CI)', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('heading', { name: 'Verify in your terminal', exact: true })).toBeVisible();

  await page.goto('/install/#mcp-setup');
  await expect(modeTabs.getByRole('tab', { name: 'MCP (AI Agents)', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('heading', { name: 'Register the MCP server', exact: true })).toBeVisible();

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
  await page.getByRole('tablist', { name: 'Operating system' }).getByRole('tab', { name: 'Windows', exact: true }).click();
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { value: { writeText: () => Promise.reject(new Error('denied')) }, configurable: true }));
  await page.getByRole('button', { name: 'Copy installation command', exact: true }).click();
  await expect(page.getByText('Select the command above and copy it manually.')).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Installation navigation' }).getByRole('link', { name: 'Docs', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.getByRole('tablist', { name: 'Setup mode' }).getByRole('tab', { name: 'CLI (Terminal & CI)', exact: true }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/install-mobile.png', fullPage: true });
});
