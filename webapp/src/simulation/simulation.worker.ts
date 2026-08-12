/// <reference lib="webworker" />
import type {
  BrowserAnalysisPlan,
  BrowserSimulationPlan,
  ComplexValue,
  Dataset,
  EngineResult,
  SimulationResult,
  WorkerRequest,
  WorkerResponse,
} from './types';

const context = self as DedicatedWorkerGlobalScope;

function send(message: WorkerResponse) {
  context.postMessage(message);
}

function canonicalSignalName(name: string): string {
  const lower = name.trim().toLowerCase();
  const voltage = /^v\((.+)\)$/.exec(lower);
  if (voltage) return voltage[1];
  const current = /^i\((.+)\)$/.exec(lower);
  return current ? `${current[1]}#branch` : lower;
}

function finiteNumbers(values: number[], label: string): number[] {
  if (values.some((value) => !Number.isFinite(value))) {
    throw new Error(`${label} contains a non-finite value`);
  }
  return values;
}

function realValues(values: number[] | ComplexValue[], label: string): number[] {
  if (values.some((value) => typeof value !== 'number')) {
    throw new Error(`${label} unexpectedly contains complex data`);
  }
  return finiteNumbers(values as number[], label);
}

function complexValues(values: number[] | ComplexValue[], label: string): ComplexValue[] {
  if (values.some((value) => typeof value === 'number')) {
    throw new Error(`${label} unexpectedly contains real data`);
  }
  const complex = values as ComplexValue[];
  for (const value of complex) {
    if (!Number.isFinite(value.real) || !Number.isFinite(value.img)) {
      throw new Error(`${label} contains a non-finite complex value`);
    }
  }
  return complex;
}

function assertShape(result: EngineResult) {
  if (result.data.length !== result.numVariables || result.variableNames.length !== result.numVariables) {
    throw new Error('Simulator result variable count does not match its metadata');
  }
  for (const series of result.data) {
    if (series.values.length !== result.numPoints) {
      throw new Error(`Simulator series '${series.name}' has an invalid point count`);
    }
  }
}

function convertDataset(plan: BrowserAnalysisPlan, result: EngineResult): Dataset {
  assertShape(result);
  if (plan.analysis.kind === 'operating_point') {
    const values: Record<string, number> = {};
    for (const signal of result.data) {
      const series = realValues(signal.values, signal.name);
      const value = series.at(-1);
      if (value === undefined) throw new Error(`Operating-point signal '${signal.name}' is empty`);
      values[canonicalSignalName(signal.name)] = value;
    }
    return { kind: 'operating_point', values };
  }

  const [axis, ...signals] = result.data;
  if (!axis) throw new Error('Simulator result has no scale vector');

  if (plan.analysis.kind === 'ac') {
    const frequencies = complexValues(axis.values, axis.name).map((value) => value.real);
    const mapped: Record<string, { real: number[]; imaginary: number[] }> = {};
    for (const signal of signals) {
      const values = complexValues(signal.values, signal.name);
      mapped[canonicalSignalName(signal.name)] = {
        real: values.map((value) => value.real),
        imaginary: values.map((value) => value.img),
      };
    }
    return { kind: 'ac', frequency_hz: finiteNumbers(frequencies, axis.name), signals: mapped };
  }

  const axisValues = realValues(axis.values, axis.name);
  const mapped: Record<string, number[]> = {};
  for (const signal of signals) {
    mapped[canonicalSignalName(signal.name)] = realValues(signal.values, signal.name);
  }
  const series = { axis: { name: canonicalSignalName(axis.name), values: axisValues }, signals: mapped };
  return plan.analysis.kind === 'transient' ? { kind: 'transient', ...series } : { kind: 'dc_sweep', ...series };
}

function simulatorVersion(info: string, results: EngineResult[]): string {
  const match = `${info}\n${results.map((result) => result.header).join('\n')}`.match(/ngspice-[^\s,:]+/i);
  return match?.[0] ?? 'ngspice-wasm-unknown';
}

async function runPlan(id: number, plan: BrowserSimulationPlan, includeRawLog: boolean) {
  if (plan.schema_version !== 'netlang.simulation.v1') {
    throw new Error(`Unsupported simulation plan schema: ${plan.schema_version}`);
  }
  send({ type: 'progress', id, completed: 0, total: plan.analyses.length, message: 'Simulator indiriliyor' });
  // Keep the Worker bootstrap tiny and observable. Importing the engine at the
  // module top level delays registration of the message handler until its
  // large embedded model library has been parsed.
  const { Simulation } = await import('eecircuit-engine');
  send({ type: 'progress', id, completed: 0, total: plan.analyses.length, message: 'Simulator başlatılıyor' });
  const simulation = new Simulation();
  await simulation.start();
  const initInfo = simulation.getInitInfo();
  const engineResults: EngineResult[] = [];
  const datasets: SimulationResult['datasets'] = [];
  const warnings: string[] = [];

  for (const analysisPlan of plan.analyses) {
    send({
      type: 'progress',
      id,
      completed: analysisPlan.index,
      total: plan.analyses.length,
      message: `${analysisPlan.analysis.kind} çalışıyor`,
    });
    simulation.setNetList(analysisPlan.netlist);
    const result = (await simulation.runSim()) as EngineResult;
    engineResults.push(result);
    datasets.push({
      index: analysisPlan.index,
      analysis: analysisPlan.analysis,
      data: convertDataset(analysisPlan, result),
    });
    for (const message of simulation.getError()) {
      if (!warnings.includes(message)) warnings.push(message);
    }
  }

  const result: SimulationResult = {
    schema_version: 'netlang.simulation.v1',
    status: 'succeeded',
    analyses: plan.analyses.map((entry) => entry.analysis),
    simulator: {
      executable: 'browser-worker:eecircuit-engine@1.7.0',
      version: simulatorVersion(initInfo, engineResults),
    },
    process: { exit_code: 0, success: true },
    measurements: {},
    datasets,
    diagnostics: warnings.map((message) => ({
      code: 'NL-S003',
      severity: 'warning',
      kind: 'warning',
      message,
    })),
    warnings,
    errors: [],
    raw_log: {
      stdout: includeRawLog ? `${initInfo}\n${engineResults.map((entry) => entry.header).join('\n')}` : '',
      stderr: includeRawLog ? warnings.join('\n') : '',
    },
    artifacts: [],
  };
  send({ type: 'result', id, result });
}

context.addEventListener('message', (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  if (request.type !== 'run') return;
  void runPlan(request.id, request.plan, request.includeRawLog).catch((error: unknown) => {
    send({
      type: 'error',
      id: request.id,
      message: error instanceof Error ? error.message : String(error),
    });
  });
});
