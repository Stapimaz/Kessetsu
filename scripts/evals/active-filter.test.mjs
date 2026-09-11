import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { MODEL, inspectU2, referenceU2, settledPeak, evaluateU2 } from './active-filter.mjs';

// Developer-created evaluator fixtures, NOT independent agent trials.
const valid = `Test U2
VIN in 0 SIN(0 0.05 100) AC 1
VP vcc 0 6
VN 0 vee 6
RS in filtered 10k
C1 filtered 0 8n
RF out feedback 20k
RG feedback 0 10k
RL out 0 10k
X1 filtered feedback vcc vee out KESSETSU_OPAMP_V1
${MODEL}
.end
`;
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('U2 accepts the supported topology and rejects requirement/model tampering', () => {
  assert.equal(inspectU2(valid).feedback, 20000);
  for (const candidate of [valid.replace('RL out 0 10k', 'RL out 0 20k'), valid.replace('VP vcc 0 6', 'VP vcc 0 9'),
    valid.replace('0.05 100', '0.01 100'), valid.replace('AC 1', 'AC 2'), valid.replace('200000', '200001'),
    valid.replace('X1 filtered feedback', 'X1 feedback filtered'), valid.replace('.end\n', '.include other.model\n.end\n'),
    valid.replace('C1 filtered 0 8n', 'C1 in 0 8n'), valid.replace('RG feedback 0 10k', 'RG feedback 0 -10k'),
    valid.replace('vcc vee out KESSETSU', 'vcc vcc out KESSETSU'), valid.replace('RS in filtered 10k', 'RS in filtered 1e999'),
    valid.replace('.end\n', `${MODEL}\n.end\n`), valid.replace('VIN in', '.end\nVIN in')]) assert.throws(() => inspectU2(candidate));
  assert.deepEqual(inspectU2(valid.replace('.end\n', '.control\necho FAKE PASS\nquit\n.endc\n.end\n')), inspectU2(valid));
});

test('independent finite-gain formula tends to expected non-inverting gain and RC cutoff', () => {
  const circuit = inspectU2(valid);
  assert.ok(Math.abs(referenceU2(circuit, 0)[0] - 3) < 0.0001);
  const cutoff = 1 / (2 * Math.PI * circuit.resistance * circuit.capacitance);
  assert.ok(Math.abs(Math.hypot(...referenceU2(circuit, cutoff)) - 3 / Math.sqrt(2)) < 0.001);
  assert.ok(referenceU2(circuit, 100)[1] < 0);
});

test('transient measurement rejects missing windows, coarse steps, nonfinite and reversed data', () => {
  const rows = Array.from({ length: 50001 }, (_, index) => [index * 2e-6, 0.15 * Math.sin(2 * Math.PI * 100 * index * 2e-6)]);
  assert.ok(Math.abs(settledPeak(rows) - 0.15) < 1e-8);
  for (const invalid of [rows.slice(0, -10), rows.slice(30000), [...rows].reverse(), rows.filter((_, i) => i % 2 === 0),
    rows.map((row, i) => i === 40000 ? [row[0], NaN] : row)]) assert.throws(() => settledPeak(invalid));
});

test('real U2 simulation agrees with independent formula; wrong gain/cutoff fail', () => {
  const result = evaluateU2(valid, simulator);
  assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
  assert.equal(result.model_adequacy.hardware_validated, false);
  assert.equal(result.checks.independent_formula_agreement, true);
  assert.ok(result.measurements.settled_peak_v > 0.149);
  assert.equal(evaluateU2(valid.replace('8n', '80n'), simulator).checks.relative_cutoff, false);
  assert.equal(evaluateU2(valid.replace('20k', '40k'), simulator).checks.gain_100hz, false);
});

test('two-section RC ladder includes interstage loading in the independent formula', () => {
  const ladder = valid.replace('RS in filtered 10k\nC1 filtered 0 8n', 'RS in middle 10k\nR2 middle filtered 10k\nC1 middle 0 3n\nC2 filtered 0 3n');
  const result = evaluateU2(ladder, simulator);
  assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
  assert.equal(result.checks.independent_formula_agreement, true);
  assert.throws(() => inspectU2(ladder.replace('C2 filtered 0', 'C2 middle 0')));
  assert.throws(() => inspectU2(ladder.replace('R2 middle filtered', 'R2 middle out')));
});

test('declared linear models retain actual gain/pole and reject altered structure', () => {
  const custom = valid.replaceAll('KESSETSU_OPAMP_V1', 'CustomOp').replace('200000', '100000').replace('0.03183098861837907', '0.007957747154594767');
  const result = evaluateU2(custom, simulator);
  assert.equal(result.status, 'PASS');
  assert.equal(result.model_adequacy.model_name, 'customop');
  assert.notEqual(result.model_adequacy.model_sha256, evaluateU2(valid, simulator).model_adequacy.model_sha256);
  for (const altered of [custom.replace('RPOLE n_int out 1', 'RPOLE n_int out 2'), custom.replace('100000', '-1'),
    custom.replace('EGAIN n_int 0 in_p in_n', 'EGAIN n_int 0 in_n in_p'), custom.replace('RLOAD out 0 1e9', 'RLOAD out 0 1e9\nB1 out 0 V=1')]) assert.throws(() => inspectU2(altered));
});

test('U2 retains partial and malformed datasets on process/parse failures', () => {
  for (const status of [0, 1]) {
    assert.throws(() => evaluateU2(valid, 'fake-test-runner', (_exe, _args, options) => {
      for (const name of ['op', 'ac', 'tran']) writeFileSync(join(options.cwd, `${name}.data`), 'malformed result');
      return { status, stdout: 'partial stdout', stderr: 'diagnostic stderr', signal: null };
    }), (error) => error.evidence.op_data === 'malformed result' && error.evidence.transient_data === 'malformed result' &&
      error.evidence.simulator_stderr === 'diagnostic stderr' && error.evidence.simulator_exit_code === status);
  }
});

test('both U2 frontends retain evidence and failed requirement provenance', () => {
  const source = `net GND
net IN
net FILTERED
net FB
net OUT
net VCC
net VEE
source VIN sine_ac(0V,50mV,100Hz,1V)
source VP 6V
source VN 6V
opamp U1 KESSETSU_OPAMP_V1
resistor RS 10k
capacitor C1 8n
resistor RF 20k
resistor RG 10k
resistor RL 10k
connect VIN.plus,RS.p1 to IN
connect VIN.minus,VP.minus,VN.plus,C1.p2,RG.p2,RL.p2 to GND
connect VP.plus,U1.vcc to VCC
connect VN.minus,U1.vee to VEE
connect RS.p2,C1.p1,U1.in_p to FILTERED
connect RF.p2,RG.p1,U1.in_n to FB
connect U1.out,RF.p1,RL.p1 to OUT
`;
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u2-contract-'));
  try {
    const customSource = 'subcircuit opamp CustomOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz\n' + source.replace('KESSETSU_OPAMP_V1', 'CustomOp');
    for (const [arm, candidate, expected] of [['direct', valid, 0], ['kessetsu', source, 0], ['kessetsu', customSource, 0], ['direct', valid.replace('VP vcc 0 6', 'VP vcc 0 9'), 2]]) {
      const path = join(directory, 'candidate');
      writeFileSync(path, candidate);
      const run = spawnSync(process.execPath, [fileURLToPath(new URL('./active-filter.mjs', import.meta.url)), arm, path], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: 60000, windowsHide: true });
      const report = JSON.parse(run.stdout);
      assert.equal(run.status, expected, report.message);
      assert.equal(report.candidate_source, candidate);
      assert.match(report.spec_sha256, /^[a-f0-9]{64}$/);
      assert.match(report.netlist_sha256, /^[a-f0-9]{64}$/);
      if (!expected) assert.ok(report.evidence.transient_data.length > 100000);
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
