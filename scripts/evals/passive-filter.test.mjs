import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { inspectU1, spiceNumber, cutoffFrequency, parseRows, evaluateU1 } from './passive-filter.mjs';

// Evaluator fixtures, not agent-designed candidates or comparative trial evidence.
const valid = 'Test circuit\nV1 in 0 AC 1\nR1 in out 2k\nRL out 0 100k\nC1 out 0 50n\n.end\n';
const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32'
  ? fileURLToPath(new URL('../../core/tools/ngspice/bin/ngspice_con.exe', import.meta.url)) : 'ngspice');

test('interprets SPICE suffixes and rejects malformed/non-finite values', () => {
  assert.equal(spiceNumber('2M'), 0.002);
  assert.equal(spiceNumber('2Meg'), 2e6);
  for (const value of ['1e999', 'NaN', '2garbage', '{x}', '1;quit']) assert.throws(() => spiceNumber(value));
});

test('enforces immutable topology/load/source regardless of candidate assertions', () => {
  const circuit = inspectU1(valid);
  assert.equal(circuit.resistance, 2000);
  assert.equal(circuit.load, 100000);
  assert.ok(Math.abs(circuit.capacitance - 50e-9) < 1e-20);
  for (const tampered of [valid.replace('100k', '50k'), valid.replace('AC 1', 'AC 2'),
    valid.replace('R1 in out', 'R1 in 0'), valid.replace('2k', '20k'),
    valid.replace('.end', '.include secrets.model\n.end'), valid.replace('.end', 'R2 in out 1k\n.end'),
    valid.replace('V1 in', '.end\nV1 in'), `${valid}R2 in out 1k\n`, valid.replace('.end', '.end ignored')]) {
    assert.throws(() => inspectU1(tampered));
  }
  assert.deepEqual(inspectU1(valid.replace('.end', '.control\necho FAKE PASS\nquit\n.endc\n.end')), inspectU1(valid));
  for (const directive of ['.print ac vm(out)', '.plot ac vm(out)', '.save v(out)']) {
    assert.deepEqual(inspectU1(valid.replace('.end', `${directive}\n.end`)), inspectU1(valid));
  }
  assert.throws(() => inspectU1(valid.replace('.end', '.include arbitrary.lib\n.end')));
});

test('both candidate frontends produce reproducible evidence; load tampering errors', () => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u1-contract-'));
  const evaluator = fileURLToPath(new URL('./passive-filter.mjs', import.meta.url));
  const source = `net GND\nnet IN\nnet OUT\nsource VIN ac(1V)\nresistor R1 2k\nresistor RL 100k\ncapacitor C1 50n\nconnect VIN.minus to GND\nconnect VIN.plus,R1.p1 to IN\nconnect R1.p2,RL.p1,C1.p1 to OUT\nconnect RL.p2,C1.p2 to GND\n`;
  try {
    for (const [arm, candidate] of [['direct', valid], ['kessetsu', source]]) {
      const path = join(directory, `candidate-${arm}`);
      writeFileSync(path, candidate);
      const run = spawnSync(process.execPath, [evaluator, arm, path], { encoding: 'utf8', windowsHide: true });
      assert.equal(run.status, 0, run.stdout + run.stderr);
      const report = JSON.parse(run.stdout);
      assert.equal(report.status, 'PASS');
      assert.match(report.spec_sha256, /^[0-9a-f]{64}$/);
      assert.equal(report.candidate_source, candidate);
      assert.ok(report.evidence.ac_data.length > 1000);
    }
    const tampered = join(directory, 'tampered.spice');
    writeFileSync(tampered, valid.replace('100k', '50k'));
    const run = spawnSync(process.execPath, [evaluator, 'direct', tampered], { encoding: 'utf8', windowsHide: true });
    assert.equal(run.status, 2);
    assert.equal(JSON.parse(run.stdout).status, 'ERROR');
    assert.equal(JSON.parse(run.stdout).candidate_source, valid.replace('100k', '50k'));
    const failed = spawnSync(process.execPath, [evaluator, 'direct', join(directory, 'candidate-direct')], {
      encoding: 'utf8', windowsHide: true, env: { ...process.env, KESSETSU_NGSPICE: join(directory, 'missing-simulator') } });
    const failure = JSON.parse(failed.stdout);
    assert.equal(failed.status, 2);
    assert.match(failure.evidence.simulator_error, /ENOENT/);
    assert.ok(failure.evidence.testbench.startsWith('Evaluator-owned U1'));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('U1 retains partial datasets and process errors even when parsing fails', () => {
  for (const status of [0, 1]) {
    assert.throws(() => evaluateU1(valid, 'fake-test-runner', (_exe, _args, options) => {
      writeFileSync(join(options.cwd, 'op.data'), 'malformed result');
      return { status, stdout: 'partial stdout', stderr: 'diagnostic stderr', signal: null };
    }), (error) => error.evidence.op_data === 'malformed result' && error.evidence.ac_data_error === 'ENOENT' &&
      error.evidence.simulator_stdout === 'partial stdout' && error.evidence.simulator_exit_code === status);
  }
});

test('checks data integrity and cutoff against independent analytic samples', () => {
  assert.throws(() => parseRows('1 2 NaN', 3));
  assert.throws(() => parseRows('', 2));
  const rows = Array.from({ length: 501 }, (_, index) => {
    const frequency = 10 ** (1 + index / 100);
    const normalized = frequency / 1600;
    return [frequency, 1 / (1 + normalized ** 2), -normalized / (1 + normalized ** 2), frequency, 1, 0];
  });
  assert.ok(Math.abs(cutoffFrequency(rows, 1) / 1600 - 1) < 0.001);
  assert.throws(() => cutoffFrequency([...rows].reverse(), 1));
  assert.throws(() => cutoffFrequency(rows.map((row) => [row[0], 1, 0, row[0], 1, 0]), 1));
});

test('runs evaluator-owned Ngspice and rejects a wrong cutoff without candidate assertions', () => {
  const result = evaluateU1(valid, simulator);
  assert.equal(result.status, 'PASS');
  assert.ok(Math.abs(result.measurements.cutoff_hz - 1623.38) < 2);
  assert.equal(result.checks.independent_formula_agreement, true);
  const failed = evaluateU1(valid.replace('50n', '500n'), simulator);
  assert.equal(failed.status, 'FAIL');
  assert.equal(failed.checks.cutoff, false);
  assert.equal(failed.checks.independent_formula_agreement, true);
});
