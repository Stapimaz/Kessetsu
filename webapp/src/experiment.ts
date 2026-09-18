import type { AssertionReport, SimulationResult } from './simulation/types';

export type Unit = 'Ohm' | 'Farad' | 'Henry' | 'Volt' | 'Ampere' | 'Hertz' | 'Second' | 'Watt' | 'Joule' | 'Ratio' | 'Percent' | 'Degree';
export const suffix: Record<Unit, string> = { Ohm: 'Ohm', Farad: 'F', Henry: 'H', Volt: 'V', Ampere: 'A', Hertz: 'Hz', Second: 's', Watt: 'W', Joule: 'J', Ratio: '', Percent: '%', Degree: 'deg' };
export type AxisValues = { kind: 'list'; values: string[] } | { kind: 'linear' | 'log'; start: string; stop: string; points: number };
export interface StudySpec {
  schema_version: 'kessetsu.experiment.v1'; name: string; source: string; requirements: string | null;
  axes: Array<{ parameter: string; values: AxisValues }>;
  revisions: Array<{ name: string; source: string | null; parameters: Record<string, string> }>;
  tolerances: null | { mode: 'corners' | 'monte_carlo'; seed: number; samples: number; parameters: Array<{ parameter: string; relative: number; distribution: 'uniform' | 'normal'; group: string | null }> };
  temperatures_c: number[]; timeout_ms: number;
  measurements: Array<{ name: string; expression: string; unit: Unit }>;
  objective: null | { measurement: string; direction: 'minimize' | 'maximize' };
}
export interface StudyParameter { name: string; declared_unit: Unit; resolved: { value: number; unit: Unit } }
export interface StudyCase { id: string; revision: number; name: string; parameters: Record<string, string>; temperature_c: number }
export interface StudyPlan { identity: string; spec: StudySpec; cases: StudyCase[] }
export type CaseStatus = 'pending' | 'completed' | 'passed' | 'failed' | 'error' | 'cancelled';
export interface StudyCaseResult {
  case_id: string; status: CaseStatus;
  measurements: Record<string, { value: number | null; unit: Unit; error: string | null }>;
  assertions: AssertionReport | null; simulation: SimulationResult | null; errors: string[]; content_sha256: string;
}
export interface StudyResults {
  schema_version: 'kessetsu.experiment-results.v1'; identity: string; plan: StudyPlan;
  simulator: SimulationResult['simulator']; solver_fingerprint: string; cases: StudyCaseResult[];
  summary: { total: number; pending: number; completed: number; passed: number; failed: number; errors: number; cancelled: number; best_case: string | null };
}
export const reusable = (status: CaseStatus) => status !== 'pending' && status !== 'cancelled';
export function newStudy(source: string, name: string): StudySpec {
  return { schema_version: 'kessetsu.experiment.v1', name: `${name} study`, source, requirements: null, axes: [], revisions: [], tolerances: null, temperatures_c: [27], timeout_ms: 30_000, measurements: [], objective: null };
}
export function parameterAxis(parameter: StudyParameter): StudySpec['axes'][number] {
  const unit = suffix[parameter.declared_unit]; const value = parameter.resolved.value;
  return { parameter: parameter.name, values: { kind: 'list', values: [0.9, 1, 1.1].map(f => `${value * f}${unit}`) } };
}
