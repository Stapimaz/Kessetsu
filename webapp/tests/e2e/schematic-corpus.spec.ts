import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const corpus = [
  ['minimal', '../../../core/tests/fixtures/valid/minimal.nl', 'V1'],
  ['rc-filter', '../../../core/tests/fixtures/benchmarks/rc_filter.nl', 'C1'],
  ['wheatstone', '../../../examples/wheatstone.nl', 'Rx'],
  ['gain-stage', '../../../core/tests/fixtures/benchmarks/gain_stage.nl', 'RF'],
  ['high-fanout', '../../../core/tests/fixtures/schematic/high_fanout.nl', 'R8'],
  ['power-amplifier', '../../../core/tests/fixtures/benchmarks/power_amplifier.nl', 'QP'],
  ['inverting-amplifier', '../../../core/tests/fixtures/schematic/inverting_amplifier.nl', 'RF'],
  ['differential-pair', '../../../core/tests/fixtures/schematic/differential_pair.nl', 'Q2'],
  ['mosfet-common-source', '../../../core/tests/fixtures/schematic/mosfet_common_source.nl', 'M1'],
  ['rlc-ladder', '../../../core/tests/fixtures/schematic/rlc_ladder.nl', 'L1'],
  ['diode-clamp', '../../../core/tests/fixtures/schematic/diode_clamp.nl', 'DHI'],
  ['bjt-common-emitter', '../../../core/tests/fixtures/schematic/bjt_common_emitter.nl', 'Q1'],
  ['summing-amplifier', '../../../core/tests/fixtures/schematic/summing_amplifier.nl', 'U1'],
] as const;

test('renders the canonical schematic corpus with verified quality', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.monaco-editor')).toBeVisible();
  const surface = page.getByLabel('Canonical schematic').locator('.schematic-surface');
  const gridButton = page.getByRole('button', { name: 'Toggle schematic grid' });
  await expect(surface).toHaveClass(/has-grid/);
  await expect(gridButton).toHaveAttribute('aria-pressed', 'true');
  await gridButton.click();
  await expect(surface).not.toHaveClass(/has-grid/);
  await expect(gridButton).toHaveAttribute('aria-pressed', 'false');
  await gridButton.click();
  await expect(surface).toHaveClass(/has-grid/);

  for (const [name, relativePath, component] of corpus) {
    const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8');
    await page.locator('.monaco-editor').click();
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.insertText(source);

    const schematic = page.getByTestId('canonical-schematic');
    await expect(schematic).toHaveAttribute('data-quality', 'pass');
    await expect(schematic.locator(`[data-component="${component}"]`)).toBeVisible();
    await expect(schematic.locator('svg[data-schema="netlang.schematic.v1"]')).toBeVisible();

    if (process.env.NETLANG_CAPTURE_VISUALS) {
      await schematic.screenshot({
        path: join('test-results', 'schematic-corpus', `${name}.png`),
        animations: 'disabled',
      });
    }
  }
});
