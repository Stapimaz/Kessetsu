import { describe, expect, it } from 'vitest';
import {
  AGENT_PROPOSAL_SCHEMA,
  assertionOutcome,
  createAgentTask,
  createAgentCorrectionTask,
  describeProposalDiagnostic,
  lineDiff,
  parseAgentProposal,
  summarizeProposalChanges,
} from './agentProposal';

const source = 'net GND\nsource V1 5V\n';

describe('agent proposal contracts', () => {
  it('includes standalone syntax guidance without replacing human requirements', async () => {
    const task = await createAgentTask(source, 'Amplifier', '2 W into 8 Ohm.', '1.3.1', 'kessetsu.compile.v9');
    expect(task.requirements).toBe('2 W into 8 Ohm.');
    expect(task.language.example_source).toContain('assert value(V(OUT)) >= 2.95V');
    expect(task.language.references).toContain('https://kessetsu.com/docs/reference/language/');
    expect(task.instructions.join(' ')).toContain('Do not invent syntax');
  });

  it('carries actual rejection evidence and unchanged request/base into a correction task', async () => {
    const diagnostic = { code: 'KES-P001', severity: 'error' as const, stage: 'parse' as const,
      message: 'expected EOI or top_level', line: 4, column: 1 };
    const original = await createAgentTask(source, 'Divider', '3 V within 50 mV.', '1.3.1', 'kessetsu.compile.v9');
    const task = await createAgentCorrectionTask(source, 'Divider', original.requirements, '1.3.1', 'kessetsu.compile.v9',
      '{bad reply}', 'Rejected by Core.', [diagnostic], null);
    expect(task.circuit).toEqual(original.circuit);
    expect(task.expected_response).toEqual(original.expected_response);
    expect(task.requirements).toBe(original.requirements);
    expect(task.correction).toEqual({ rejected_reply: '{bad reply}', failure_message: 'Rejected by Core.', diagnostics: [diagnostic], assertions: null });
    expect(describeProposalDiagnostic(diagnostic)).toContain('Line 4, column 1');
    expect(task.instructions.join(' ')).toContain('Do not remove assertions');
  });

  it('binds tasks and proposals to the exact source revision', async () => {
    const task = await createAgentTask(source, 'Divider', 'Keep OUT below 3.3 V.', '1.2.0', 'kessetsu.compile.v9');
    const proposal = await parseAgentProposal(JSON.stringify({
      schema_version: AGENT_PROPOSAL_SCHEMA,
      base_source_sha256: task.circuit.source_sha256,
      proposed_source: `${source}resistor R1 1k\n`,
      summary: 'Added the load resistor.',
    }), source);
    expect(proposal.proposed_source).toContain('R1');
    await expect(parseAgentProposal(JSON.stringify({ ...proposal, base_source_sha256: '0'.repeat(64) }), source))
      .rejects.toThrow(/different circuit revision/);
  });

  it('rejects malformed and incomplete proposals', async () => {
    await expect(parseAgentProposal('{}', source)).rejects.toThrow(/schema/);
    await expect(parseAgentProposal('{', source)).rejects.toThrow(/valid JSON/);
  });

  it('produces stable line additions and removals', () => {
    expect(lineDiff('a\nb\nc', 'a\nx\nc')).toEqual([
      { kind: 'same', text: 'a', oldLine: 1, newLine: 1 },
      { kind: 'add', text: 'x', oldLine: null, newLine: 2 },
      { kind: 'remove', text: 'b', oldLine: 2, newLine: null },
      { kind: 'same', text: 'c', oldLine: 3, newLine: 3 },
    ]);
  });

  it('does not treat missing or skipped assertions as verified', () => {
    expect(assertionOutcome({ total: 0, passed: 0, failed: 0, errors: 0, skipped: 0 })).toBe('unchecked');
    expect(assertionOutcome({ total: 2, passed: 2, failed: 0, errors: 0, skipped: 0 })).toBe('passed');
    expect(assertionOutcome({ total: 2, passed: 1, failed: 0, errors: 0, skipped: 1 })).toBe('not_passed');
  });

  it('summarizes high-impact proposal changes separately', () => {
    const summary = summarizeProposalChanges(lineDiff(
      'source V1 5V\nresistor R1 1k\nsimulate op\nassert max(V(out)) < 5V',
      'source V1 12V\nresistor R1 2k\nresistor R2 1k\nsimulate tran 1us 1ms\nassert max(V(out)) < 3.3V',
    ));
    expect(summary.sources).toEqual({ added: 1, removed: 1 });
    expect(summary.components).toEqual({ added: 2, removed: 1 });
    expect(summary.analyses).toEqual({ added: 1, removed: 1 });
    expect(summary.assertions).toEqual({ added: 1, removed: 1 });
  });
});
