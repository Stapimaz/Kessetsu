import { describe, expect, it } from 'vitest';
import { AGENT_PROPOSAL_SCHEMA, createAgentTask, lineDiff, parseAgentProposal } from './agentProposal';

const source = 'net GND\nsource V1 5V\n';

describe('agent proposal contracts', () => {
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
});
