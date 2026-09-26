import test from 'node:test';
import assert from 'node:assert/strict';
import { inspectDivider, evaluateCorners } from './robust-divider.mjs';

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
