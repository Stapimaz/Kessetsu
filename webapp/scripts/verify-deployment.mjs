import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const expectedBase = process.env.NETLANG_BASE_PATH ?? '/';
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
for (const required of [
  'LICENSE.txt',
  'NOTICE.txt',
  'COMMERCIAL_LICENSE.md',
  'licenses/eecircuit-engine-MIT.txt',
  'licenses/ngspice-COPYING.txt',
  'licenses/RobotoMono-OFL.txt',
  'licenses/npm-inventory.json',
]) {
  if (!existsSync(resolve(dist, required))) throw new Error(`Deployment notice is missing: ${required}`);
}
console.log(`Deployment audit PASS: ${references.length} assets use ${expectedBase}; CSP and license bundle present.`);
