import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

test('downloads every advertised artifact from the shared Core contract', async ({ page }) => {
  await page.goto('/#editor');
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await page.screenshot({ path: 'test-results/exports-ui.png', fullPage: true });
  const expected = ['svg', 'png', 'pdf', 'schematic_json', 'spice', 'kicad', 'ltspice'];
  const binary = resolve('../core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
  const fixture = resolve('../core/tests/fixtures/benchmarks/rc_filter.kess');
  await expect(page.locator('[data-export-format]')).toHaveCount(expected.length);

  for (const format of expected) {
    const downloadPromise = page.waitForEvent('download');
    await page.locator(`[data-export-format="${format}"]`).click();
    const download = await downloadPromise;
    const suggested = download.suggestedFilename();
    const path = await download.path();
    expect(path).toBeTruthy();
    const bytes = await download.createReadStream().then(async (stream) => {
      const chunks: Buffer[] = [];
      for await (const chunk of stream) chunks.push(Buffer.from(chunk));
      return Buffer.concat(chunks);
    });
    expect(bytes.length).toBeGreaterThan(20);
    if (format === 'png') expect(bytes.subarray(0, 8)).toEqual(Buffer.from('\x89PNG\r\n\x1a\n', 'binary'));
    if (format === 'pdf') expect(bytes.subarray(0, 5).toString()).toBe('%PDF-');
    if (format === 'kicad') expect(bytes.toString('utf8', 0, 10)).toContain('(kicad_sch');
    if (format === 'ltspice') expect(bytes.toString('utf8', 0, 20)).toContain('Version 4');
    expect(suggested).toContain('circuit.');
    const nativePath = resolve(`test-results/native-export-${format}.${suggested.split('.').slice(1).join('.')}`);
    writeFileSync(resolve(`test-results/browser-export-${format}.${suggested.split('.').slice(1).join('.')}`), bytes);
    if (['svg', 'png', 'pdf'].includes(format)) {
      execFileSync(binary, ['render', fixture, '--output', nativePath, '--force']);
    } else {
      const target = format.replace('_', '-');
      execFileSync(binary, ['export', fixture, '--target', target, '--output', nativePath, '--force']);
    }
    const nativeBytes = readFileSync(nativePath);
    if (format === 'pdf') {
      // svg2pdf's deflate stream may differ between native and WASM targets. Both
      // exports still originate from the byte-identical canonical SVG checked in
      // this loop, so compare the stable PDF structure instead of compressed bytes.
      const browserPdf = bytes.toString('latin1');
      const nativePdf = nativeBytes.toString('latin1');
      expect(browserPdf.match(/\/Type \/Page\b/g)?.length, 'PDF page parity').toBe(
        nativePdf.match(/\/Type \/Page\b/g)?.length,
      );
      expect(browserPdf.match(/\/Subtype \/Form\b/g)?.length, 'PDF form parity').toBe(
        nativePdf.match(/\/Subtype \/Form\b/g)?.length,
      );
      expect(browserPdf.match(/\/MediaBox\s*\[[^\]]+\]/)?.[0], 'PDF media box parity').toBe(
        nativePdf.match(/\/MediaBox\s*\[[^\]]+\]/)?.[0],
      );
    } else {
      expect(bytes.equals(nativeBytes), `${format} native/WASM byte parity`).toBe(true);
    }
    await expect(page.locator('.export-status')).not.toContainText('undefined');
  }

  await expect(page.locator('.export-status')).toContainText('engineering assertions remain');
});
