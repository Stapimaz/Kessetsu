import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../node_modules', import.meta.url);
const denied = /(?:^|[^A-Z])(AGPL|GPL|SSPL)(?:-|\b)/i;
const packages = new Map();
const visited = new Set();

function visitModules(directory) {
  if (!existsSync(directory)) return;
  const canonical = realpathSync(directory);
  if (visited.has(canonical)) return;
  visited.add(canonical);
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.name.startsWith('.')) continue;
    if (entry.name.startsWith('@')) {
      for (const scoped of readdirSync(join(directory, entry.name), { withFileTypes: true })) {
        if (scoped.isDirectory()) visitPackage(join(directory, entry.name, scoped.name));
      }
    } else {
      visitPackage(join(directory, entry.name));
    }
  }
}

function visitPackage(directory) {
  const manifestPath = join(directory, 'package.json');
  if (!existsSync(manifestPath)) return;
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const license = typeof manifest.license === 'string'
    ? manifest.license
    : Array.isArray(manifest.licenses)
      ? manifest.licenses.map((entry) => entry.type).filter(Boolean).join(' OR ')
      : '';
  packages.set(`${manifest.name}@${manifest.version}`, license);
  visitModules(join(directory, 'node_modules'));
}

visitModules(root.pathname.replace(/^\/(.:)/, '$1'));
const missing = [...packages].filter(([, license]) => !license || license === 'UNLICENSED');
const incompatible = [...packages].filter(([, license]) => denied.test(license));
if (missing.length || incompatible.length) {
  if (missing.length) console.error('Packages without a declared license:', missing);
  if (incompatible.length) console.error('Packages requiring copyleft review:', incompatible);
  process.exit(1);
}
console.log(`npm license audit PASS: ${packages.size} installed packages declare non-GPL/AGPL/SSPL licenses.`);
const outputFlag = process.argv.indexOf('--output');
if (outputFlag >= 0) {
  const output = process.argv[outputFlag + 1];
  if (!output) throw new Error('--output requires a path');
  mkdirSync(new URL('../dist/licenses/', import.meta.url), { recursive: true });
  writeFileSync(output, `${JSON.stringify({ schema_version: 'netlang.npm-licenses.v1', packages: Object.fromEntries([...packages].sort()) }, null, 2)}\n`);
}
