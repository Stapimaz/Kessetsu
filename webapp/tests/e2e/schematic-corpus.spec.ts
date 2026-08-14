import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const corpus = [
  ['minimal', '../../../core/tests/fixtures/valid/minimal.kess', 'V1'],
  ['rc-filter', '../../../core/tests/fixtures/benchmarks/rc_filter.kess', 'C1'],
  ['wheatstone', '../../../examples/wheatstone.kess', 'Rx'],
  ['gain-stage', '../../../core/tests/fixtures/benchmarks/gain_stage.kess', 'RF'],
  ['high-fanout', '../../../core/tests/fixtures/schematic/high_fanout.kess', 'R8'],
  ['power-amplifier', '../../../core/tests/fixtures/benchmarks/power_amplifier.kess', 'QP'],
  ['inverting-amplifier', '../../../core/tests/fixtures/schematic/inverting_amplifier.kess', 'RF'],
  ['differential-pair', '../../../core/tests/fixtures/schematic/differential_pair.kess', 'Q2'],
  ['mosfet-common-source', '../../../core/tests/fixtures/schematic/mosfet_common_source.kess', 'M1'],
  ['rlc-ladder', '../../../core/tests/fixtures/schematic/rlc_ladder.kess', 'L1'],
  ['diode-clamp', '../../../core/tests/fixtures/schematic/diode_clamp.kess', 'DHI'],
  ['bjt-common-emitter', '../../../core/tests/fixtures/schematic/bjt_common_emitter.kess', 'Q1'],
  ['summing-amplifier', '../../../core/tests/fixtures/schematic/summing_amplifier.kess', 'U1'],
] as const;

test('renders the canonical schematic corpus with verified quality', async ({ page }) => {
  await page.goto('/#editor');
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
    const componentSymbol = schematic.locator(`g.component[data-component="${component}"]`);
    await expect(componentSymbol).toBeVisible();
    await expect(schematic.locator('svg[data-schema="kessetsu.schematic.v1"]')).toBeVisible();

    if (name === 'gain-stage') {
      const componentTexts = schematic.locator(`text[data-component="${component}"]`);
      await expect(componentTexts).toHaveCount(2);
      await componentSymbol.hover();
      await expect(componentSymbol).toHaveClass(/is-component-active/);
      for (const text of await componentTexts.all()) await expect(text).toHaveClass(/is-component-active/);
      await surface.hover({ position: { x: 4, y: 4 } });
      await expect(componentSymbol).not.toHaveClass(/is-component-active/);
      await componentSymbol.click();
      await expect(schematic).toHaveAttribute('data-selected-component', component);
      await expect(componentSymbol).toHaveClass(/is-component-active/);
      for (const text of await componentTexts.all()) await expect(text).toHaveClass(/is-component-active/);
      if (process.env.KESSETSU_CAPTURE_VISUALS) {
        await schematic.screenshot({
          path: join('test-results', 'schematic-corpus', 'gain-stage-selected.png'),
          animations: 'disabled',
        });
      }
      await page.keyboard.press('Escape');
      await expect(schematic).not.toHaveAttribute('data-selected-component', component);
      await expect(componentSymbol).not.toHaveClass(/is-component-active/);
      await componentSymbol.click();
      await expect(schematic).toHaveAttribute('data-selected-component', component);
      await componentSymbol.click();
      await expect(schematic).not.toHaveAttribute('data-selected-component', component);
      await componentSymbol.click();
      await surface.click({ position: { x: 4, y: 4 } });
      await expect(schematic).not.toHaveAttribute('data-selected-component', component);
    }

    if (process.env.KESSETSU_CAPTURE_VISUALS) {
      await schematic.screenshot({
        path: join('test-results', 'schematic-corpus', `${name}.png`),
        animations: 'disabled',
      });
    }
  }
});
