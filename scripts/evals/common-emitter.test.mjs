import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { MODEL, inspectU3, fundamental, evaluateU3 } from './common-emitter.mjs';

// Developer fixtures; not comparative agent evidence.
const valid = `U3 fixture
VIN in 0 SIN(0 0.005 1000) AC 1
VP rail 0 9
RC rail collector 3.3k
RE emitter 0 270
RT rail base 75k
RB base 0 10k
RL out 0 10k
CI in base 10u
CO collector out 10u
Q1 collector base emitter 2N3904
${MODEL}
.end
`;
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('agent harness distributes the exact evaluator 2N3904 model', () => {
  const distributed = readFileSync(fileURLToPath(new URL('./models/2N3904.lib', import.meta.url)), 'utf8').trim();
  assert.equal(distributed, MODEL);
});

test('U3 requires immutable bias topology, source, load and generic transistor model', () => {
  assert.equal(inspectU3(valid).rc, 3300);
  for (const candidate of [valid.replace('VP rail 0 9', 'VP rail 0 12'), valid.replace('RL out 0 10k', 'RL out 0 100k'),
    valid.replace('0.005 1000', '0.001 1000'), valid.replace('Bf=416.4', 'Bf=999'), valid.replace('CO collector out', 'CO base out'),
    valid.replace('Q1 collector base emitter', 'Q1 emitter base collector'), valid.replace('VIN in', '.end\nVIN in'),
    valid.replace('.end\n', 'E1 out 0 in 0 -10\n.end\n')]) assert.throws(() => inspectU3(candidate));
  assert.deepEqual(inspectU3(valid.replace('.end\n', '.control\necho FALSE PASS\n.endc\n.end\n')), inspectU3(valid));
  for (const directive of ['.print ac vm(out)', '.plot tran v(out)', '.save v(out)']) {
    assert.deepEqual(inspectU3(valid.replace('.end\n', `${directive}\n.end\n`)), inspectU3(valid));
  }
});

test('Fourier integration recovers fundamental despite DC/harmonics and rejects invalid windows', () => {
  const rows = Array.from({ length: 10002 }, (_, i) => {
    const time = i * 1e-6;
    return [time, 0.2 + 0.05 * Math.sin(2 * Math.PI * 1000 * time + 0.4) + 0.02 * Math.sin(2 * Math.PI * 3000 * time)];
  });
  assert.ok(Math.abs(fundamental(rows, 0.0050003, 0.0100003, 1000) - 0.05) < 1e-6);
  assert.throws(() => fundamental(rows.slice(0, 9000), 0.005, 0.01, 1000));
  assert.throws(() => fundamental([...rows].reverse(), 0.005, 0.01, 1000));
  assert.throws(() => fundamental(rows.filter((_, i) => i % 3 === 0), 0.005, 0.01, 1000));
  assert.throws(() => fundamental(rows, 0.005, 0.0095, 1000));
});

test('real U3 simulation checks DC KCL and independent transient fundamental', () => {
  const result = evaluateU3(valid, simulator);
  assert.equal(result.status, 'PASS', JSON.stringify(result.measurements));
  assert.equal(result.checks.independent_dc_kcl, true);
  assert.equal(result.model_adequacy.hardware_validated, false);
  const wrong = evaluateU3(valid.replace('RE emitter 0 270', 'RE emitter 0 1k'), simulator);
  assert.equal(wrong.status, 'FAIL');
  assert.equal(wrong.checks.gain, false);
});

test('U3 preserves process and malformed-dataset evidence', () => {
  assert.throws(() => evaluateU3(valid, 'fake-test-runner', (_exe, _args, options) => {
    for (const name of ['op', 'ac', 'tran']) writeFileSync(join(options.cwd, `${name}.data`), 'bad data');
    return { status: 0, stdout: 'stdout retained', stderr: 'stderr retained', signal: null };
  }), (error) => error.evidence.op_data === 'bad data' && error.evidence.simulator_stderr === 'stderr retained');
});

test('U3 CLI evaluates direct and Core-compiled candidates under the same fixed testbench', () => {
  const source = `net GND
net IN
net RAIL
net COLLECTOR
net BASE
net EMITTER
net OUT
source VIN sine_ac(0V,5mV,1kHz,1V)
source VP 9V
transistor Q1 npn 2N3904
resistor RC 3.3k
resistor RE 270
resistor RT 75k
resistor RB 10k
resistor RL 10k
capacitor CI 10u
capacitor CO 10u
connect VIN.minus,VP.minus,RE.p2,RB.p2,RL.p2 to GND
connect VIN.plus,CI.p1 to IN
connect VP.plus,RC.p1,RT.p1 to RAIL
connect RC.p2,Q1.c,CO.p1 to COLLECTOR
connect RT.p2,RB.p1,CI.p2,Q1.b to BASE
connect RE.p1,Q1.e to EMITTER
connect CO.p2,RL.p1 to OUT
`;
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u3-contract-'));
  try {
    for (const [arm, candidate] of [['direct', valid], ['kessetsu', source]]) {
      const path = join(directory, 'candidate'); writeFileSync(path, candidate);
      const run = spawnSync(process.execPath, [fileURLToPath(new URL('./common-emitter.mjs', import.meta.url)), arm, path], { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024, timeout: 60000, windowsHide: true });
      const report = JSON.parse(run.stdout);
      assert.equal(run.status, 0, report.message ?? JSON.stringify(report.measurements));
      assert.equal(report.schema_version, 'kessetsu.u3-evaluation.v2');
      assert.equal(report.candidate_source, candidate);
      assert.match(report.model_adequacy.model_sha256, /^[a-f0-9]{64}$/);
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
