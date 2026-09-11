import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { digest } from './runtime.mjs';
import { OFFICIAL_MODEL_SHA256, inspectU6, loadVerifiedModel, evaluateU6 } from './manufacturer-opamp.mjs';

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
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('U6 topology rejects generic substitution and immutable requirement changes', () => {
  assert.equal(inspectU6(valid).rf, 40000);
  for (const candidate of [valid.replace('VP vcc 0 6', 'VP vcc 0 9'), valid.replace('RL out 0 10k', 'RL out 0 1k'),
    valid.replace('0.1 1000', '0.2 1000'), valid.replace('OPAx197', 'KESSETSU_OPAMP_V1'),
    valid.replace('RF out feedback', 'RF in feedback'), valid.replace('.end\n', '.include model.lib\n.end\n'),
    valid.replace('VIN in', '.end\nVIN in')]) assert.throws(() => inspectU6(candidate));
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

test('official user-supplied TI model records bundled-Ngspice capability failure',
  { skip: !process.env.KESSETSU_U6_MODEL }, () => {
    assert.throws(() => evaluateU6(valid, simulator),
      (error) => /U6_MODEL_CAPABILITY_ERROR/.test(error.message) && /no such function 'if'/.test(error.evidence.simulator_stderr) &&
        !error.evidence.testbench.includes('Green-Williams-Lis') && error.evidence.model_provenance.supplied_file_sha256 === OFFICIAL_MODEL_SHA256);
  });

test('U6 CLI records acquisition failure rather than accepting a generic fallback', () => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u6-contract-'));
  try {
    const path = join(directory, 'candidate.spice'); writeFileSync(path, valid);
    const run = spawnSync(process.execPath, [fileURLToPath(new URL('./manufacturer-opamp.mjs', import.meta.url)), 'direct', path],
      { encoding: 'utf8', env: { ...process.env, KESSETSU_U6_MODEL: '' }, windowsHide: true });
    const report = JSON.parse(run.stdout);
    assert.equal(run.status, 2);
    assert.match(report.message, /MODEL_ACQUISITION_REQUIRED/);
    assert.equal(report.candidate_source, valid);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
