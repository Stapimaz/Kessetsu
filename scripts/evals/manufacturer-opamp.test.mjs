import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, copyFileSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { digest } from './runtime.mjs';
import { OFFICIAL_MODEL_SHA256, inspectU6, loadVerifiedModel, evaluateU6 } from './manufacturer-opamp.mjs';
import { FOLLOWUP_SCHEMA, validateKessetsuU6Compilation } from './manufacturer-opamp-followup.mjs';

const valid = `U6 fixture
VIN in 0 SIN(0 0.1 1000) AC 1
VP vcc 0 6
VN 0 vee 6
RF out feedback 40k
RG feedback 0 10k
RL out 0 10k
X1 in feedback vcc vee out OPAx197
.end
`;
const mockModel = `* Test-only model, not TI content
.SUBCKT OPAx197 IN+ IN- VCC VEE OUT
E1 OUT 0 IN+ IN- 1000000
R1 OUT 0 1e9
.ENDS OPAx197
`;
const kessValid = `external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPAx197 sha256=${OFFICIAL_MODEL_SHA256} version="Final 1.3" license="TI terms" source="https://www.ti.com/lit/zip/SBOMA34" simulator=ngspice_ps redistribution=prohibited
net GND
net VCC
net VEE
net IN
net FB
net OUT
source VP 6V
source VN 6V
source VIN sine_ac(0V,100mV,1kHz,1V)
opamp U1 OPA197
resistor RF 40k
resistor RG 10k
resistor RL 10k
connect VP.minus to GND
connect VP.plus to VCC
connect VN.plus to GND
connect VN.minus to VEE
connect VIN.minus to GND
connect VIN.plus to IN
connect U1.in_p to IN
connect U1.in_n to FB
connect U1.vcc to VCC
connect U1.vee to VEE
connect U1.out to OUT
connect RF.p1 to OUT
connect RF.p2 to FB
connect RG.p1 to FB
connect RG.p2 to GND
connect RL.p1 to OUT
connect RL.p2 to GND
simulate op
simulate ac dec 100 10Hz 1MHz
simulate tran 2us 10ms
`;
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('U6 topology rejects generic substitution and immutable requirement changes', () => {
  assert.equal(inspectU6(valid).rf, 40000);
  for (const candidate of [valid.replace('VP vcc 0 6', 'VP vcc 0 9'), valid.replace('RL out 0 10k', 'RL out 0 1k'),
    valid.replace('0.1 1000', '0.2 1000'), valid.replace('OPAx197', 'KESSETSU_OPAMP_V1'),
    valid.replace('RF out feedback', 'RF in feedback'), valid.replace('.end\n', '.include model.lib\n.end\n'),
    valid.replace('VIN in', '.end\nVIN in')]) assert.throws(() => inspectU6(candidate));
  for (const directive of ['.four 1k v(out)', '.print tran v(out)', '.plot tran v(out)', '.save v(out)']) {
    assert.deepEqual(inspectU6(valid.replace('.end\n', `${directive}\n.end\n`)), inspectU6(valid));
  }
});

test('U6 requires the exact externally acquired model hash and pin contract', () => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u6-model-'));
  try {
    const path = join(directory, 'model.lib'); writeFileSync(path, mockModel);
    assert.equal(loadVerifiedModel(path, digest(mockModel)).sha256, digest(mockModel));
    assert.throws(() => loadVerifiedModel(path), /INTEGRITY/);
    writeFileSync(path, mockModel.replace('IN+ IN-', 'IN- IN+'));
    assert.throws(() => loadVerifiedModel(path, digest(mockModel.replace('IN+ IN-', 'IN- IN+'))), /CAPABILITY/);
    assert.throws(() => loadVerifiedModel(), /ACQUISITION/);
    assert.match(OFFICIAL_MODEL_SHA256, /^[a-f0-9]{64}$/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('U6 evaluator measurement path passes a synthetic test-only five-pin model', () => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u6-mock-'));
  try {
    const path = join(directory, 'mock.lib'); writeFileSync(path, mockModel);
    const result = evaluateU6(valid, simulator, undefined, path, digest(mockModel));
    assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
    assert.equal(result.model_adequacy.authentic_manufacturer_model, false);
    assert.equal(result.model_adequacy.hardware_validated, false);
    const wrongGain = evaluateU6(valid.replace('40k', '10k'), simulator, undefined, path, digest(mockModel));
    assert.equal(wrongGain.status, 'FAIL');
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('official user-supplied TI model executes through bundled-Ngspice PSpice library mode',
  { skip: !process.env.KESSETSU_U6_MODEL }, () => {
    const result = evaluateU6(valid, simulator);
    assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
    assert.equal(result.model_adequacy.authentic_manufacturer_model, true);
    assert.equal(result.model_provenance.supplied_file_sha256, OFFICIAL_MODEL_SHA256);
    assert.ok(!result.evidence.testbench.includes('Green-Williams-Lis'));
  });

test('U6 CLI records acquisition failure rather than accepting a generic fallback', () => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u6-contract-'));
  try {
    const path = join(directory, 'candidate.spice'); writeFileSync(path, valid);
    const run = spawnSync(process.execPath, [fileURLToPath(new URL('./manufacturer-opamp.mjs', import.meta.url)), 'direct', path],
      { encoding: 'utf8', env: { ...process.env, KESSETSU_U6_MODEL: '' }, windowsHide: true });
    const report = JSON.parse(run.stdout);
    assert.equal(run.status, 2);
    assert.equal(report.schema_version, 'kessetsu.u6-evaluation.v2');
    assert.match(report.message, /MODEL_ACQUISITION_REQUIRED/);
    assert.equal(report.candidate_source, valid);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('U6 follow-up accepts only the exact typed external-model compilation contract', () => {
  const model = {
    name: 'OPA197', source: 'external',
    provenance: { content_hash: `sha256:${OFFICIAL_MODEL_SHA256}` },
    external: {
      resource: 'models/OPAx197.LIB', entry: 'OPAx197',
      pins: ['in_p', 'in_n', 'vcc', 'vee', 'out'],
      simulator: 'ngspice_ps', redistribution: 'prohibited',
    },
  };
  const compilation = { debug: { models: { manifest: { schema_version: 'kessetsu.models.v2', models: [model] } } } };
  const netlist = valid.replace('.end', '.include "models/OPAx197.LIB"\n.end');
  const checked = validateKessetsuU6Compilation(compilation, netlist);
  assert.equal(FOLLOWUP_SCHEMA, 'kessetsu.u6-followup-evaluation.v1');
  assert.equal(checked.contract.model_body_serialized, false);
  assert.ok(!checked.netlist.includes('.include'));

  for (const changed of [
    { ...model, provenance: { content_hash: 'sha256:changed' } },
    { ...model, external: { ...model.external, pins: ['in_n', 'in_p', 'vcc', 'vee', 'out'] } },
    { ...model, external: { ...model.external, simulator: 'ngspice' } },
    { ...model, external: { ...model.external, redistribution: 'permitted' } },
  ]) {
    const altered = { debug: { models: { manifest: { schema_version: 'kessetsu.models.v2', models: [changed] } } } };
    assert.throws(() => validateKessetsuU6Compilation(altered, netlist), /FOLLOWUP_CONTRACT_ERROR/);
  }
  assert.throws(() => validateKessetsuU6Compilation(compilation, valid), /FOLLOWUP_CONTRACT_ERROR/);
});

test('U6 follow-up runs file-bound Kessetsu through the exact official model without serializing its body',
  { skip: !process.env.KESSETSU_U6_MODEL }, () => {
    const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u6-followup-'));
    try {
      const models = join(directory, 'models'); mkdirSync(models);
      const modelPath = join(models, 'OPAx197.LIB');
      copyFileSync(process.env.KESSETSU_U6_MODEL, modelPath);
      const candidatePath = join(directory, 'candidate.kess'); writeFileSync(candidatePath, kessValid);
      const evaluator = fileURLToPath(new URL('./manufacturer-opamp-followup.mjs', import.meta.url));
      const run = spawnSync(process.execPath, [evaluator, 'kessetsu', candidatePath], {
        encoding: 'utf8', windowsHide: true, maxBuffer: 4 * 1024 * 1024,
        env: { ...process.env, KESSETSU_U6_MODEL: modelPath },
      });
      assert.equal(run.status, 0, run.stderr);
      const report = JSON.parse(run.stdout);
      assert.equal(report.schema_version, FOLLOWUP_SCHEMA);
      assert.equal(report.status, 'PASS');
      assert.equal(report.kessetsu_contract.content_hash, `sha256:${OFFICIAL_MODEL_SHA256}`);
      assert.equal(report.kessetsu_contract.model_body_serialized, false);
      assert.ok(!run.stdout.includes('Green-Williams-Lis'));
    } finally { rmSync(directory, { recursive: true, force: true }); }
  });
