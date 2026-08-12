import type { BrowserEvaluation } from './simulation/types';

export interface CompileDiagnostic {
  code: string;
  severity: 'error' | 'warning' | 'info';
  stage: 'parse' | 'flatten' | 'semantic' | 'erc' | 'io' | 'cli' | 'simulation' | 'assertion' | 'schematic';
  message: string;
  component?: string;
  pin?: string;
  field?: string;
  line?: number;
  column?: number;
}

export interface SchematicSummary {
  schema_version: string;
  connectivity: { verified: boolean };
  quality: { passed: boolean; issues: string[] };
}

export interface CompileReport {
  schema_version: string;
  diagnostics: CompileDiagnostic[];
  schematic: SchematicSummary | null;
  schematic_svg: string | null;
  kicad_sch: string | null;
  spice_netlist: string | null;
}

export type SimulationState = 'idle' | 'running' | 'succeeded' | 'failed' | 'cancelled';

export interface WorkspaceState {
  code: string;
  diagnostics: CompileDiagnostic[];
  compileSucceeded: boolean;
  wasmError: string | null;
  wasmLoaded: boolean;
  schematic: SchematicSummary | null;
  schematicSvg: string;
  kicadSch: string;
  spiceNetlist: string;
  simulationState: SimulationState;
  simulationMessage: string;
  evaluation: BrowserEvaluation | null;
}
