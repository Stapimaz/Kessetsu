import { expect, test } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { readFileSync, mkdtempSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import { encodeShareFragment } from '../../src/share';

for (const fixture of [
  { name: 'comparator', alias: 'CMP', label: 'External Comparator', file: 'comparator.lib', assertions: 4 },
  { name: 'memristor', alias: 'MEM', label: 'Threshold Memristor', file: 'memristor.lib', assertions: 3 },
]) {
  test(`${fixture.label}: local binding, native parity and no model persistence`, async ({ page }) => {
    test.setTimeout(120_000);
    const sourcePath = resolve(`../examples/external_${fixture.name}.kess`);
    const source = readFileSync(sourcePath, 'utf8');
    const model = readFileSync(resolve(`../examples/models/${fixture.file}`));
    const folder = mkdtempSync(join(tmpdir(), 'kessetsu-browser-local-model-'));
    let native;
    try {
      const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
      mkdirSync(join(folder, 'models'));
      writeFileSync(join(folder, 'models', fixture.file), model);
      writeFileSync(join(folder, 'circuit.kess'), source);
      const result = spawnSync(binary, ['test', join(folder, 'circuit.kess'), '--format', 'json'], { encoding: 'utf8', timeout: 60_000 });
      expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0);
      native = JSON.parse(result.stdout);
    } finally { rmSync(folder, { recursive: true, force: true }); }
    const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, fixture.label);
    await page.goto(`/${fragment}`);
    await page.getByRole('button', { name: 'View', exact: true }).click();
    await page.getByRole('menuitem', { name: /Circuit details/ }).click();
    const dialog = page.getByRole('dialog', { name: 'Circuit details' });
    await expect(dialog).toContainText('File required');
    await dialog.getByLabel(`Model file for ${fixture.alias}`).setInputFiles({ name: fixture.file, mimeType: 'text/plain', buffer: model });
    await expect(dialog).toContainText('File selected');
    await dialog.getByRole('button', { name: 'Close circuit details' }).click();
    await expect(page.getByTestId('compile-success')).toBeVisible();
    await expect(page.getByTestId('canonical-schematic')).toHaveAttribute('data-quality', 'pass');
    await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
    const summary = page.getByTestId('simulation-summary');
    await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
    await expect(summary.locator('.assertion-pass')).toHaveCount(fixture.assertions);
    for (const assertion of native.assertions.assertions) {
      const actual = Number(await summary.locator(`[data-assertion-code="${assertion.code}"]`).getAttribute('data-actual'));
      expect(Math.abs(actual - assertion.actual)).toBeLessThan(Math.max(1e-6, Math.abs(assertion.actual) * 1e-4));
    }
    await page.getByTestId('canonical-schematic').screenshot({ path: `test-results/external-${fixture.name}.png` });
    const draft = await page.evaluate(() => JSON.stringify({ ...localStorage }));
    expect(draft).not.toContain('Bdrive');
    expect(draft).not.toContain('Bx 0 x');
    await page.getByRole('button', { name: 'Share circuit' }).click();
    const share = page.getByRole('dialog', { name: 'Share circuit' });
    await expect(share).toContainText('Local model files are not included');
    await share.getByRole('button', { name: 'Copy link' }).click();
    await expect(share.getByRole('status')).toContainText(/Link (copied|created)/);
    await page.reload();
    await page.getByRole('button', { name: 'View', exact: true }).click();
    await page.getByRole('menuitem', { name: /Circuit details/ }).click();
    await expect(dialog).toContainText('File required');
    await dialog.getByLabel(`Model file for ${fixture.alias}`).setInputFiles({ name: fixture.file, mimeType: 'text/plain', buffer: model });
    await expect(dialog).toContainText('File selected');
    await dialog.getByRole('button', { name: 'Clear files' }).click();
    await expect(dialog).toContainText('File required');
    await dialog.getByRole('button', { name: 'Close circuit details' }).click();
    await expect(page.getByTestId('compile-success')).not.toBeVisible();
  });
}

test('native-only mode fails before loading the browser simulator', async ({ page }) => {
  const source = readFileSync(resolve('../examples/external_comparator.kess'), 'utf8').replace('simulator=ngspice ', 'simulator=ngspice_ps ');
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Native-only comparator');
  let engineRequests = 0;
  page.on('request', (request) => { if (/ngspice\.wasm/.test(request.url())) engineRequests += 1; });
  await page.goto(`/${fragment}`);
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: /Circuit details/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Circuit details' });
  await expect(dialog).toContainText('Native-only compatibility');
  await dialog.getByLabel('Model file for CMP').setInputFiles(resolve('../examples/models/comparator.lib'));
  await dialog.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'failed');
  await expect(page.getByTestId('simulation-summary')).toContainText('native-only');
  expect(engineRequests).toBe(0);
});

test('portable external op-amp and wrong-file recovery use the same local binding path', async ({ page }) => {
  // Synthetic contract fixture, never manufacturer-model evidence.
  const body = Buffer.from('.subckt PORTABLE_OPAMP inp inn vcc vee out\nE1 out 0 inp inn 100000\nR1 out 0 1e9\n.ends PORTABLE_OPAMP\n');
  const hash = createHash('sha256').update(body).digest('hex');
  const source = `external_subcircuit opamp AMP (in_p,in_n,vcc,vee,out) file="amp.lib" entry=PORTABLE_OPAMP sha256=${hash} version=1.0.0 license=AGPL-3.0-only source="synthetic contract fixture" simulator=ngspice redistribution=permitted
net GND
net VCC
net IN
net OUT
source VP 5V
source VIN 1.2V
opamp U1 AMP
resistor RL 10k
connect VP.plus to VCC
connect VP.minus, VIN.minus, U1.vee, RL.p2 to GND
connect VIN.plus, U1.in_p to IN
connect U1.vcc to VCC
connect U1.in_n, U1.out, RL.p1 to OUT
simulate op
assert value(V(OUT)) > 1.199V
assert value(V(OUT)) < 1.201V
`;
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v5', null, 'Portable op-amp fixture');
  await page.goto(`/${fragment}`);
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: /Circuit details/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Circuit details' });
  await dialog.getByLabel('Model file for AMP').setInputFiles({ name: 'amp.lib', mimeType: 'text/plain', buffer: Buffer.concat([body, Buffer.from('* wrong bytes\n')]) });
  await dialog.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(page.locator('.inline-diagnostics')).toContainText('KES-C016');
  await page.getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: /Circuit details/ }).click();
  await dialog.getByLabel('Model file for AMP').setInputFiles({ name: 'different-filename.lib', mimeType: 'text/plain', buffer: body });
  await dialog.getByRole('button', { name: 'Close circuit details' }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  await expect(summary.locator('.assertion-pass')).toHaveCount(2);
  const actual = Number(await summary.locator('[data-assertion-code="KES-T001"]').getAttribute('data-actual'));
  expect(Math.abs(actual - 1.2 * 100000 / 100001)).toBeLessThan(1e-6);
});
