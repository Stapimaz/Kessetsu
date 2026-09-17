import { expect, test } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { encodeShareFragment } from '../../src/share';

test('shared input contract materializes native/WASM settings for normal Web simulation and sharing', async ({ page }) => {
  test.setTimeout(120_000);
  const source = readFileSync(new URL('../../../core/tests/fixtures/parameters/assertions.kess', import.meta.url), 'utf8');
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const result = spawnSync(binary, ['test', '-', '--param', 'amplitude=2V', '--format', 'json', '--include', 'effective-source,spice,ir'], { input: source, encoding: 'utf8', timeout: 60_000 });
  expect(result.status).toBe(4); // An unchanged acceptance ceiling must still fail.
  expect(result.stderr).toBe('');
  const native = JSON.parse(result.stdout);
  // Exercise the actual public WASM adapter without adding test hooks to the product UI.
  await page.route('**/parameter-input-core.js', route => route.fulfill({
    contentType: 'text/javascript', body: readFileSync(resolve('../core/pkg/kessetsu_core.js')),
  }));
  await page.route('**/kessetsu_core_bg.wasm', route => route.fulfill({
    contentType: 'application/wasm', body: readFileSync(resolve('../core/pkg/kessetsu_core_bg.wasm')),
  }));
  await page.goto('/');
  const wasm = await page.evaluate(async (code) => {
    const url = '/parameter-input-core.js';
    const core = await import(/* @vite-ignore */ url);
    await core.default();
    const report = core.compile_kessetsu_with_inputs(code, {
      schema_version: 'kessetsu.inputs.v1', parameters: [{ name: 'amplitude', value: '2V' }],
    });
    const invalid = core.compile_kessetsu_with_inputs(code, {
      schema_version: 'kessetsu.inputs.v1', parameters: [{ name: 'amplitude', value: '1A' }],
    });
    return { source: report.effective_source, spice: report.spice_netlist, parameters: report.ir.parameter_manifest, invalid: invalid.diagnostics, invalidSpice: invalid.spice_netlist };
  }, source);
  expect(wasm.source).toBe(native.debug.effective_source);
  expect(wasm.spice).toBe(native.debug.spice_netlist);
  expect(wasm.parameters).toEqual(native.debug.ir.parameter_manifest);
  expect(wasm.invalid[0].code).toBe('KES-C022');
  expect(wasm.invalidSpice).toBeNull();
  const fragment = await encodeShareFragment(wasm.source, 'kessetsu.compile.v5', null, 'Effective parameter values');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  await expect(summary.locator('.assertion-fail')).toHaveCount(1);
  for (const assertion of native.assertions.assertions) {
    const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
    expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
  }
});
