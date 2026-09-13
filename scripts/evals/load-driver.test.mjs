import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { MODEL, inspectU4, settledMetrics, evaluateU4 } from './load-driver.mjs';

const valid = `U4 fixture
VG gate 0 PULSE(0 10 0 1u 1u 500u 1m)
VP rail 0 12
RL rail drain 120
M1 drain gate 0 0 IRF540
${MODEL}
.end
`;
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('agent harness distributes the exact evaluator IRF540 model', () => {
  const distributed = readFileSync(fileURLToPath(new URL('./models/IRF540.lib', import.meta.url)), 'utf8').trim();
  assert.equal(distributed, MODEL);
});

test('U4 rejects source/load/model/topology tampering and accepts bounded gate networks', () => {
  assert.equal(inspectU4(valid).gateSeries, null);
  const withGate = valid.replace('VG gate', 'VG in').replace('M1 drain gate', 'RG in gate 100\nRPD gate 0 100k\nM1 drain gate');
  assert.deepEqual(inspectU4(withGate), { gateSeries: 100, gatePullDown: 100000 });
  for (const candidate of [valid.replace('VP rail 0 12', 'VP rail 0 9'), valid.replace('RL rail drain 120', 'RL rail drain 100'),
    valid.replace('500u 1m', '400u 1m'), valid.replace('Vto=4.0', 'Vto=2'), valid.replace('M1 drain gate 0 0', 'M1 gate drain 0 0'),
    valid.replace('.end\n', 'C1 gate 0 1n\n.end\n'), valid.replace('VG gate', '.end\nVG gate'),
    withGate.replace('RPD gate 0 100k', 'RPD drain 0 100k'), withGate.replace('RPD gate 0 100k', 'RPD gate 0 100k\nRPD2 gate 0 100k')]) assert.throws(() => inspectU4(candidate));
  for (const directive of ['.print tran v(drain)', '.plot tran v(drain)', '.save v(drain)']) {
    assert.deepEqual(inspectU4(valid.replace('.end\n', `${directive}\n.end\n`)), inspectU4(valid));
  }
});

test('settled U4 metrics are time weighted, exclude transitions, and enforce KCL/data integrity', () => {
  const rows = Array.from({ length: 5001 }, (_, i) => {
    const time = i * 2e-6, phase = time % 1e-3;
    const on = phase <= 500e-6;
    const drain = on ? 0.2 : 12;
    return [time, drain, (12 - drain) / 120];
  });
  const result = settledMetrics(rows);
  assert.ok(result.on_current_a > 0.098);
  assert.equal(result.off_current_peak_a, 0);
  assert.ok(result.mean_dissipation_w < 0.02);
  assert.throws(() => settledMetrics(rows.slice(0, -100)));
  assert.throws(() => settledMetrics(rows.filter((_, i) => i % 2 === 0)));
  assert.throws(() => settledMetrics(rows.map((row, i) => i === 4100 ? [row[0], row[1], row[2] + 0.01] : row)));
});

test('real U4 Ngspice run passes fixed settled requirements and records exclusions', () => {
  const result = evaluateU4(valid, simulator);
  assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
  assert.equal(result.model_adequacy.hardware_validated, false);
  assert.match(result.model_adequacy.limitations.join(' '), /excludes 50 us/);
  const slowGate = valid.replace('VG gate', 'VG in').replace('M1 drain gate', 'RG in gate 100Meg\nM1 drain gate');
  assert.equal(evaluateU4(slowGate, simulator).status, 'FAIL');
});

test('U4 retains failed simulator evidence', () => {
  assert.throws(() => evaluateU4(valid, 'fake', () => ({ status: 1, stdout: 'partial', stderr: 'failure', signal: null })),
    (error) => error.evidence.simulator_stderr === 'failure' && error.evidence.tran_data_error === 'ENOENT');
});

test('U4 CLI uses the same evaluator-owned testbench for direct and Kessetsu candidates', () => {
  const source = `net GND
net GATE
net RAIL
net DRAIN
source VG pulse(0V,10V,0s,1us,1us,500us,1ms)
source VP 12V
resistor RL 120
mosfet M1 IRF540
connect VG.minus,VP.minus,M1.s to GND
connect VG.plus,M1.g to GATE
connect VP.plus,RL.p1 to RAIL
connect RL.p2,M1.d to DRAIN
`;
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u4-contract-'));
  try {
    for (const [arm, candidate] of [['direct', valid], ['kessetsu', source]]) {
      const path = join(directory, 'candidate'); writeFileSync(path, candidate);
      const run = spawnSync(process.execPath, [fileURLToPath(new URL('./load-driver.mjs', import.meta.url)), arm, path], { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024, timeout: 60000, windowsHide: true });
      const report = JSON.parse(run.stdout);
      assert.equal(run.status, 0, report.message ?? JSON.stringify(report.measurements));
      assert.equal(report.schema_version, 'kessetsu.u4-evaluation.v2');
      assert.equal(report.candidate_source, candidate);
      assert.equal(report.status, 'PASS');
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
