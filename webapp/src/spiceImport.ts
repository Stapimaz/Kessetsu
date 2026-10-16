import { import_spice_netlist, spice_import_schema_version } from 'kessetsu-core';
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

const SPICE_DIRECTIVE_LINE = /^\s*\.(?:op|ac|tran|dc|end|model|subckt|ends|param|control|title|include|lib|options|meas|print|plot|save|probe)\b/i;
const SPICE_CARD_LINE = /^\s*[RCLVIDQMX][A-Za-z0-9_]*\s+\S+\s+\S+\s+\S+/i;

/** Detect whether raw text in the editor or an agent reply looks like a SPICE netlist rather than .kess source. */
export function looksLikeSpiceNetlist(source: string): boolean {
  const lines = source.split(/\r?\n/);
  let cardCount = 0;
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('//')) continue;
    if (SPICE_DIRECTIVE_LINE.test(line)) return true;
    if (SPICE_CARD_LINE.test(line)) {
      cardCount += 1;
      if (cardCount >= 2) return true;
    }
  }
  return false;
}

/** Convert raw SPICE netlist text into .kess source using Kessetsu Core's canonical importer. */
export function convertSpiceTextToKess(input: string): SpiceImportReport {
  const report = import_spice_netlist(input) as SpiceImportReport;
  if (report.schema_version !== spice_import_schema_version()) {
    throw new Error(`Unsupported SPICE import report: ${report.schema_version}`);
  }
  const errors = report.diagnostics.filter((diagnostic) => diagnostic.severity === 'error');
  if (errors.length > 0 || !report.kess_source) {
    throw new SpiceImportError(report.diagnostics);
  }
  return report;
}
