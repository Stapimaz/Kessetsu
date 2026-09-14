import { createHash } from 'node:crypto';
import { readFileSync, statSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const manifest = JSON.parse(readFileSync(resolve(root, 'public/runtime-manifest.json'), 'utf8'));
const lock = JSON.parse(readFileSync(resolve(root, 'package-lock.json'), 'utf8'));
const packageEntry = lock.packages?.['node_modules/eecircuit-engine'];
const enginePath = resolve(root, 'node_modules/eecircuit-engine/dist/eecircuit-engine.mjs');
const engineBytes = readFileSync(enginePath);
const sha256 = createHash('sha256').update(engineBytes).digest('hex');

function requireEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label} mismatch: expected ${expected}, received ${actual}`);
  }
}

requireEqual(packageEntry?.version, manifest.simulator.version, 'runtime version');
requireEqual(packageEntry?.integrity, manifest.simulator.npm_integrity, 'npm integrity');
requireEqual(sha256, manifest.simulator.esm_sha256, 'runtime ESM SHA-256');
requireEqual(engineBytes.length, manifest.simulator.esm_bytes, 'runtime ESM byte length');

for (const path of [
  'dist/licenses/eecircuit-engine-MIT.txt',
  'dist/licenses/ngspice-COPYING.txt',
  'dist/licenses/inter-OFL.txt',
  'dist/THIRD_PARTY_NOTICES.md',
]) {
  if (statSync(resolve(root, path)).size === 0) throw new Error(`${path} is empty`);
}

console.log(`Verified eecircuit-engine@${packageEntry.version} (${sha256}) and runtime notices.`);
