import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const expectedBase = process.env.KESSETSU_BASE_PATH ?? '/';
const dist = resolve(import.meta.dirname, '../dist');
const html = readFileSync(resolve(dist, 'index.html'), 'utf8');
const references = [...html.matchAll(/(?:src|href)="([^"]+)"/g)].map((match) => match[1]);
for (const reference of references) {
  if (/^(?:data:|https?:)/.test(reference)) continue;
  if (!reference.startsWith(expectedBase)) throw new Error(`Deployment asset escaped ${expectedBase}: ${reference}`);
  const relative = reference.slice(expectedBase.length);
  if (!existsSync(resolve(dist, relative))) throw new Error(`Deployment asset is missing: ${relative}`);
}
for (const directive of ["default-src 'self'", "script-src 'self' 'wasm-unsafe-eval'", "worker-src 'self' blob:", "object-src 'none'"]) {
  if (!html.includes(directive)) throw new Error(`Production CSP is missing: ${directive}`);
}
for (const metadata of [
  '<link rel="canonical" href="https://kessetsu.com/"',
  '<meta property="og:url" content="https://kessetsu.com/"',
  '<meta property="og:image" content="https://kessetsu.com/og-kessetsu.png"',
  '<meta name="twitter:card" content="summary_large_image"',
]) {
  if (!html.includes(metadata)) throw new Error(`Production metadata is missing: ${metadata}`);
}
for (const required of [
  'CNAME',
  'robots.txt',
  'sitemap.xml',
  'og-kessetsu.png',
  'performance-budget.json',
  'LICENSE.txt',
  'NOTICE.txt',
  'COMMERCIAL_LICENSE.md',
  'licenses/eecircuit-engine-MIT.txt',
  'licenses/ngspice-COPYING.txt',
  'licenses/inter-OFL.txt',
  'licenses/RobotoMono-OFL.txt',
  'licenses/npm-inventory.json',
]) {
  if (!existsSync(resolve(dist, required))) throw new Error(`Deployment notice is missing: ${required}`);
}
if (readFileSync(resolve(dist, 'CNAME'), 'utf8').trim() !== 'kessetsu.com') {
  throw new Error('Production CNAME must equal kessetsu.com');
}
console.log(`Deployment audit PASS: ${references.length} assets use ${expectedBase}; CSP and license bundle present.`);
