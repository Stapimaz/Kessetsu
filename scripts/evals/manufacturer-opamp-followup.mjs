import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { evaluatorMain } from './runtime.mjs';
import { evaluateU6, OFFICIAL_MODEL_SHA256 } from './manufacturer-opamp.mjs';

export const FOLLOWUP_SCHEMA = 'kessetsu.u6-followup-evaluation.v1';
export const FOLLOWUP_SPEC = 'scripts/evals/specs/unseen-design-u6-followup-v1.md';

export function validateKessetsuU6Compilation(compilation, netlist, expectedHash = OFFICIAL_MODEL_SHA256) {
  const manifest = compilation?.debug?.models?.manifest;
  const models = manifest?.models;
  if (manifest?.schema_version !== 'kessetsu.models.v3' || !Array.isArray(models)) {
    throw new Error('U6_FOLLOWUP_CONTRACT_ERROR: missing kessetsu.models.v3 manifest');
  }
  const external = models.filter((model) => model?.external);
  if (external.length !== 1) {
    throw new Error('U6_FOLLOWUP_CONTRACT_ERROR: exactly one external model is required');
  }
  const model = external[0], metadata = model.external;
  const hash = model.provenance?.content_hash?.replace(/^sha256:/, '');
  const exact = model.name === 'OPA197'
    && model.source?.toLowerCase() === 'external'
    && hash === expectedHash
    && metadata.resource === 'models/OPAx197.LIB'
    && metadata.entry === 'OPAx197'
    && JSON.stringify(metadata.pins) === JSON.stringify(['in_p', 'in_n', 'vcc', 'vee', 'out'])
    && metadata.simulator === 'ngspice_ps'
    && metadata.redistribution === 'prohibited';
  if (!exact) {
    throw new Error('U6_FOLLOWUP_CONTRACT_ERROR: external model identity, hash, pins, runtime, or redistribution policy changed');
  }

  const lines = netlist.split(/\r?\n/);
  const includes = lines.filter((line) => /^\.include\s+"models\/OPAx197\.LIB"\s*$/i.test(line.trim()));
  if (includes.length !== 1) {
    throw new Error('U6_FOLLOWUP_CONTRACT_ERROR: canonical netlist must contain one exact relative model include');
  }
  return {
    netlist: lines.filter((line) => !/^\.include\s+/i.test(line.trim())).join('\n'),
    contract: {
      manifest_schema: manifest.schema_version,
      model: model.name,
      content_hash: model.provenance.content_hash,
      resource: metadata.resource,
      entry: metadata.entry,
      pins: metadata.pins,
      simulator: metadata.simulator,
      redistribution: metadata.redistribution,
      model_body_serialized: false,
    },
  };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  evaluatorMain('U6', evaluateU6, 1, {
    schemaVersion: FOLLOWUP_SCHEMA,
    specPath: FOLLOWUP_SPEC,
    compileKessetsuFromFile: true,
    validateKessetsuCompilation: validateKessetsuU6Compilation,
  });
}
