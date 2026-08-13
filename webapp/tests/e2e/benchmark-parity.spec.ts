import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const cases = [
  { option: 'gain', fixture: 'gain_stage.nl', assertions: 7, models: ['NLANG_OPAMP_V1'] },
  { option: 'power', fixture: 'power_amplifier.nl', assertions: 12, models: ['NLANG_POWER_NPN_V1', 'NLANG_POWER_PNP_V1', 'NLANG_OPAMP_V1'] },
] as const;

test('keeps gain-stage and power-amplifier native/browser decisions, SPICE and models in parity', async ({ page }) => {
  test.setTimeout(240_000);
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'netlang.exe' : 'netlang');
  await page.goto('/');

  for (const benchmark of cases) {
    const fixturePath = resolve(`../core/tests/fixtures/benchmarks/${benchmark.fixture}`);
    await page.getByLabel('Örnek devre').selectOption(benchmark.option);
    await expect(page.getByTestId('compile-success')).toBeVisible();

    const outputPath = resolve(`test-results/native-${benchmark.option}.spice`);
    const native = JSON.parse(execFileSync(binary, [
      'test', fixturePath, '--output', outputPath, '--force', '--format', 'json', '--include', 'models',
    ], { encoding: 'utf8', timeout: 60_000 }));
    const browserSpice = await page.locator('.spice-details pre').textContent();
    expect(browserSpice?.replace(/\r\n/g, '\n')).toBe(readFileSync(outputPath, 'utf8').replace(/\r\n/g, '\n'));

    const browserManifest = JSON.parse(await page.getByTestId('model-manifest').getAttribute('data-manifest') ?? '{}');
    expect(browserManifest).toEqual(native.debug.models.manifest);
    for (const model of benchmark.models) {
      await expect(page.getByTestId('model-manifest')).toHaveAttribute('data-manifest', new RegExp(model));
    }

    await page.getByRole('button', { name: 'Run' }).click();
    const summary = page.getByTestId('simulation-summary');
    await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 180_000 });
    await expect(summary.locator('.assertion-pass')).toHaveCount(benchmark.assertions);
    await expect(summary.locator('.assertion-fail, .assertion-error, .assertion-skipped')).toHaveCount(0);
    expect(native.assertions.assertions.map((assertion: { status: string }) => assertion.status))
      .toEqual(Array.from({ length: benchmark.assertions }, () => 'PASS'));
  }
});
