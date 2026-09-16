import { expect, test } from '@playwright/test';

test('serves HTTPS, correctly typed cached assets, and a same-origin no-upload runtime', async ({ page, request }) => {
  test.setTimeout(120_000);
  const origin = 'https://kessetsu.com';
  const requests: { url: string; method: string }[] = [];
  const failures: string[] = [];
  const pageErrors: string[] = [];
  page.on('request', (entry) => {
    if (/^https?:/.test(entry.url())) requests.push({ url: entry.url(), method: entry.method() });
  });
  page.on('requestfailed', (entry) => failures.push(`${entry.url()}: ${entry.failure()?.errorText}`));
  page.on('pageerror', (error) => pageErrors.push(error.message));

  // A unique read-only URL avoids a stale HTTP response cached before enforcement.
  const redirect = await request.get(`http://kessetsu.com/?https-check=${Date.now()}`, { maxRedirects: 0 });
  expect([301, 302, 307, 308]).toContain(redirect.status());
  expect(redirect.headers().location).toMatch(/^https:\/\/kessetsu\.com\//);

  const document = await page.goto('/');
  expect(document?.status()).toBe(200);
  expect(new URL(document!.url()).origin).toBe(origin);
  expect(document!.headers()['content-type']).toContain('text/html');
  expect(document!.headers()['cache-control']).toMatch(/\bmax-age=\d+/);
  await expect(page.locator('meta[http-equiv="Content-Security-Policy"]')).toHaveAttribute('content', /default-src 'self'/);
  const csp = await page.locator('meta[http-equiv="Content-Security-Policy"]').getAttribute('content');
  for (const directive of ["script-src 'self' 'wasm-unsafe-eval'", "connect-src 'self' blob:", "object-src 'none'"]) {
    expect(csp).toContain(directive);
  }

  const manifestResponse = await request.get(`${origin}/performance-budget.json`);
  expect(manifestResponse.status()).toBe(200);
  const manifest = await manifestResponse.json() as {
    schema_version: string;
    stages: { assets: { name: string; bytes: number }[]; gzip_bytes: number; budget_gzip_bytes: number }[];
  };
  expect(manifest.schema_version).toBe('kessetsu.web-performance.v1');
  const assets = manifest.stages.flatMap((stage) => {
    expect(stage.gzip_bytes).toBeLessThanOrEqual(stage.budget_gzip_bytes);
    return stage.assets;
  });
  expect(assets.some((asset) => asset.name.endsWith('.wasm'))).toBe(true);
  expect(assets.some((asset) => asset.name.startsWith('simulation.worker-'))).toBe(true);
  for (const asset of assets) {
    expect(asset.name).toMatch(/^[A-Za-z0-9_.-]+$/);
    const response = await request.head(`${origin}/assets/${asset.name}`);
    expect(response.status(), asset.name).toBe(200);
    const mime = asset.name.endsWith('.wasm') ? 'application/wasm'
      : asset.name.endsWith('.css') ? 'text/css' : '(?:application|text)/javascript';
    expect(response.headers()['content-type'], asset.name).toMatch(new RegExp(mime));
    expect(response.headers()['cache-control'], asset.name).toMatch(/\bmax-age=\d+/);
    const length = Number(response.headers()['content-length']);
    expect(length, asset.name).toBeGreaterThan(0);
    // Transport compression may change Content-Length without changing the asset.
    if (!response.headers()['content-encoding']) expect(length, asset.name).toBe(asset.bytes);
  }

  await page.locator('.landing-hero').getByRole('link', { name: 'Open Web Hub' }).click();
  await expect(page.getByTestId('compile-success')).toBeVisible();
  await page.getByRole('button', { name: 'Run' }).click();
  await expect(page.getByTestId('simulation-summary')).toHaveAttribute('data-state', 'succeeded', { timeout: 90_000 });
  expect(requests.some((entry) => entry.url.endsWith('.wasm'))).toBe(true);
  expect(requests.some((entry) => /simulation\.worker-.*\.js$/.test(entry.url))).toBe(true);
  expect(requests.filter((entry) => new URL(entry.url).origin !== origin)).toEqual([]);
  expect(requests.filter((entry) => !['GET', 'HEAD'].includes(entry.method))).toEqual([]);
  expect(failures).toEqual([]);
  expect(pageErrors).toEqual([]);
});
