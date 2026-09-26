import { MAX_DOCUMENT_SOURCE_BYTES } from './document';

export const AGENT_TASK_SCHEMA = 'kessetsu.agent-task.v1' as const;
export const AGENT_PROPOSAL_SCHEMA = 'kessetsu.agent-proposal.v1' as const;
export const MAX_AGENT_REQUIREMENTS_LENGTH = 12_000;
export const MAX_AGENT_SUMMARY_LENGTH = 4_000;
export const MAX_AGENT_PROPOSAL_BYTES = MAX_DOCUMENT_SOURCE_BYTES + 64 * 1024;

export interface AgentTaskEnvelope {
  schema_version: typeof AGENT_TASK_SCHEMA;
  product: { name: 'Kessetsu'; version: string; compile_contract: string };
  workflow: { cli: 'kess'; discovery_command: 'kess capabilities --format json' };
  circuit: { name: string; source: string; source_sha256: string };
  requirements: string;
  expected_response: {
    schema_version: typeof AGENT_PROPOSAL_SCHEMA;
    base_source_sha256: string;
    proposed_source: string;
    summary: string;
  };
  instructions: string[];
}

export interface AgentProposalEnvelope {
  schema_version: typeof AGENT_PROPOSAL_SCHEMA;
  base_source_sha256: string;
  proposed_source: string;
  summary: string;
}

export interface DiffLine {
  kind: 'same' | 'remove' | 'add';
  text: string;
  oldLine: number | null;
  newLine: number | null;
}

function utf8Length(value: string) {
  return new TextEncoder().encode(value).byteLength;
}

export async function sha256Text(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const hash = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(hash), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

export async function createAgentTask(
  source: string,
  name: string,
  requirements: string,
  version: string,
  compileContract: string,
): Promise<AgentTaskEnvelope> {
  const normalizedRequirements = requirements.trim();
  if (!normalizedRequirements) throw new Error('Describe the requirements before creating an agent task');
  if (normalizedRequirements.length > MAX_AGENT_REQUIREMENTS_LENGTH) {
    throw new Error(`Requirements are limited to ${MAX_AGENT_REQUIREMENTS_LENGTH.toLocaleString()} characters`);
  }
  if (utf8Length(source) > MAX_DOCUMENT_SOURCE_BYTES) throw new Error('Circuit source exceeds the browser document limit');
  const sourceHash = await sha256Text(source);
  return {
    schema_version: AGENT_TASK_SCHEMA,
    product: { name: 'Kessetsu', version, compile_contract: compileContract },
    workflow: { cli: 'kess', discovery_command: 'kess capabilities --format json' },
    circuit: { name, source, source_sha256: sourceHash },
    requirements: normalizedRequirements,
    expected_response: {
      schema_version: AGENT_PROPOSAL_SCHEMA,
      base_source_sha256: sourceHash,
      proposed_source: '<complete revised .kess source>',
      summary: '<what changed, assumptions, and what should be verified>',
    },
    instructions: [
      'Return one JSON object matching expected_response; do not wrap it in Markdown.',
      'Keep the human requirements unchanged and put the complete proposed .kess source in proposed_source.',
      'Use Kessetsu checks, simulations, studies, and exports when available; never invent a PASS result.',
      'State model and physical-validation limits in summary. The user will review, simulate, and accept or reject the proposal.',
    ],
  };
}

function objectValue(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Proposal must be a JSON object');
  return value as Record<string, unknown>;
}

export async function parseAgentProposal(text: string, currentSource: string): Promise<AgentProposalEnvelope> {
  if (utf8Length(text) > MAX_AGENT_PROPOSAL_BYTES) throw new Error('Proposal file is too large');
  let parsed: Record<string, unknown>;
  try { parsed = objectValue(JSON.parse(text.replace(/^\uFEFF/, ''))); }
  catch (cause) { throw new Error(`Proposal is not valid JSON: ${cause instanceof Error ? cause.message : String(cause)}`); }
  if (parsed.schema_version !== AGENT_PROPOSAL_SCHEMA) {
    throw new Error(`Unsupported proposal schema. Expected ${AGENT_PROPOSAL_SCHEMA}`);
  }
  if (typeof parsed.base_source_sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(parsed.base_source_sha256)) {
    throw new Error('Proposal base_source_sha256 must be a lowercase SHA-256 digest');
  }
  const currentHash = await sha256Text(currentSource);
  if (parsed.base_source_sha256 !== currentHash) {
    throw new Error('This proposal targets a different circuit revision. Create a new task from the current source.');
  }
  if (typeof parsed.proposed_source !== 'string' || !parsed.proposed_source.trim()) {
    throw new Error('Proposal must contain the complete proposed_source');
  }
  if (utf8Length(parsed.proposed_source) > MAX_DOCUMENT_SOURCE_BYTES) {
    throw new Error('Proposed source exceeds the 1 MiB browser document limit');
  }
  if (typeof parsed.summary !== 'string' || !parsed.summary.trim()) throw new Error('Proposal must include a summary');
  if (parsed.summary.length > MAX_AGENT_SUMMARY_LENGTH) {
    throw new Error(`Proposal summary is limited to ${MAX_AGENT_SUMMARY_LENGTH.toLocaleString()} characters`);
  }
  return {
    schema_version: AGENT_PROPOSAL_SCHEMA,
    base_source_sha256: parsed.base_source_sha256,
    proposed_source: parsed.proposed_source,
    summary: parsed.summary.trim(),
  };
}

function fallbackDiff(before: string[], after: string[]): DiffLine[] {
  let prefix = 0;
  while (prefix < before.length && prefix < after.length && before[prefix] === after[prefix]) prefix++;
  let suffix = 0;
  while (suffix < before.length - prefix && suffix < after.length - prefix
    && before[before.length - 1 - suffix] === after[after.length - 1 - suffix]) suffix++;
  const rows: DiffLine[] = [];
  for (let index = 0; index < prefix; index++) rows.push({ kind: 'same', text: before[index], oldLine: index + 1, newLine: index + 1 });
  for (let index = prefix; index < before.length - suffix; index++) rows.push({ kind: 'remove', text: before[index], oldLine: index + 1, newLine: null });
  for (let index = prefix; index < after.length - suffix; index++) rows.push({ kind: 'add', text: after[index], oldLine: null, newLine: index + 1 });
  for (let offset = suffix; offset > 0; offset--) {
    const oldIndex = before.length - offset;
    const newIndex = after.length - offset;
    rows.push({ kind: 'same', text: before[oldIndex], oldLine: oldIndex + 1, newLine: newIndex + 1 });
  }
  return rows;
}

export function lineDiff(beforeSource: string, afterSource: string): DiffLine[] {
  const before = beforeSource.replace(/\r\n/g, '\n').split('\n');
  const after = afterSource.replace(/\r\n/g, '\n').split('\n');
  if (before.length * after.length > 250_000) return fallbackDiff(before, after);
  const width = after.length + 1;
  const table = new Uint32Array((before.length + 1) * width);
  for (let oldIndex = before.length - 1; oldIndex >= 0; oldIndex--) {
    for (let newIndex = after.length - 1; newIndex >= 0; newIndex--) {
      table[oldIndex * width + newIndex] = before[oldIndex] === after[newIndex]
        ? table[(oldIndex + 1) * width + newIndex + 1] + 1
        : Math.max(table[(oldIndex + 1) * width + newIndex], table[oldIndex * width + newIndex + 1]);
    }
  }
  const rows: DiffLine[] = [];
  let oldIndex = 0;
  let newIndex = 0;
  while (oldIndex < before.length || newIndex < after.length) {
    if (oldIndex < before.length && newIndex < after.length && before[oldIndex] === after[newIndex]) {
      rows.push({ kind: 'same', text: before[oldIndex], oldLine: oldIndex + 1, newLine: newIndex + 1 });
      oldIndex++; newIndex++;
    } else if (newIndex < after.length && (oldIndex === before.length
      || table[oldIndex * width + newIndex + 1] >= table[(oldIndex + 1) * width + newIndex])) {
      rows.push({ kind: 'add', text: after[newIndex], oldLine: null, newLine: newIndex + 1 });
      newIndex++;
    } else {
      rows.push({ kind: 'remove', text: before[oldIndex], oldLine: oldIndex + 1, newLine: null });
      oldIndex++;
    }
  }
  return rows;
}
