import type { SpiceImportDiagnostic, SpiceImportReport } from './domain';

/** Preserve Core's complete diagnostics; the UI does not reinterpret SPICE. */
export class SpiceImportError extends Error {
  readonly diagnostics: SpiceImportDiagnostic[];

  constructor(diagnostics: SpiceImportDiagnostic[]) {
    const first = diagnostics.find((item) => item.severity === 'error');
    super(first ? importDiagnosticLabel(first) : 'The netlist could not be represented as a complete Kessetsu circuit.');
    this.name = 'SpiceImportError';
    this.diagnostics = diagnostics;
  }
}

export function importDiagnosticLabel(diagnostic: SpiceImportDiagnostic) {
  return `${diagnostic.code}${diagnostic.line ? ` line ${diagnostic.line}` : ''}: ${diagnostic.message}`;
}

const KESSETSU_STATEMENT_PATTERN = /^\s*(?:net|connect|resistor|capacitor|inductor|diode|bjt|mosfet|opamp|module|group|model_include)\b/m;
const SPICE_DIRECTIVE_LINE = /^\s*\.(?:op|ac|tran|dc|end|model|subckt|ends|param|control|title|include|lib|options|meas|print|plot|save|probe)\b/i;
const SPICE_CARD_LINE = /^\s*(?:[RCLVIDQMX][A-Za-z0-9_]*)\s+\S+\s+\S+\s+\S+/i;
const KESSETSU_IDENTIFIER_WORDS = /^\s*(?:resistor|capacitor|inductor|connect|simulate|assert|param)\b/i;

/** Detect whether raw text in the editor or an agent reply looks like a SPICE netlist rather than .kess source. */
export function looksLikeSpiceNetlist(source: string): boolean {
  if (KESSETSU_STATEMENT_PATTERN.test(source)) {
    return false;
  }
  const lines = source.split(/\r?\n/);
  let cardCount = 0;
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('//') || line.startsWith('*')) continue;
    if (KESSETSU_IDENTIFIER_WORDS.test(line)) return false;
    if (SPICE_DIRECTIVE_LINE.test(line)) return true;
    if (SPICE_CARD_LINE.test(line)) {
      cardCount += 1;
      if (cardCount >= 2) return true;
    }
  }
  return false;
}

/** Validate and unwrap a SPICE import report, throwing SpiceImportError if there are errors. */
export function assertValidSpiceReport(report: SpiceImportReport, expectedVersion: string): SpiceImportReport {
  if (report.schema_version !== expectedVersion) {
    throw new Error(`Unsupported SPICE import report: ${report.schema_version}`);
  }
  const errors = report.diagnostics.filter((diagnostic) => diagnostic.severity === 'error');
  if (errors.length > 0 || !report.kess_source) {
    throw new SpiceImportError(report.diagnostics);
  }
  return report;
}

/** Merge a converted SPICE .kess circuit into an existing .kess source without duplicating net or analysis declarations. */
export function mergeConvertedKessIntoSource(currentSource: string, convertedKess: string): string {
  const trimmedCurrent = currentSource.trim();
  if (!trimmedCurrent) return convertedKess;

  const existingNets = new Set<string>();
  const existingAnalysisKinds = new Set<string>();
  for (const line of trimmedCurrent.split(/\r?\n/)) {
    const netMatch = /^\s*net\s+([A-Za-z_][A-Za-z0-9_]*)\s*$/.exec(line);
    if (netMatch) existingNets.add(netMatch[1]);
    const simMatch = /^\s*simulate\s+(op|ac|tran|dc)\b/i.exec(line);
    if (simMatch) existingAnalysisKinds.add(simMatch[1].toLowerCase());
  }

  const netLinesToInsert: string[] = [];
  const bodyLinesToAppend: string[] = [];

  for (const line of convertedKess.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    if (trimmed.startsWith('// Imported from ') || trimmed.startsWith('// Original source:')) {
      continue;
    }
    const netMatch = /^net\s+([A-Za-z_][A-Za-z0-9_]*)$/.exec(trimmed);
    if (netMatch) {
      if (!existingNets.has(netMatch[1])) {
        existingNets.add(netMatch[1]);
        netLinesToInsert.push(`net ${netMatch[1]}`);
      }
      continue;
    }
    const simMatch = /^simulate\s+(op|ac|tran|dc)\b/i.exec(trimmed);
    if (simMatch && existingAnalysisKinds.has(simMatch[1].toLowerCase())) {
      continue;
    }
    bodyLinesToAppend.push(line);
  }

  const currentLines = trimmedCurrent.split(/\r?\n/);
  if (netLinesToInsert.length > 0) {
    let lastNetIndex = -1;
    for (let i = 0; i < currentLines.length; i += 1) {
      if (/^\s*net\s+[A-Za-z_][A-Za-z0-9_]*\s*$/.test(currentLines[i])) {
        lastNetIndex = i;
      }
    }
    if (lastNetIndex >= 0) {
      currentLines.splice(lastNetIndex + 1, 0, ...netLinesToInsert);
    } else {
      let headerEnd = 0;
      while (headerEnd < currentLines.length && /^\s*\/\//.test(currentLines[headerEnd])) {
        headerEnd += 1;
      }
      currentLines.splice(headerEnd, 0, ...netLinesToInsert);
    }
  }

  const merged = [
    currentLines.join('\n'),
    ...(bodyLinesToAppend.length > 0 ? ['// --- Merged from SPICE netlist ---', ...bodyLinesToAppend] : []),
  ].join('\n');
  return `${merged.replace(/\s+$/, '')}\n`;
}

