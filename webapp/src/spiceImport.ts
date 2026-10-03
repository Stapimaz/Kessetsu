import type { SpiceImportDiagnostic } from './domain';

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
