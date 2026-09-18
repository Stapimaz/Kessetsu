import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { encodeShareFragment } from '../../src/share';

test('loaded amplifier AC gain and both cutoff edges match native measurements', async ({ page }) => {
  const source = readFileSync(new URL('../../../core/tests/fixtures/benchmarks/ac_coupled_amplifier.kess', import.meta.url), 'utf8');
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const native = JSON.parse(execFileSync(binary, ['test', '-', '--format', 'json'], {
    input: source, encoding: 'utf8', timeout: 60_000,
  }));
  expect(native.domain_versions.measurement).toBe('kessetsu.measurement.v3');
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'AC-coupled amplifier');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(6);
  await expect(summary.locator('.assertion-fail, .assertion-error')).toHaveCount(0);
  for (const assertion of native.assertions.assertions) {
    expect(assertion.status).toBe('PASS');
    const row = summary.locator(`[data-assertion-code="${assertion.code}"]`);
    const actual = Number(await row.getAttribute('data-actual'));
    expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
  }
});
