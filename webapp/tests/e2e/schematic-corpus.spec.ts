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
] as const;

test('renders the canonical schematic corpus with verified quality', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.monaco-editor')).toBeVisible();

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
