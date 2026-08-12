import { useCallback, useEffect, useRef, useState } from 'react';
import init, {
  compile_netlang,
  compile_schema_version,
  evaluate_browser_simulation,
  prepare_browser_simulation,
} from 'netlang-core';
import rcFilter from '../../../core/tests/fixtures/benchmarks/rc_filter.nl?raw';
import gainStage from '../../../core/tests/fixtures/benchmarks/gain_stage.nl?raw';
import powerAmplifier from '../../../core/tests/fixtures/benchmarks/power_amplifier.nl?raw';
import type { CompileReport, WorkspaceState } from '../domain';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';

export const examples = {
  rc: { label: 'RC Low-pass', description: '1 kHz cutoff, AC analysis', source: rcFilter },
  gain: { label: 'Gain Stage', description: 'Op-amp OP, AC and transient', source: gainStage },
  power: { label: 'Power Amplifier', description: '8 Ω multi-stage benchmark', source: powerAmplifier },
} as const;

export type ExampleId = keyof typeof examples;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const initialState: WorkspaceState = {
  code: rcFilter,
  diagnostics: [],
  compileSucceeded: false,
  wasmError: null,
  wasmLoaded: false,
  schematic: null,
  schematicSvg: '',
  kicadSch: '',
  spiceNetlist: '',
  simulationState: 'idle',
  simulationMessage: 'Run ile simülasyonu başlatın',
  evaluation: null,
};

export function useNetlangWorkspace() {
  const [state, setState] = useState(initialState);
  const runnerRef = useRef<BrowserSimulationRunner | null>(null);

  useEffect(() => {
    let mounted = true;
    void init()
      .then(() => mounted && setState((current) => ({ ...current, wasmLoaded: true })))
      .catch((error: unknown) => mounted && setState((current) => ({ ...current, wasmError: errorMessage(error) })));
    const runner = new BrowserSimulationRunner();
    runnerRef.current = runner;
    return () => {
      mounted = false;
      runner.dispose();
    };
  }, []);

  const compile = useCallback(() => {
    if (!state.wasmLoaded) return;
    try {
      const result = compile_netlang(state.code) as CompileReport;
      if (result.schema_version !== compile_schema_version()) {
        throw new Error(`Unsupported compile report schema: ${result.schema_version}`);
      }
      const hasErrors = result.diagnostics.some((diagnostic) => diagnostic.severity === 'error');
      const verified = Boolean(result.schematic?.connectivity.verified && result.schematic_svg);
      setState((current) => ({
        ...current,
        diagnostics: result.diagnostics,
        compileSucceeded: !hasErrors && verified,
        schematic: !hasErrors ? result.schematic : null,
        schematicSvg: !hasErrors ? result.schematic_svg ?? '' : '',
        spiceNetlist: !hasErrors ? result.spice_netlist ?? '' : '',
        kicadSch: !hasErrors ? result.kicad_sch ?? '' : '',
      }));
    } catch (error: unknown) {
      setState((current) => ({
        ...current,
        compileSucceeded: false,
        diagnostics: [{ code: 'NL-W001', severity: 'error', stage: 'io', message: errorMessage(error) }],
        schematic: null,
        schematicSvg: '',
        spiceNetlist: '',
        kicadSch: '',
      }));
    }
  }, [state.code, state.wasmLoaded]);

  useEffect(() => {
    const timeout = globalThis.setTimeout(compile, 250);
    return () => globalThis.clearTimeout(timeout);
  }, [compile]);

  const setCode = useCallback((code: string) => {
    runnerRef.current?.cancel();
    setState((current) => ({
      ...current,
      code,
      compileSucceeded: false,
      simulationState: 'idle',
      simulationMessage: 'Kaynak değişti; Run ile yeniden simüle edin',
      evaluation: null,
    }));
  }, []);

  const loadExample = useCallback((id: ExampleId) => setCode(examples[id].source), [setCode]);

  const run = useCallback(async () => {
    if (!state.wasmLoaded || !state.compileSucceeded || !runnerRef.current) return;
    setState((current) => ({ ...current, evaluation: null, simulationState: 'running', simulationMessage: 'Hazırlanıyor…' }));
    try {
      const plan = prepare_browser_simulation(state.code) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('Devrede çalıştırılacak simulate komutu yok');
      const simulation = await runnerRef.current.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => setState((current) => ({ ...current, simulationMessage: progress.message })),
      });
      const evaluation = evaluate_browser_simulation(state.code, simulation) as BrowserEvaluation;
      setState((current) => ({
        ...current,
        evaluation,
        simulationState: 'succeeded',
        simulationMessage: `${evaluation.simulation.datasets.length} analiz tamamlandı`,
      }));
    } catch (error: unknown) {
      if (error instanceof SimulationCancelledError) {
        setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simülasyon iptal edildi' }));
      } else {
        setState((current) => ({ ...current, simulationState: 'failed', simulationMessage: errorMessage(error) }));
      }
    }
  }, [state.code, state.compileSucceeded, state.wasmLoaded]);

  const cancel = useCallback(() => {
    runnerRef.current?.cancel();
    setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simülasyon iptal edildi' }));
  }, []);

  return { state, setCode, loadExample, compile, run, cancel };
}
