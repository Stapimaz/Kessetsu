import { test } from 'node:test';
import assert from 'node:assert/strict';
import { verificationScope, verificationTasks } from './verification-scope.mjs';

test('every frozen unseen task has an explicit machine-readable verification boundary', () => {
  assert.deepEqual(verificationTasks, ['U1', 'U2', 'U3', 'U4', 'U5', 'U6']);
  for (const task of verificationTasks) {
    const scope = verificationScope(task, { z_check: true, a_check: true });
    assert.equal(scope.schema_version, 'kessetsu.evaluation-verification-scope.v1');
    assert.equal(scope.specification_id, `unseen-design-v1/${task}`);
    assert.equal(scope.conclusion, 'requirements_satisfied_within_scope');
    assert.deepEqual(scope.requirement_checks, ['a_check', 'z_check']);
    assert.equal(scope.hardware_validated, false);
    for (const field of ['operating_conditions', 'model_assumptions', 'untested_effects']) {
      assert.ok(scope[field].length > 0 && scope[field].every((entry) => typeof entry === 'string' && entry.length > 0));
    }
  }
});

test('a failed check cannot produce a satisfied conclusion and malformed calls fail closed', () => {
  assert.equal(verificationScope('U5', { fixed_requirements: true, output_power: false }).conclusion,
    'requirements_not_satisfied_within_scope');
  assert.throws(() => verificationScope('U7', { check: true }), /Invalid/);
  assert.throws(() => verificationScope('U1', { check: 'yes' }), /Invalid/);
});
