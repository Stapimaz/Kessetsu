import { useCallback, useEffect, useRef, useState } from 'react';
import init, {
  compile_kessetsu_with_resources,
  compile_schema_version,
  evaluate_browser_simulation_with_resources,
  export_kessetsu_with_resources,
  export_schema_version,
  import_spice_netlist,
  prepare_browser_simulation_with_resources,
  local_model_requirements,
  spice_import_schema_version,
  supported_export_capabilities,
} from 'kessetsu-core';
import rcFilter from '../../../core/tests/fixtures/benchmarks/rc_filter.kess?raw';
import gainStage from '../../../core/tests/fixtures/benchmarks/gain_stage.kess?raw';
import powerAmplifier from '../../../core/tests/fixtures/benchmarks/power_amplifier.kess?raw';
import reusableFilters from '../../../examples/reusable_filters.kess?raw';
import reusableAmplifiers from '../../../examples/reusable_amplifiers.kess?raw';
import externalComparator from '../../../examples/external_comparator.kess?raw';
import externalMemristor from '../../../examples/external_memristor.kess?raw';
import comparatorModelSource from '../../../examples/models/comparator.lib?raw';
import memristorModelSource from '../../../examples/models/memristor.lib?raw';
import loadedFilter from '../../../examples/loaded_filter.kess?raw';
import transistorDriver from '../../../examples/transistor_driver.kess?raw';
import zenerRegulator from '../../../examples/zener_shunt_regulator.kess?raw';
import sampleHold from '../../../examples/switched_sample_hold.kess?raw';
import currentSense from '../../../examples/current_sense_amplifier.kess?raw';
import type { CompileReport, ExportArtifact, ExportFormat, SpiceImportReport, WorkspaceState } from '../domain';
import { SpiceImportError } from '../spiceImport';
import {
  decodeWorkspaceDraft,
  documentNameFromFile,
  documentNameFromSpiceFile,
  MAX_DOCUMENT_SOURCE_BYTES,
  normalizeDocumentName,
  writeWorkspaceDraft,
  WEB_DRAFT_STORAGE_KEY,
  takeToolCircuit,
  type DocumentRevision,
} from '../document';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';
import { assertSharedPackages, decodeShareFragment, encodeShareFragment, type ShareEnvelope } from '../share';

const lm358PwmLed = `// Single-supply LM358 PWM comparator driving a red LED with soft-start initial condition.
net GND
net VCC_5V
net RAMP
net REF
net DRV
net LED_A

source VCC 5V
source VTRI pulse(0V, 5V, 0s, 499us, 499us, 2us, 1ms)
resistor RREF1 10k
resistor RREF2 10k
capacitor CREF 100nF
opamp U1 LM358
resistor RLED 220
diode D1 LED_RED

connect VCC.plus to VCC_5V
connect VCC.minus, VTRI.minus, RREF2.p2, CREF.p2, U1.vee, D1.cathode to GND
connect VTRI.plus to RAMP
connect RREF1.p1 to VCC_5V
connect RREF1.p2, RREF2.p1, CREF.p1, U1.in_n to REF
connect U1.in_p to RAMP
connect U1.vcc to VCC_5V
connect U1.out to DRV
connect RLED.p1 to DRV
connect RLED.p2, D1.anode to LED_A

initial V(REF) = 0V
simulate tran 10us 6ms uic

assert min(V(REF)) < 50mV
assert freq(V(DRV), 2ms, 6ms) > 950Hz
assert freq(V(DRV), 2ms, 6ms) < 1050Hz
assert duty_cycle(V(DRV), 2ms, 6ms) > 40%
assert duty_cycle(V(DRV), 2ms, 6ms) < 60%
assert max(V(D1.anode, D1.cathode), 2ms, 6ms) > 1.5V
`;

export const examples = {
  study_filter: { label: 'Loaded Filter', description: 'Parameterized RC filter with finite loading', source: loadedFilter },
  study_driver: { label: 'Transistor Driver', description: 'BJT supply, load and bias condition study', source: transistorDriver },
  rc: { label: 'RC Low-pass', description: '1 kHz cutoff, AC analysis', source: rcFilter },
  gain: { label: 'Gain Stage', description: 'Op-amp OP, AC and transient', source: gainStage },
  power: { label: 'Power Amplifier', description: '8 Ω multi-stage benchmark', source: powerAmplifier },
  filters: { label: 'Reusable Filters', description: 'One RC block, 500 Hz and 2 kHz instances', source: reusableFilters },
  amplifiers: { label: 'Reusable Amplifiers', description: 'One op-amp block, gains 5 and 10', source: reusableAmplifiers },
  comparator: { label: 'External Comparator', description: 'Generic comparator subcircuit (comparator.lib)', source: externalComparator },
  memristor: { label: 'Threshold Memristor', description: 'Threshold memristor subcircuit (memristor.lib)', source: externalMemristor },
  zener: { label: 'Zener Regulator', description: '5.1 V shunt regulator with BZX84C5V1', source: zenerRegulator },
  sample_hold: { label: 'Sample & Hold', description: 'Switched capacitor track and hold', source: sampleHold },
  current_sense: { label: 'Current Sense Amp', description: 'CCVS sensor with rail-clamped op-amp', source: currentSense },
  lm358_pwm_led: { label: 'LM358 & LED Driver', description: 'Single-supply PWM comparator, LED_RED & initial', source: lm358PwmLed },
} as const;

export type ExampleId = keyof typeof examples;

const BUNDLED_MODEL_FILES: Record<string, { sha256: string; bytes: number[] }> = {
  'models/comparator.lib': {
    sha256: '00198a3b00ef90c78b93beb6c7e7ed61c9f0220dc597a6c71cd0661343246c34',
    bytes: Array.from(new TextEncoder().encode(comparatorModelSource)),
  },
  'models/memristor.lib': {
    sha256: 'cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6',
    bytes: Array.from(new TextEncoder().encode(memristorModelSource)),
  },
};

export function hasBundledExampleModel(resource: string, sha256: string): boolean {
  const entry = BUNDLED_MODEL_FILES[resource];
  return Boolean(entry && entry.sha256 === sha256);
}

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

function exampleLabelForSource(source: string): string | null {
  return Object.values(examples).find((example) => example.source === source)?.label ?? null;
}

function resolveHasLocalModels(source: string): boolean {
  try {
    return (local_model_requirements(source) as LocalModelRequirement[]).length > 0;
  } catch {
    return false;
  }
}

function shareLoadFailureNotice(error: unknown): string {
  return `Could not open shared circuit link: ${errorMessage(error)}. Loaded the default circuit instead.`;
}

function resetDocumentFields(
  current: WorkspaceState,
  code: string,
  circuitName: string | null,
  simulationMessage = 'Run the simulation to inspect plots and requirements.',
): WorkspaceState {
  return {
    ...current,
    code,
    circuitName,
    isDirty: false,
    draftRestored: false,
    compileState: 'checking',
    compileSucceeded: false,
    diagnostics: [],
    schematic: null,
    circuitIr: null,
    schematicSvg: '',
    spiceNetlist: '',
    kicadSch: '',
    modelManifest: null,
    simulationState: 'idle',
    simulationMessage,
    hasSimulationAttempt: false,
    evaluation: null,
    exportMessage: '',
  };
}

const initialState: WorkspaceState = {
  code: rcFilter,
  circuitName: examples.rc.label,
  isDirty: false,
  draftRestored: false,
  draftStorageError: null,
  diagnostics: [],
  compileState: 'loading',
  compileSucceeded: false,
  wasmError: null,
  wasmLoaded: false,
  schematic: null,
  circuitIr: null,
  schematicSvg: '',
  kicadSch: '',
  spiceNetlist: '',
  modelManifest: null,
  exportCapabilities: [],
  exportMessage: '',
  simulationState: 'idle',
  hasSimulationAttempt: false,
  simulationMessage: 'Run the simulation to inspect plots and requirements.',
  evaluation: null,
};

export function useKessetsuWorkspace() {
  const [state, setState] = useState(initialState);
  const [shareError, setShareError] = useState<string | null>(null);
  const [modelResources, setModelResources] = useState<Record<string, number[]>>({});
  const [modelRequirements, setModelRequirements] = useState<LocalModelRequirement[]>([]);
  const revisionRef = useRef(0);
  const pendingAutoRerunRef = useRef(false);
  const simAutoRerunEligibleRef = useRef(false);
  const shareNavigationRef = useRef(0);
  const [documentRevision, setDocumentRevision] = useState(0);
  const documentRevisionRef = useRef<DocumentRevision>({ document: 0, edit: 0 });
  const captureDocumentRevision = useCallback(() => ({ ...documentRevisionRef.current }), []);
  const isCurrentDocument = useCallback((snapshot: DocumentRevision) =>
    snapshot.document === documentRevisionRef.current.document, []);
  const isCurrentDocumentRevision = useCallback((snapshot: DocumentRevision) =>
    snapshot.document === documentRevisionRef.current.document && snapshot.edit === documentRevisionRef.current.edit, []);
  const beginDocument = useCallback(() => {
    documentRevisionRef.current = { document: documentRevisionRef.current.document + 1, edit: 0 };
    setDocumentRevision((current) => current + 1);
  }, []);
  const runnerRef = useRef<BrowserSimulationRunner | null>(null);
  const sharedEnvelopeRef = useRef<ShareEnvelope | null>(null);
  const clearShareError = useCallback(() => setShareError(null), []);

  const leaveSharedUrl = useCallback(() => {
    sharedEnvelopeRef.current = null;
    if (!globalThis.location.hash.startsWith('#kessetsu=')) return;
    const url = new URL(globalThis.location.href);
    url.hash = 'editor';
    globalThis.history.replaceState(null, '', url);
  }, []);

  useEffect(() => {
    simAutoRerunEligibleRef.current = Boolean(state.hasSimulationAttempt && state.circuitIr?.analyses.length);
  }, [state.hasSimulationAttempt, state.circuitIr]);

  useEffect(() => {
    let mounted = true;
    void init()
      .then(async () => {
        if (!mounted) return;
        const capabilities = supported_export_capabilities();
        const shareFragment = globalThis.location.hash.startsWith('#kessetsu=') ? globalThis.location.hash : '';
        let shared: ShareEnvelope | null = null;
        let shareLoadFailure: string | null = null;
        if (shareFragment) {
          try {
            shared = await decodeShareFragment(shareFragment, compile_schema_version());
          } catch (error: unknown) {
            shareLoadFailure = shareLoadFailureNotice(error);
          }
          if (!mounted) return;
        }

        let draft: ReturnType<typeof decodeWorkspaceDraft> = null;
        let toolCircuit: ReturnType<typeof decodeWorkspaceDraft> = null;
        if (!shared && !shareLoadFailure) {
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
        if (shareLoadFailure) {
          leaveSharedUrl();
        }
        const initialCode = shareLoadFailure
          ? initialState.code
          : (shared?.source ?? toolCircuit?.source ?? draft?.source ?? initialState.code);
        const hasLocalModelRequirements = resolveHasLocalModels(initialCode);
        sharedEnvelopeRef.current = shared;
        pendingAutoRerunRef.current = !hasLocalModelRequirements;
        setShareError(shareLoadFailure);
        setState((current) => {
          const source = shareLoadFailure
            ? current.code
            : (shared?.source ?? toolCircuit?.source ?? draft?.source ?? current.code);
          const fallbackCircuitName = shareLoadFailure ? examples.rc.label : exampleLabelForSource(source);
          return {
            ...current,
            code: source,
            circuitName: shareLoadFailure
              ? examples.rc.label
              : (shared?.name ?? toolCircuit?.name ?? draft?.name ?? fallbackCircuitName),
            isDirty: shared || shareLoadFailure ? false : toolCircuit?.dirty ?? draft?.dirty ?? false,
            draftRestored: !shared && !shareLoadFailure && !toolCircuit && (draft?.dirty ?? false),
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
  }, [leaveSharedUrl]);

  useEffect(() => {
    if (!state.wasmLoaded) return;
    let active = true;
    const onHashChange = async () => {
      const fragment = globalThis.location.hash;
      if (!fragment.startsWith('#kessetsu=')) return;
      const requestId = shareNavigationRef.current + 1;
      shareNavigationRef.current = requestId;
      let shared: ShareEnvelope | null = null;
      let shareLoadFailure: string | null = null;
      try {
        shared = await decodeShareFragment(fragment, compile_schema_version());
        if (!shared) shareLoadFailure = shareLoadFailureNotice(new Error('Invalid shared link payload'));
      } catch (error: unknown) {
        shareLoadFailure = shareLoadFailureNotice(error);
      }
      if (!active || requestId !== shareNavigationRef.current) return;

      beginDocument();
      revisionRef.current += 1;
      runnerRef.current?.cancel();
      setModelResources({});

      if (shared && !shareLoadFailure) {
        const validShare = shared;
        const hasLocalModels = resolveHasLocalModels(validShare.source);
        sharedEnvelopeRef.current = validShare;
        pendingAutoRerunRef.current = !hasLocalModels;
        setShareError(null);
        setState((current) => resetDocumentFields(
          current,
          validShare.source,
          validShare.name ?? exampleLabelForSource(validShare.source),
          hasLocalModels
            ? 'Choose declared local model files in View > Circuit details before simulating.'
            : 'Run the simulation to inspect plots and requirements.',
        ));
        return;
      }

      leaveSharedUrl();
      pendingAutoRerunRef.current = true;
      setShareError(shareLoadFailure);
      setState((current) => resetDocumentFields(current, initialState.code, examples.rc.label));
    };

    const handleHashChange = () => {
      void onHashChange();
    };
    globalThis.addEventListener('hashchange', handleHashChange);
    return () => {
      active = false;
      globalThis.removeEventListener('hashchange', handleHashChange);
    };
  }, [beginDocument, leaveSharedUrl, state.wasmLoaded]);

  useEffect(() => {
    if (!state.wasmLoaded) return;
    const timeout = globalThis.setTimeout(() => {
      try {
        writeWorkspaceDraft(globalThis.localStorage, state.circuitName, state.code, state.isDirty);
        setState((current) => current.draftStorageError ? { ...current, draftStorageError: null } : current);
      } catch {
        const message = 'Browser draft recovery is unavailable. Save a .kess copy to keep this circuit.';
        setState((current) => current.draftStorageError === message ? current : { ...current, draftStorageError: message });
      }
    }, 400);
    return () => globalThis.clearTimeout(timeout);
  }, [state.circuitName, state.code, state.isDirty, state.wasmLoaded]);

  useEffect(() => {
    if (!state.wasmLoaded) return;
    const preserveBeforeLeaving = (event: BeforeUnloadEvent) => {
      try {
        // Flush the latest edit even if the ordinary 400 ms recovery timer has not fired.
        writeWorkspaceDraft(globalThis.localStorage, state.circuitName, state.code, state.isDirty);
      } catch {
        if (!state.isDirty) return;
        event.preventDefault();
        event.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', preserveBeforeLeaving);
    return () => window.removeEventListener('beforeunload', preserveBeforeLeaving);
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
      const valid = !hasErrors && verified;
      if (!valid) pendingAutoRerunRef.current = false;
      if (!hasErrors && sharedEnvelopeRef.current) {
        assertSharedPackages(sharedEnvelopeRef.current, result.ir?.model_manifest ?? null);
        sharedEnvelopeRef.current = null;
      }
      setState((current) => ({
        ...current,
        diagnostics: result.diagnostics,
        compileState: valid ? 'valid' : 'invalid',
        compileSucceeded: valid,
        schematic: !hasErrors ? result.schematic : null,
        circuitIr: !hasErrors ? result.ir ?? null : null,
        schematicSvg: !hasErrors ? result.schematic_svg ?? '' : '',
        spiceNetlist: !hasErrors ? result.spice_netlist ?? '' : '',
        kicadSch: !hasErrors ? result.kicad_sch ?? '' : '',
        modelManifest: !hasErrors ? result.ir?.model_manifest ?? null : null,
        evaluation: valid ? current.evaluation : null,
        simulationState: valid ? current.simulationState : 'idle',
        simulationMessage: valid
          ? current.simulationMessage
          : (current.hasSimulationAttempt ? 'Simulation results are out of date. Run again.' : 'Run the simulation to inspect plots and requirements.'),
      }));
    } catch (error: unknown) {
      pendingAutoRerunRef.current = false;
      setState((current) => ({
        ...current,
        compileSucceeded: false,
        compileState: 'invalid',
        diagnostics: [{ code: 'KES-W001', severity: 'error', stage: 'io', message: errorMessage(error) }],
        schematic: null,
        circuitIr: null,
        schematicSvg: '',
        spiceNetlist: '',
        kicadSch: '',
        modelManifest: null,
        evaluation: null,
        simulationState: 'idle',
        simulationMessage: current.hasSimulationAttempt ? 'Simulation results are out of date. Run again.' : 'Run the simulation to inspect plots and requirements.',
      }));
    }
  }, [state.code, state.wasmLoaded, modelResources]);

  useEffect(() => {
    const timeout = globalThis.setTimeout(compile, 250);
    return () => globalThis.clearTimeout(timeout);
  }, [compile]);

  const setCode = useCallback((code: string) => {
    pendingAutoRerunRef.current = false;
    documentRevisionRef.current.edit += 1;
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
      circuitIr: null,
      schematicSvg: '',
      spiceNetlist: '',
      kicadSch: '',
      modelManifest: null,
      simulationState: 'idle',
      simulationMessage: current.hasSimulationAttempt ? 'Simulation results are out of date. Run again.' : 'Run the simulation to inspect plots and requirements.',
      evaluation: null,
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const tweakCode = useCallback((code: string) => {
    documentRevisionRef.current.edit += 1;
    revisionRef.current += 1;
    leaveSharedUrl();
    runnerRef.current?.cancel();
    const willAutoRerun = simAutoRerunEligibleRef.current;
    pendingAutoRerunRef.current = willAutoRerun;
    setState((current) => ({
      ...current,
      code,
      isDirty: true,
      draftRestored: false,
      compileState: 'checking',
      compileSucceeded: false,
      diagnostics: [],
      evaluation: willAutoRerun ? current.evaluation : null,
      simulationState: willAutoRerun ? current.simulationState : 'idle',
      simulationMessage: willAutoRerun
        ? 'Updating schematic & simulation…'
        : (current.hasSimulationAttempt ? 'Simulation results are out of date. Run again.' : 'Run the simulation to inspect plots and requirements.'),
      exportMessage: '',
    }));
  }, [leaveSharedUrl]);

  const loadExample = useCallback((id: ExampleId) => {
    pendingAutoRerunRef.current = true;
    beginDocument();
    revisionRef.current += 1;
    if (id === 'comparator') {
      setModelResources({ 'models/comparator.lib': BUNDLED_MODEL_FILES['models/comparator.lib'].bytes });
    } else if (id === 'memristor') {
      setModelResources({ 'models/memristor.lib': BUNDLED_MODEL_FILES['models/memristor.lib'].bytes });
    } else {
      setModelResources({});
    }
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setShareError(null);
    setState((current) => resetDocumentFields(current, examples[id].source, examples[id].label));
  }, [beginDocument, leaveSharedUrl]);

  const newDocument = useCallback(() => {
    pendingAutoRerunRef.current = false;
    beginDocument();
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setShareError(null);
    setState((current) => resetDocumentFields(
      current,
      newCircuitSource,
      null,
      'This starter circuit includes an operating-point analysis. Press Run to simulate it.',
    ));
  }, [beginDocument, leaveSharedUrl]);

  const openDocument = useCallback(async (file: File) => {
    const snapshot = captureDocumentRevision();
    if (file.size > MAX_DOCUMENT_SOURCE_BYTES) throw new Error('Circuit source exceeds the 1 MiB browser file limit');
    const source = await file.text();
    if (!isCurrentDocumentRevision(snapshot)) throw new Error('The circuit changed while opening the file. Open it again when you are ready to replace the current work.');
    if (new TextEncoder().encode(source).byteLength > MAX_DOCUMENT_SOURCE_BYTES) {
      throw new Error('Circuit source exceeds the 1 MiB browser file limit');
    }
    const name = documentNameFromFile(file.name);
    pendingAutoRerunRef.current = false;
    beginDocument();
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setShareError(null);
    setState((current) => resetDocumentFields(current, source, name));
  }, [beginDocument, captureDocumentRevision, isCurrentDocumentRevision, leaveSharedUrl]);

  const applyConvertedSpice = useCallback((kessSource: string, name: string | null = null) => {
    pendingAutoRerunRef.current = true;
    beginDocument();
    revisionRef.current += 1;
    setModelResources({});
    leaveSharedUrl();
    runnerRef.current?.cancel();
    setShareError(null);
    setState((current) => ({
      ...resetDocumentFields(current, kessSource, name, 'Imported from SPICE. Run the simulation to inspect results.'),
      isDirty: true,
    }));
  }, [beginDocument, leaveSharedUrl]);

  const importSpiceDocument = useCallback(async (file: File): Promise<SpiceImportReport> => {
    const snapshot = captureDocumentRevision();
    if (!state.wasmLoaded) throw new Error('Kessetsu Core is still loading');
    if (file.size > MAX_DOCUMENT_SOURCE_BYTES) throw new Error('SPICE source exceeds the 1 MiB browser import limit');
    const input = await file.text();
    if (!isCurrentDocumentRevision(snapshot)) throw new Error('The circuit changed while reading the netlist. Import it again when you are ready to replace the current work.');
    if (new TextEncoder().encode(input).byteLength > MAX_DOCUMENT_SOURCE_BYTES) {
      throw new Error('SPICE source exceeds the 1 MiB browser import limit');
    }
    const report = import_spice_netlist(input) as SpiceImportReport;
    if (report.schema_version !== spice_import_schema_version()) {
      throw new Error(`Unsupported SPICE import report: ${report.schema_version}`);
    }
    const errors = report.diagnostics.filter((diagnostic) => diagnostic.severity === 'error');
    if (errors.length > 0 || !report.kess_source) {
      throw new SpiceImportError(report.diagnostics);
    }
    applyConvertedSpice(report.kess_source, documentNameFromSpiceFile(file.name));
    return report;
  }, [applyConvertedSpice, captureDocumentRevision, isCurrentDocumentRevision, state.wasmLoaded]);

  const markSaved = useCallback((snapshot: DocumentRevision) => {
    setState((current) => isCurrentDocumentRevision(snapshot) ? { ...current, isDirty: false, draftRestored: false } : current);
  }, [isCurrentDocumentRevision]);

  const saveBrowserDocument = useCallback(() => {
    writeWorkspaceDraft(globalThis.localStorage, state.circuitName, state.code, false);
    markSaved(captureDocumentRevision());
    setState((current) => current.draftStorageError ? { ...current, draftStorageError: null } : current);
  }, [captureDocumentRevision, markSaved, state.circuitName, state.code]);

  const renameDocument = useCallback((name: string) => {
    const normalized = normalizeDocumentName(name);
    documentRevisionRef.current.edit += 1;
    setState((current) => ({ ...current, circuitName: normalized, isDirty: true, draftRestored: false }));
  }, []);

  const executeRun = useCallback(async (preserveEvaluation: boolean) => {
    if (!state.wasmLoaded || !state.compileSucceeded || !state.circuitIr?.analyses.length || !runnerRef.current) return;
    const revision = ++revisionRef.current;
    setState((current) => ({
      ...current,
      evaluation: preserveEvaluation ? current.evaluation : null,
      simulationState: 'running',
      hasSimulationAttempt: true,
      simulationMessage: preserveEvaluation && current.evaluation ? 'Updating simulation…' : 'Preparing simulation…',
    }));
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
        setState((current) => ({ ...current, evaluation: null, simulationState: 'cancelled', simulationMessage: 'Simulation cancelled' }));
      } else {
        setState((current) => ({ ...current, evaluation: null, simulationState: 'failed', simulationMessage: errorMessage(error) }));
      }
    }
  }, [state.code, state.compileSucceeded, state.circuitIr, state.wasmLoaded, modelResources]);

  const run = useCallback(() => executeRun(false), [executeRun]);

  useEffect(() => {
    if (!pendingAutoRerunRef.current) return;
    if (state.compileState === 'invalid') {
      pendingAutoRerunRef.current = false;
      return;
    }
    if (state.compileState === 'valid' && state.compileSucceeded) {
      pendingAutoRerunRef.current = false;
      void executeRun(true);
    }
  }, [executeRun, state.compileState, state.compileSucceeded]);

  const cancel = useCallback(() => {
    pendingAutoRerunRef.current = false;
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
    const notices = [...new Set([...artifact.warnings, ...artifact.losses])];
    setState((current) => ({
      ...current,
      exportMessage: notices.length > 0
        ? `${artifact.label}: ${notices.join(' ')}`
        : `${artifact.label}: ${artifact.connectivity_verified ? 'connectivity verified' : 'supporting data'} · ${artifact.sha256.slice(0, 12)}`,
    }));
    return artifact;
  }, [state.code, state.compileSucceeded, state.wasmLoaded, modelResources]);

  const invalidateBindings = useCallback(() => {
    pendingAutoRerunRef.current = false;
    revisionRef.current += 1;
    runnerRef.current?.cancel();
    setState((current) => ({ ...current, compileState: 'checking', compileSucceeded: false,
      schematic: null, circuitIr: null, schematicSvg: '', spiceNetlist: '', kicadSch: '', modelManifest: null,
      diagnostics: [], evaluation: null, simulationState: 'idle', exportMessage: '',
      simulationMessage: current.hasSimulationAttempt ? 'Model bindings changed. Run the simulation again.' : 'Run the simulation to inspect plots and requirements.' }));
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

  const bindBundledModel = useCallback((resource: string) => {
    const req = modelRequirements.find((item) => item.resource === resource);
    const bundled = req ? BUNDLED_MODEL_FILES[resource] : undefined;
    if (!req || !bundled || bundled.sha256 !== req.sha256) return;
    invalidateBindings();
    setModelResources((current) => ({ ...current, [resource]: bundled.bytes }));
  }, [modelRequirements, invalidateBindings]);

  const share = useCallback(async (name: string, signal: AbortSignal): Promise<string> => {
    const snapshot = captureDocumentRevision();
    if (!state.wasmLoaded || !state.compileSucceeded) throw new Error('Compile must succeed before sharing');
    const circuitName = name.trim();
    const fragment = await encodeShareFragment(state.code, compile_schema_version(), state.modelManifest, circuitName);
    if (signal.aborted) throw new Error('Link creation cancelled.');
    if (!isCurrentDocumentRevision(snapshot)) throw new Error('The circuit changed while creating the link. Create a new link for the current version.');
    const url = new URL(globalThis.location.href);
    url.hash = fragment.slice(1);
    globalThis.history.replaceState(null, '', url);
    if (circuitName !== state.circuitName) documentRevisionRef.current.edit += 1;
    setState((current) => ({
      ...current,
      circuitName,
      isDirty: current.isDirty || current.circuitName !== circuitName,
      draftRestored: false,
    }));
    return url.href;
  }, [captureDocumentRevision, isCurrentDocumentRevision, state.circuitName, state.code, state.compileSucceeded, state.modelManifest, state.wasmLoaded]);

  return {
    state,
    modelRequirements,
    boundModelResources: Object.keys(modelResources),
    modelResources,
    bindModelFile,
    bindBundledModel,
    clearModelFiles,
    setCode,
    tweakCode,
    documentRevision,
    loadExample,
    newDocument,
    openDocument,
    importSpiceDocument,
    applyConvertedSpice,
    markSaved,
    captureDocumentRevision,
    isCurrentDocument,
    isCurrentDocumentRevision,
    saveBrowserDocument,
    renameDocument,
    compile,
    run,
    cancel,
    createExport,
    share,
    shareError,
    clearShareError,
  };
}
