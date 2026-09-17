import { useCallback, useEffect, useRef, useState } from 'react';
import init, {
  compile_kessetsu_with_resources,
  compile_schema_version,
  evaluate_browser_simulation_with_resources,
  export_kessetsu_with_resources,
  export_schema_version,
  prepare_browser_simulation_with_resources,
  local_model_requirements,
  supported_export_capabilities,
} from 'kessetsu-core';
import rcFilter from '../../../core/tests/fixtures/benchmarks/rc_filter.kess?raw';
import gainStage from '../../../core/tests/fixtures/benchmarks/gain_stage.kess?raw';
import powerAmplifier from '../../../core/tests/fixtures/benchmarks/power_amplifier.kess?raw';
import reusableFilters from '../../../examples/reusable_filters.kess?raw';
import reusableAmplifiers from '../../../examples/reusable_amplifiers.kess?raw';
import externalComparator from '../../../examples/external_comparator.kess?raw';
import externalMemristor from '../../../examples/external_memristor.kess?raw';
import type { CompileReport, ExportArtifact, ExportFormat, WorkspaceState } from '../domain';
import {
  decodeWorkspaceDraft,
  documentNameFromFile,
  MAX_DOCUMENT_SOURCE_BYTES,
  normalizeDocumentName,
  writeWorkspaceDraft,
  WEB_DRAFT_STORAGE_KEY,
  takeToolCircuit,
} from '../document';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';
import { assertSharedPackages, decodeShareFragment, encodeShareFragment, type ShareEnvelope } from '../share';

export const examples = {
  rc: { label: 'RC Low-pass', description: '1 kHz cutoff, AC analysis', source: rcFilter },
  gain: { label: 'Gain Stage', description: 'Op-amp OP, AC and transient', source: gainStage },
  power: { label: 'Power Amplifier', description: '8 Ω multi-stage benchmark', source: powerAmplifier },
  filters: { label: 'Reusable Filters', description: 'One RC block, 500 Hz and 2 kHz instances', source: reusableFilters },
  amplifiers: { label: 'Reusable Amplifiers', description: 'One op-amp block, gains 5 and 10', source: reusableAmplifiers },
  comparator: { label: 'External Comparator', description: 'Choose the local comparator.lib model file', source: externalComparator },
  memristor: { label: 'Threshold Memristor', description: 'Choose the local memristor.lib model file', source: externalMemristor },
} as const;

export type ExampleId = keyof typeof examples;

export interface LocalModelRequirement {
  model: string;
  resource: string;
  sha256: string;
  simulator: string;
}

const newCircuitSource = `// New Kessetsu circuit: a valid 5 V source with a 1 kOhm load.
net GND
net OUT
source V1 5V
resistor R1 1k
connect V1.plus, R1.p1 to OUT
connect V1.minus, R1.p2 to GND
simulate op
`;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const initialState: WorkspaceState = {
  code: rcFilter,
  circuitName: examples.rc.label,
  isDirty: false,
  draftRestored: false,
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
  simulationState: 'idle',
  simulationMessage: 'Run the simulation to inspect plots and requirements.',
  evaluation: null,
};

export function useKessetsuWorkspace() {
  const [state, setState] = useState(initialState);
  const [modelResources, setModelResources] = useState<Record<string, number[]>>({});
  const [modelRequirements, setModelRequirements] = useState<LocalModelRequirement[]>([]);
  const revisionRef = useRef(0);
  const runnerRef = useRef<BrowserSimulationRunner | null>(null);
  const sharedEnvelopeRef = useRef<ShareEnvelope | null>(null);

  const leaveSharedUrl = useCallback(() => {
    sharedEnvelopeRef.current = null;
    if (!globalThis.location.hash.startsWith('#kessetsu=')) return;
    const url = new URL(globalThis.location.href);
    url.hash = 'editor';
    globalThis.history.replaceState(null, '', url);
  }, []);

  useEffect(() => {
    let mounted = true;
    void init()
      .then(async () => {
        if (!mounted) return;
        const capabilities = supported_export_capabilities();
        const shareFragment = globalThis.location.hash.startsWith('#kessetsu=') ? globalThis.location.hash : '';
        const shared = await decodeShareFragment(shareFragment, compile_schema_version());
        let draft: ReturnType<typeof decodeWorkspaceDraft> = null;
        let toolCircuit: ReturnType<typeof decodeWorkspaceDraft> = null;
        if (!shared) {
          try {
            toolCircuit = takeToolCircuit();
          } catch {
            // Session storage may be disabled independently of draft storage.
          }
          try {
            draft = decodeWorkspaceDraft(globalThis.localStorage.getItem(WEB_DRAFT_STORAGE_KEY));
          } catch {
            // Local storage is an optional recovery layer; the editor remains usable without it.
          }
        }
        sharedEnvelopeRef.current = shared;
        setState((current) => {
          const source = shared?.source ?? toolCircuit?.source ?? draft?.source ?? current.code;
          const exampleName = Object.values(examples).find((example) => example.source === source)?.label ?? null;
          return {
            ...current,
            code: source,
            circuitName: shared?.name ?? toolCircuit?.name ?? draft?.name ?? exampleName,
            isDirty: shared ? false : toolCircuit?.dirty ?? draft?.dirty ?? false,
            draftRestored: !shared && !toolCircuit && (draft?.dirty ?? false),
            wasmLoaded: true,
            compileState: 'checking',
            exportCapabilities: capabilities as WorkspaceState['exportCapabilities'],
          };
        });
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

  useEffect(() => {
    if (!state.wasmLoaded) return;
    const timeout = globalThis.setTimeout(() => {
      try {
        writeWorkspaceDraft(globalThis.localStorage, state.circuitName, state.code, state.isDirty);
      } catch {
        // Compilation and file downloads must not depend on storage availability.
      }
    }, 400);
    return () => globalThis.clearTimeout(timeout);
  }, [state.circuitName, state.code, state.isDirty, state.wasmLoaded]);

  const compile = useCallback(() => {
    if (!state.wasmLoaded) return;
    try {
      try {
        setModelRequirements(local_model_requirements(state.code) as LocalModelRequirement[]);
      } catch {
        setModelRequirements([]); // The canonical compiler owns source diagnostics.
      }
      const result = compile_kessetsu_with_resources(state.code, modelResources) as CompileReport;
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
  }, [state.code, state.wasmLoaded, modelResources]);

  useEffect(() => {
    const timeout = globalThis.setTimeout(compile, 250);
    return () => globalThis.clearTimeout(timeout);
  }, [compile]);

  const setCode = useCallback((code: string) => {
    revisionRef.current += 1;
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setState((current) => ({
      ...current,
      code,
      isDirty: true,
      draftRestored: false,
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      schematic: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: 'Simulation results are out of date. Run again.',
      evaluation: null,
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const loadExample = useCallback((id: ExampleId) => {
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setState((current) => ({
      ...current,
      code: examples[id].source,
      circuitName: examples[id].label,
      isDirty: false,
      draftRestored: false,
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      schematic: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: 'Run the simulation to inspect plots and requirements.',
      evaluation: null,
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const newDocument = useCallback(() => {
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setState((current) => ({
      ...current,
      code: newCircuitSource,
      circuitName: null,
      isDirty: false,
      draftRestored: false,
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      schematic: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: 'Add a simulation command, then run it.',
      evaluation: null,
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const openDocument = useCallback(async (file: File) => {
    if (file.size > MAX_DOCUMENT_SOURCE_BYTES) throw new Error('Circuit source exceeds the 1 MiB browser file limit');
    const source = await file.text();
    if (new TextEncoder().encode(source).byteLength > MAX_DOCUMENT_SOURCE_BYTES) {
      throw new Error('Circuit source exceeds the 1 MiB browser file limit');
    }
    const name = documentNameFromFile(file.name);
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setState((current) => ({
      ...current,
      code: source,
      circuitName: name,
      isDirty: false,
      draftRestored: false,
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      schematic: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: 'Run the simulation to inspect plots and requirements.',
      evaluation: null,
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const markSaved = useCallback(() => {
    setState((current) => ({ ...current, isDirty: false, draftRestored: false }));
  }, []);

  const saveBrowserDocument = useCallback(() => {
    writeWorkspaceDraft(globalThis.localStorage, state.circuitName, state.code, false);
    markSaved();
  }, [markSaved, state.circuitName, state.code]);

  const renameDocument = useCallback((name: string) => {
    const normalized = normalizeDocumentName(name);
    setState((current) => ({ ...current, circuitName: normalized, isDirty: true, draftRestored: false }));
  }, []);

  const run = useCallback(async () => {
    if (!state.wasmLoaded || !state.compileSucceeded || !runnerRef.current) return;
    const revision = ++revisionRef.current;
    setState((current) => ({ ...current, evaluation: null, simulationState: 'running', simulationMessage: 'Preparing simulation…' }));
    try {
      const plan = prepare_browser_simulation_with_resources(state.code, modelResources) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('The circuit has no simulation command to run');
      const simulation = await runnerRef.current.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => {
          if (revision === revisionRef.current) setState((current) => ({ ...current, simulationMessage: progress.message }));
        },
      });
      if (revision !== revisionRef.current) return;
      const evaluation = evaluate_browser_simulation_with_resources(state.code, simulation, modelResources) as BrowserEvaluation;
      setState((current) => ({
        ...current,
        evaluation,
        simulationState: 'succeeded',
        simulationMessage: `${evaluation.simulation.datasets.length} analyses completed`,
      }));
    } catch (error: unknown) {
      if (revision !== revisionRef.current) return;
      if (error instanceof SimulationCancelledError) {
        setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simulation cancelled' }));
      } else {
        setState((current) => ({ ...current, simulationState: 'failed', simulationMessage: errorMessage(error) }));
      }
    }
  }, [state.code, state.compileSucceeded, state.wasmLoaded, modelResources]);

  const cancel = useCallback(() => {
    revisionRef.current += 1;
    runnerRef.current?.cancel();
    setState((current) => ({ ...current, simulationState: 'cancelled', simulationMessage: 'Simulation cancelled' }));
  }, []);

  const createExport = useCallback((format: ExportFormat, scale = 2, transparent = false): ExportArtifact => {
    if (!state.wasmLoaded || !state.compileSucceeded) {
      throw new Error('Compile and connectivity verification must succeed before export');
    }
    const artifact = export_kessetsu_with_resources(state.code, format, scale, transparent, modelResources) as ExportArtifact;
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
  }, [state.code, state.compileSucceeded, state.wasmLoaded, modelResources]);

  const invalidateBindings = useCallback(() => {
    revisionRef.current += 1;
    runnerRef.current?.cancel();
    setState((current) => ({ ...current, compileState: 'checking', compileSucceeded: false,
      schematic: null, schematicSvg: '', spiceNetlist: '', kicadSch: '', modelManifest: null,
      diagnostics: [], evaluation: null, simulationState: 'idle', exportMessage: '',
      simulationMessage: 'Model bindings changed. Run the simulation again.' }));
  }, []);

  const bindModelFile = useCallback(async (resource: string, file: File) => {
    if (!modelRequirements.some((item) => item.resource === resource)) throw new Error('This resource is not declared by the current source');
    if (file.size > 16 * 1024 * 1024) throw new Error('Model files must be at most 16 MiB');
    const revision = revisionRef.current;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    if (revision !== revisionRef.current) throw new Error('The circuit changed while reading the model. Select the file again.');
    const next = { ...modelResources, [resource]: bytes };
    if (Object.keys(next).length > 64 || Object.values(next).reduce((sum, value) => sum + value.length, 0) > 32 * 1024 * 1024) {
      throw new Error('Local models are limited to 64 files and 32 MiB combined');
    }
    invalidateBindings();
    setModelResources(next);
  }, [modelRequirements, modelResources, invalidateBindings]);

  const clearModelFiles = useCallback(() => {
    invalidateBindings();
    setModelResources({});
  }, [invalidateBindings]);

  const share = useCallback(async (name: string): Promise<string> => {
    if (!state.wasmLoaded || !state.compileSucceeded) throw new Error('Compile must succeed before sharing');
    const circuitName = name.trim();
    const fragment = await encodeShareFragment(state.code, compile_schema_version(), state.modelManifest, circuitName);
    const url = new URL(globalThis.location.href);
    url.hash = fragment.slice(1);
    globalThis.history.replaceState(null, '', url);
    setState((current) => ({
      ...current,
      circuitName,
      isDirty: current.isDirty || current.circuitName !== circuitName,
      draftRestored: false,
    }));
    return url.href;
  }, [state.code, state.compileSucceeded, state.modelManifest, state.wasmLoaded]);

  return {
    state,
    modelRequirements,
    boundModelResources: Object.keys(modelResources),
    bindModelFile,
    clearModelFiles,
    setCode,
    loadExample,
    newDocument,
    openDocument,
    markSaved,
    saveBrowserDocument,
    renameDocument,
    compile,
    run,
    cancel,
    createExport,
    share,
  };
}
