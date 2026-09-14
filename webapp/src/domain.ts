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
  ir?: { model_manifest: ModelManifest } | null;
}

export interface ModelManifest {
  schema_version: string;
  models: ModelInfo[];
  packages: Array<{ name: string; version: string }>;
}

export interface ModelInfo {
  name: string;
  source: string;
  provenance: {
    source: string;
    license: string;
    version: string;
    content_hash: string;
    simulator: string;
  };
  external?: {
    resource: string;
    entry: string;
    pins: string[];
    simulator: 'ngspice' | 'ngspice_ps';
    redistribution: 'permitted' | 'prohibited';
  };
}

export type ExportFormat = 'svg' | 'png' | 'pdf' | 'schematic_json' | 'spice' | 'kicad' | 'ltspice';

export interface ExportCapability {
  visual: boolean;
  machine_readable: boolean;
  editable: boolean;
  preserves_connectivity: boolean;
  preserves_models: boolean;
  preserves_analysis: boolean;
}

export interface ExportDescriptor {
  schema_version: string;
  format: ExportFormat;
  label: string;
  extension: string;
  mime_type: string;
  capability: ExportCapability;
}

export interface ExportArtifact extends ExportDescriptor {
  exporter: string;
  exporter_version: number;
  sha256: string;
  byte_length: number;
  connectivity_verified: boolean;
  warnings: string[];
  losses: string[];
  bytes: number[];
}

export type SimulationState = 'idle' | 'running' | 'succeeded' | 'failed' | 'cancelled';
export type CompileState = 'loading' | 'checking' | 'valid' | 'invalid';

export interface WorkspaceState {
  code: string;
  diagnostics: CompileDiagnostic[];
  compileState: CompileState;
  compileSucceeded: boolean;
  wasmError: string | null;
  wasmLoaded: boolean;
  schematic: SchematicSummary | null;
  schematicSvg: string;
  kicadSch: string;
  spiceNetlist: string;
  modelManifest: ModelManifest | null;
  exportCapabilities: ExportDescriptor[];
  exportMessage: string;
  shareMessage: string;
  simulationState: SimulationState;
  simulationMessage: string;
  evaluation: BrowserEvaluation | null;
}
