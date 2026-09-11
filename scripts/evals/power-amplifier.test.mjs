import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { MODEL as OPAMP_MODEL } from './active-filter.mjs';
import { NPN_MODEL, PNP_MODEL, inspectU5, measureU5, evaluateU5 } from './power-amplifier.mjs';

const valid = `U5 fixture
VP vcc 0 9
VN 0 vee 9
VIN in 0 SIN(0 0.1 1000) AC 1
X1 in buf vcc vee buf KESSETSU_OPAMP_V1
X2 buf feedback vcc vee gain KESSETSU_OPAMP_V1
RF gain feedback 27.5k
RG feedback 0 1k
X3 gain out vcc vee drive KESSETSU_OPAMP_V1
QN vcc drive out KESSETSU_POWER_NPN_V1
QP vee drive out KESSETSU_POWER_PNP_V1
RL out 0 4
${OPAMP_MODEL}
${NPN_MODEL}
${PNP_MODEL}
.end
`;
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('U5 locks sources, load, models, complementary stage, and bounded gain/driver topology', () => {
  assert.equal(inspectU5(valid).rf, 27500);
  for (const candidate of [valid.replace('VN 0 vee 9', 'VN 0 vee 12'), valid.replace('RL out 0 4', 'RL out 0 8'),
    valid.replace('0.1 1000', '0.2 1000'), valid.replace('Bf=80', 'Bf=800'), valid.replace('QP vee drive out', 'QP vcc drive out'),
    valid.replace('X3 gain out', 'X3 gain feedback'), valid.replace('X1 in buf', 'X1 gain buf'),
    valid.replace('RG feedback 0', 'RG feedback vee'), valid.replace('.end\n', 'C1 out 0 1n\n.end\n'),
    valid.replace('VP vcc', '.end\nVP vcc'), valid.replace('VIN in 0', 'VIN out 0')]) assert.throws(() => inspectU5(candidate));
});

test('U5 metric integration recovers RMS power, THD, and positive device dissipation', () => {
  const rows = Array.from({ length: 15001 }, (_, i) => {
    const t = i * 2e-6, v = Math.sqrt(8) * Math.sin(2 * Math.PI * 1000 * t) + 0.028284271 * Math.sin(4 * Math.PI * 1000 * t);
    return [t, v, Math.max(0, v / 4), Math.min(0, v / 4)];
  });
  const result = measureU5(rows);
  assert.ok(Math.abs(result.output_power_w - 1.0001) < 1e-4);
  assert.ok(Math.abs(result.thd - 0.01) < 1e-5);
  assert.ok(result.npn_dissipation_w > 0 && result.pnp_dissipation_w > 0);
  assert.throws(() => measureU5(rows.slice(0, -20)));
  assert.throws(() => measureU5(rows.filter((_, i) => i % 2 === 0)));
  assert.throws(() => measureU5([...rows].reverse()));
});

test('real U5 simulation meets frozen electrical requirements without claiming total efficiency', () => {
  const result = evaluateU5(valid, simulator);
  assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
  assert.equal(result.driver_power.status, 'UNAVAILABLE');
  assert.equal(result.model_adequacy.hardware_validated, false);
  const lowGain = evaluateU5(valid.replace('27.5k', '10k'), simulator);
  assert.equal(lowGain.status, 'FAIL');
  assert.equal(lowGain.checks.output_power, false);
});

test('U5 retains simulator failure evidence', () => {
  assert.throws(() => evaluateU5(valid, 'fake', () => ({ status: 1, stdout: 'partial', stderr: 'failed', signal: null })),
    (error) => error.evidence.simulator_stderr === 'failed' && error.evidence.tran_data_error === 'ENOENT');
});

test('U5 CLI evaluates equivalent direct and Core candidates with external requirements', () => {
  const source = `net GND
net VCC
net VEE
net IN
net BUF
net GAIN
net FB
net DRIVE
net OUT
source VP 9V
source VN 9V
source VIN sine_ac(0V,100mV,1kHz,1V)
opamp U1 KESSETSU_OPAMP_V1
opamp U2 KESSETSU_OPAMP_V1
opamp U3 KESSETSU_OPAMP_V1
resistor RF 27.5k
resistor RG 1k
transistor QN npn KESSETSU_POWER_NPN_V1
transistor QP pnp KESSETSU_POWER_PNP_V1
resistor RL 4
connect VP.minus,VN.plus,VIN.minus,RG.p2,RL.p2 to GND
connect VP.plus,U1.vcc,U2.vcc,U3.vcc,QN.c to VCC
connect VN.minus,U1.vee,U2.vee,U3.vee,QP.c to VEE
connect VIN.plus,U1.in_p to IN
connect U1.in_n,U1.out,U2.in_p to BUF
connect U2.in_n,RF.p2,RG.p1 to FB
connect U2.out,RF.p1,U3.in_p to GAIN
connect U3.in_n,QN.e,QP.e,RL.p1 to OUT
connect U3.out,QN.b,QP.b to DRIVE
`;
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u5-contract-'));
  try {
    for (const [arm, candidate] of [['direct', valid], ['kessetsu', source]]) {
      const path = join(directory, 'candidate'); writeFileSync(path, candidate);
      const run = spawnSync(process.execPath, [fileURLToPath(new URL('./power-amplifier.mjs', import.meta.url)), arm, path], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: 60000, windowsHide: true });
      const report = JSON.parse(run.stdout);
      assert.equal(run.status, 0, report.message ?? JSON.stringify(report.measurements));
      assert.equal(report.candidate_source, candidate);
      assert.equal(report.status, 'PASS');
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
