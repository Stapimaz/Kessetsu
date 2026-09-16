import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

const expectedBase = process.env.KESSETSU_BASE_PATH ?? '/';
const dist = resolve(import.meta.dirname, '../dist');
const html = readFileSync(resolve(dist, 'index.html'), 'utf8');
const pages = readdirSync(dist, { recursive: true }).map((file) => file.replaceAll('\\', '/')).filter((file) => file === 'index.html' || file.endsWith('/index.html'));
const titles = new Set();
let referenceCount = 0;
for (const file of pages) {
  const text = readFileSync(resolve(dist, file), 'utf8');
  const title = text.match(/<title>(.*?)<\/title>/)?.[1];
  if (!title || titles.has(title)) throw new Error(`Missing or duplicated public-page title: ${file}`);
  titles.add(title);
  if (!/<h1\b/.test(text)) throw new Error(`Public page has no initial HTML content: ${file}`);
  const route = file.slice(0, -'index.html'.length);
  if (!text.includes(`rel="canonical" href="https://kessetsu.com/${route}"`)) throw new Error(`Incorrect canonical URL: ${file}`);
  for (const match of text.matchAll(/(?:src|href)="([^"]+)"/g)) {
    const reference = match[1];
    if (/^(?:data:|https?:|mailto:|#)/.test(reference)) continue;
    const url = new URL(reference, `https://deployment.test${expectedBase}${route}`);
    if (!url.pathname.startsWith(expectedBase)) throw new Error(`Deployment asset escaped ${expectedBase}: ${reference}`);
    const relative = decodeURIComponent(url.pathname.slice(expectedBase.length));
    if (!existsSync(resolve(dist, relative))) throw new Error(`Deployment asset is missing: ${file} -> ${relative}`);
    referenceCount++;
  }
  if (route.startsWith('docs/') && /<script\b/.test(text)) throw new Error(`Static documentation unexpectedly loads JavaScript: ${file}`);
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
  'install/index.html',
  'docs/index.html',
  'docs/guides/tutorial/index.html',
  'docs.css',
  'install.ps1',
  'install.sh',
  'examples/rc_low_pass.kess',
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
const installHtml = readFileSync(resolve(dist, 'install/index.html'), 'utf8');
if (!installHtml.includes('rel="canonical" href="https://kessetsu.com/install/"')) throw new Error('Installation page canonical URL is missing.');
for (const script of ['install.ps1', 'install.sh']) {
  if (!readFileSync(resolve(dist, script), 'utf8').includes('Kessetsu')) throw new Error(`Missing installer source: ${script}`);
}
const sitemap = readFileSync(resolve(dist, 'sitemap.xml'), 'utf8');
for (const file of pages) {
  const route = file.slice(0, -'index.html'.length);
  if (!sitemap.includes(`<loc>https://kessetsu.com/${route}</loc>`)) throw new Error(`Sitemap omits public page: ${file}`);
}
console.log(`Deployment audit PASS: ${pages.length} content pages, distinct titles, ${referenceCount} local references; CSP, sitemap and license bundle present.`);
