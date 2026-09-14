import { useCallback, useEffect, useRef, useState } from 'react';
import init, {
  compile_kessetsu,
  compile_schema_version,
  evaluate_browser_simulation,
  export_kessetsu,
  export_schema_version,
  prepare_browser_simulation,
  supported_export_capabilities,
} from 'kessetsu-core';
import rcFilter from '../../../core/tests/fixtures/benchmarks/rc_filter.kess?raw';
import gainStage from '../../../core/tests/fixtures/benchmarks/gain_stage.kess?raw';
import powerAmplifier from '../../../core/tests/fixtures/benchmarks/power_amplifier.kess?raw';
import type { CompileReport, ExportArtifact, ExportFormat, WorkspaceState } from '../domain';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';
import { assertSharedPackages, decodeShareFragment, encodeShareFragment, type ShareEnvelope } from '../share';

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
  compileState: 'loading',
  compileSucceeded: false,
  wasmError: null,
  wasmLoaded: false,
  schematic: null,
  schematicSvg: '',
  kicadSch: '',
  spiceNetlist: '',
  modelManifest: null,
  exportCapabilities: [],
  exportMessage: '',
  shareMessage: '',
  simulationState: 'idle',
  simulationMessage: 'Run a simulation to inspect results.',
  evaluation: null,
};

export function useKessetsuWorkspace() {
  const [state, setState] = useState(initialState);
  const runnerRef = useRef<BrowserSimulationRunner | null>(null);
  const sharedEnvelopeRef = useRef<ShareEnvelope | null>(null);

  useEffect(() => {
    let mounted = true;
    void init()
      .then(async () => {
        if (!mounted) return;
        const capabilities = supported_export_capabilities();
        const shareFragment = globalThis.location.hash.startsWith('#kessetsu=') ? globalThis.location.hash : '';
        const shared = await decodeShareFragment(shareFragment, compile_schema_version());
        sharedEnvelopeRef.current = shared;
        setState((current) => ({
          ...current,
          code: shared?.source ?? current.code,
          wasmLoaded: true,
          compileState: 'checking',
          exportCapabilities: capabilities as WorkspaceState['exportCapabilities'],
          shareMessage: shared ? 'Shared circuit loaded · package versions will be verified' : '',
        }));
      })
      .catch((error: unknown) => mounted && setState((current) => ({
        ...current,
        compileState: 'invalid',
        wasmError: errorMessage(error),
      })));
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
      const result = compile_kessetsu(state.code) as CompileReport;
      if (result.schema_version !== compile_schema_version()) {
        throw new Error(`Unsupported compile report schema: ${result.schema_version}`);
      }
      const hasErrors = result.diagnostics.some((diagnostic) => diagnostic.severity === 'error');
      const verified = Boolean(result.schematic?.connectivity.verified && result.schematic_svg);
      if (!hasErrors && sharedEnvelopeRef.current) {
        assertSharedPackages(sharedEnvelopeRef.current, result.ir?.model_manifest ?? null);
        sharedEnvelopeRef.current = null;
      }
      setState((current) => ({
        ...current,
        diagnostics: result.diagnostics,
        compileState: !hasErrors && verified ? 'valid' : 'invalid',
        compileSucceeded: !hasErrors && verified,
        schematic: !hasErrors ? result.schematic : null,
        schematicSvg: !hasErrors ? result.schematic_svg ?? '' : '',
        spiceNetlist: !hasErrors ? result.spice_netlist ?? '' : '',
        kicadSch: !hasErrors ? result.kicad_sch ?? '' : '',
        modelManifest: !hasErrors ? result.ir?.model_manifest ?? null : null,
      }));
    } catch (error: unknown) {
      setState((current) => ({
        ...current,
        compileSucceeded: false,
        compileState: 'invalid',
        diagnostics: [{ code: 'KES-W001', severity: 'error', stage: 'io', message: errorMessage(error) }],
        schematic: null,
        schematicSvg: '',
        spiceNetlist: '',
        kicadSch: '',
        modelManifest: null,
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
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      schematic: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: 'Source changed; run the simulation again.',
      evaluation: null,
      exportMessage: '',
      shareMessage: '',
    }));
  }, []);

  const loadExample = useCallback((id: ExampleId) => setCode(examples[id].source), [setCode]);

  const run = useCallback(async () => {
    if (!state.wasmLoaded || !state.compileSucceeded || !runnerRef.current) return;
    setState((current) => ({ ...current, evaluation: null, simulationState: 'running', simulationMessage: 'Preparing simulation…' }));
    try {
      const plan = prepare_browser_simulation(state.code) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('The circuit has no simulation command to run');
      const simulation = await runnerRef.current.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => setState((current) => ({ ...current, simulationMessage: progress.message })),
      });
      const evaluation = evaluate_browser_simulation(state.code, simulation) as BrowserEvaluation;
      setState((current) => ({
        ...current,
        evaluation,
        simulationState: 'succeeded',
        simulationMessage: `${evaluation.simulation.datasets.length} analyses completed`,
      }));
    } catch (error: unknown) {
      if (error instanceof SimulationCancelledError) {
        setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simulation cancelled' }));
      } else {
        setState((current) => ({ ...current, simulationState: 'failed', simulationMessage: errorMessage(error) }));
      }
    }
  }, [state.code, state.compileSucceeded, state.wasmLoaded]);

  const cancel = useCallback(() => {
    runnerRef.current?.cancel();
    setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simulation cancelled' }));
  }, []);

  const createExport = useCallback((format: ExportFormat, scale = 2, transparent = false): ExportArtifact => {
    if (!state.wasmLoaded || !state.compileSucceeded) {
      throw new Error('Compile and connectivity verification must succeed before export');
    }
    const artifact = export_kessetsu(state.code, format, scale, transparent) as ExportArtifact;
    if (artifact.schema_version !== export_schema_version()) {
      throw new Error(`Unsupported export schema: ${artifact.schema_version}`);
    }
    setState((current) => ({
      ...current,
      exportMessage: artifact.losses.length > 0
        ? `${artifact.label}: ${artifact.losses.join(' ')}`
        : `${artifact.label}: connectivity verified · ${artifact.sha256.slice(0, 12)}`,
    }));
    return artifact;
  }, [state.code, state.compileSucceeded, state.wasmLoaded]);

  const share = useCallback(async () => {
    if (!state.wasmLoaded || !state.compileSucceeded) throw new Error('Compile must succeed before sharing');
    const fragment = await encodeShareFragment(state.code, compile_schema_version(), state.modelManifest);
    const url = new URL(globalThis.location.href);
    url.hash = fragment.slice(1);
    globalThis.history.replaceState(null, '', url);
    try {
      await globalThis.navigator.clipboard.writeText(url.href);
      setState((current) => ({ ...current, shareMessage: 'Share URL copied · source and package versions embedded' }));
    } catch {
      setState((current) => ({ ...current, shareMessage: 'Share URL ready in the address bar · source and package versions embedded' }));
    }
  }, [state.code, state.compileSucceeded, state.modelManifest, state.wasmLoaded]);

  return { state, setCode, loadExample, compile, run, cancel, createExport, share };
}
