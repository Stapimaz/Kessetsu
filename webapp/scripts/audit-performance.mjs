import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { gzipSync } from 'node:zlib';

const root = resolve(import.meta.dirname, '..');
const assetsDirectory = resolve(root, 'dist/assets');
const assets = readdirSync(assetsDirectory).map((name) => {
  const bytes = readFileSync(resolve(assetsDirectory, name));
  return { name, bytes: bytes.length, gzip_bytes: gzipSync(bytes, { level: 9 }).length };
});

function select(label, patterns) {
  const selected = assets.filter((asset) => patterns.some((pattern) => pattern.test(asset.name)));
  if (selected.length !== patterns.length) {
    throw new Error(`${label} expected ${patterns.length} assets, found ${selected.map((asset) => asset.name).join(', ')}`);
  }
  return selected;
}

const stages = [
  {
    id: 'landing',
    description: 'Initial landing JavaScript and CSS',
    budget_gzip_bytes: 90 * 1024,
    assets: select('landing', [/^index-.*\.js$/, /^index-.*\.css$/]),
  },
  {
    id: 'editor_activation',
    description: 'Editor UI, Monaco worker and canonical Core WASM loaded after Open Web Hub',
    // Reviewed shared experiment/measurement contract growth: measured 2970 KiB.
    // Keep a close feature ceiling; landing and Ngspice payload budgets are unchanged.
    budget_gzip_bytes: 3020 * 1024,
    assets: select('editor activation', [/^WorkspaceApp-.*\.js$/, /^WorkspaceApp-.*\.css$/, /^editor\.worker-.*\.js$/, /^kessetsu_core_bg-.*\.wasm$/]),
  },
  {
    id: 'simulation_activation',
    description: 'Browser simulation engine and Worker loaded only on first Run',
    budget_gzip_bytes: 5700 * 1024,
    assets: select('simulation activation', [/^eecircuit-engine-.*\.js$/, /^simulation\.worker-.*\.js$/]),
  },
].map((stage) => ({
  ...stage,
  gzip_bytes: stage.assets.reduce((sum, asset) => sum + asset.gzip_bytes, 0),
}));

for (const stage of stages) {
  if (stage.gzip_bytes > stage.budget_gzip_bytes) {
    throw new Error(`${stage.id} is ${stage.gzip_bytes} gzip bytes; budget is ${stage.budget_gzip_bytes}`);
  }
}

const report = {
  schema_version: 'kessetsu.web-performance.v1',
  compression: 'gzip-9',
  stages,
};
writeFileSync(resolve(root, 'dist/performance-budget.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Web performance budget PASS: ${stages.map((stage) => `${stage.id}=${Math.ceil(stage.gzip_bytes / 1024)} KiB`).join(', ')}.`);
