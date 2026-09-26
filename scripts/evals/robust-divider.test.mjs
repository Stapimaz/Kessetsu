import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { inspectDivider, evaluateCorners, inspectWorkflow } from './robust-divider.mjs';

const netlist = (upper = '30k', lower = '12k', load = '100k') => `M1 candidate
VIN in 0 12
RUP in out ${upper}
RLOW out 0 ${lower}
RL out 0 ${load}
.end
`;

test('accepts the robust E24 candidate across all 27 independent cases', () => {
  const result = evaluateCorners(inspectDivider(netlist()));
  assert.equal(result.status, 'PASS');
  assert.equal(result.cases.length, 27);
  assert.ok(result.measurements.output_min_v >= 2.75);
  assert.ok(result.measurements.output_max_v <= 3.60);
});

test('keeps a valid topology but fails a design outside the output range', () => {
  const result = evaluateCorners(inspectDivider(netlist('10k', '10k')));
  assert.equal(result.status, 'FAIL');
  assert.equal(result.checks.output_range, false);
});

test('rejects changed loads and non-E24 design values', () => {
  assert.throws(() => inspectDivider(netlist('30k', '12k', '90k')), /100 kohm load/);
  assert.throws(() => inspectDivider(netlist('31k', '12k')), /E24 values/);
});

test('accepts BOM-prefixed study JSON and candidate-prefixed export names', (context) => {
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-m1-evaluator-'));
  context.after(() => import('node:fs').then(({ rmSync }) => rmSync(directory, { recursive: true, force: true })));
  const candidate = join(directory, 'candidate.kess');
  writeFileSync(candidate, 'placeholder');
  const study = {
    schema_version: 'kessetsu.experiment-results.v1',
    summary: { total: 27, passed: 27 },
    cases: Array.from({ length: 27 }, (_, index) => ({ id: index, status: 'passed' })),
  };
  writeFileSync(join(directory, 'study-results.json'), `\uFEFF${JSON.stringify(study)}`);
  writeFileSync(join(directory, 'candidate.svg'), '<svg/>');
  writeFileSync(join(directory, 'candidate.png'), 'png');
  writeFileSync(join(directory, 'candidate.bom.csv'), 'reference,value');
  writeFileSync(join(directory, 'candidate.handoff.json'), '{}');
  mkdirSync(join(directory, 'revisions'));
  writeFileSync(join(directory, 'revisions', 'first.kess'), 'revision');

  const result = inspectWorkflow(candidate, 'kessetsu');
  assert.equal(result.study.valid, true);
  assert.equal(result.files['schematic.svg'], true);
  assert.equal(result.files['schematic.png'], true);
  assert.equal(result.files['bom.csv'], true);
  assert.equal(result.files['handoff.json'], true);
  assert.equal(result.revisions, 1);
});
