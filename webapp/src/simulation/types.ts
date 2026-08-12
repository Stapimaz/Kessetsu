export type Analysis =
  | { kind: 'operating_point' }
  | { kind: 'transient'; step: Quantity; stop: Quantity }
  | { kind: 'ac'; scale: 'Decade' | 'Octave' | 'Linear'; points: number; start: Quantity; stop: Quantity }
  | { kind: 'dc_sweep'; source: string; start: Quantity; stop: Quantity; step: Quantity };

interface Quantity {
  value: number;
  unit: string;
}

export interface BrowserAnalysisPlan {
  index: number;
  analysis: Analysis;
  netlist: string;
}

export interface BrowserSimulationPlan {
  schema_version: 'netlang.simulation.v1';
  simulator_adapter: 'eecircuit-engine@1.7.0';
  analyses: BrowserAnalysisPlan[];
}

export interface ComplexValue {
  real: number;
  img: number;
}

export interface EngineSeries {
  name: string;
  type: 'voltage' | 'current' | 'time' | 'frequency' | 'notype';
  values: number[] | ComplexValue[];
}

export interface EngineResult {
  header: string;
  numVariables: number;
  variableNames: string[];
  numPoints: number;
  dataType: 'real' | 'complex';
  data: EngineSeries[];
}

export interface SimulationResult {
  schema_version: 'netlang.simulation.v1';
  status: 'succeeded' | 'failed' | 'timed_out' | 'cancelled';
  analyses: Analysis[];
  simulator: { executable: string; version: string };
  process: { exit_code: number | null; success: boolean };
  measurements: Record<string, number>;
  datasets: Array<{ index: number; analysis: Analysis; data: Dataset }>;
  diagnostics: Array<{
    code: string;
    severity: 'warning' | 'error';
    kind: 'warning' | 'convergence' | 'fatal' | 'result_parse';
    message: string;
  }>;
  warnings: string[];
  errors: string[];
  raw_log: { stdout: string; stderr: string };
  artifacts: [];
}

export type Dataset =
  | { kind: 'operating_point'; values: Record<string, number> }
  | { kind: 'transient'; axis: SeriesAxis; signals: Record<string, number[]> }
  | { kind: 'ac'; frequency_hz: number[]; signals: Record<string, { real: number[]; imaginary: number[] }> }
  | { kind: 'dc_sweep'; axis: SeriesAxis; signals: Record<string, number[]> };

export interface SeriesAxis {
  name: string;
  values: number[];
}

export interface AssertionResult {
  code: string;
  metric: string;
  signal: string;
  comparator: string;
  actual: number | null;
  threshold: number;
  unit: string;
  status: 'PASS' | 'FAIL' | 'ERROR' | 'SKIPPED';
  message?: string;
}

export interface AssertionReport {
  schema_version: 'netlang.assertion.v1';
  tolerance: { absolute: number; relative: number };
  assertions: AssertionResult[];
  summary: { total: number; passed: number; failed: number; errors: number; skipped: number };
}

export interface BrowserEvaluation {
  simulation: SimulationResult;
  assertions: AssertionReport;
}

export type WorkerRequest = {
  type: 'run';
  id: number;
  plan: BrowserSimulationPlan;
  includeRawLog: boolean;
};

export type WorkerResponse =
  | { type: 'progress'; id: number; completed: number; total: number; message: string }
  | { type: 'result'; id: number; result: SimulationResult }
  | { type: 'error'; id: number; message: string };
