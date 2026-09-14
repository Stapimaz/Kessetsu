import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

async function replaceSource(page: Page, source: string) {
  await page.locator('.monaco-editor').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Backspace');
  await expect(page.getByTestId('compile-success')).not.toBeVisible();
  await page.keyboard.insertText(source);
}

const packageCircuit = `model_include kessetsu_analog 1.0.0
net GND
net VDD
net OUT
source VS 5V
opamp U1 KESSETSU_PACKAGE_OPAMP
resistor R1 10k
connect VS.minus to GND
connect VS.plus to VDD
connect U1.in_p to GND
connect U1.in_n to OUT
connect U1.vcc to VDD
connect U1.vee to GND
connect U1.out to OUT
connect R1.p1 to OUT
connect R1.p2 to GND
simulate op
`;

test('keeps exact packages reproducible and rejects model directive injection in Web', async ({ page }) => {
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  await page.goto('/#editor');
  await replaceSource(page, packageCircuit);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId('model-manifest')).toHaveAttribute(
    'data-manifest',
    /"name":"kessetsu_analog","version":"1\.0\.0"/,
  );
  const manifest = JSON.parse(await page.getByTestId('model-manifest').getAttribute('data-manifest') ?? '{}');
  expect(manifest.packages[0]).toMatchObject({ name: 'kessetsu_analog', version: '1.0.0' });
  expect(manifest.models[0].name).toBe('KESSETSU_PACKAGE_OPAMP');

  const sourcePath = resolve('test-results/package-model.kess');
  const outputPath = resolve('test-results/package-model.spice');
  writeFileSync(sourcePath, packageCircuit);
  execFileSync(binary, ['compile', sourcePath, '--output', outputPath, '--force']);
  expect((await page.locator('.spice-details pre').textContent())?.replace(/\r\n/g, '\n'))
    .toBe(readFileSync(outputPath, 'utf8').replace(/\r\n/g, '\n'));

  await replaceSource(page, 'model diode Evil version=1 license=MIT Is="1e-9 .control"\n');
  await expect(page.getByText(/KES-C010/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run' })).toBeDisabled();
  await expect(page.locator('.spice-details pre')).toContainText('Waiting for a valid circuit');
});

test('fails closed when a browser design references user-owned external model bytes', async ({ page }) => {
  await page.goto('/#editor');
  await replaceSource(page, 'external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPA197 sha256=fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5 version="Final 1.3" license="TI terms" source="https://www.ti.com/lit/zip/SBOMA34" simulator=ngspice_ps redistribution=prohibited\nopamp U1 OPA197\n');
  const diagnostic = page.getByRole('button', { name: /KES-C015/ });
  await expect(diagnostic).toContainText("external model resource 'models/OPAx197.LIB' was not supplied");
  await expect(page.getByRole('button', { name: 'Run' })).toBeDisabled();
  await expect(page.locator('.spice-details pre')).toContainText('Waiting for a valid circuit');
});
