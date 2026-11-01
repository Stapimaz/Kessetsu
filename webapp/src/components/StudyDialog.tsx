import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Play,
  Square,
  Plus,
  Download,
  RotateCcw,
  Upload,
  ArrowRight,
  ArrowLeft,
  X,
  Sliders,
} from 'lucide-react';
import coreWasmUrl from 'kessetsu-core/kessetsu_core_bg.wasm?url';
import {
  study_parameters,
  plan_study,
  prepare_study_case,
  start_study_results,
  evaluate_study_case,
  inspect_study_results,
  resume_study_results,
  export_study_results,
  study_effective_source,
  compare_study_plot,
  prepare_browser_simulation,
} from 'kessetsu-core';
import { downloadTextFile, sanitizeFileStem } from '../document';
import {
  newStudy,
  parameterAxis,
  reusable,
  suffix,
  type StudySpec,
  type StudyPlan,
  type StudyResults,
  type StudyParameter,
  type StudyCaseResult,
  type Unit,
  type AxisValues,
} from '../experiment';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserSimulationPlan } from '../simulation/types';
import { WorkflowDialog, type WorkflowStep } from './WorkflowDialog';
import './StudyDialog.css';
import loadedFilterStudy from '../../../examples/studies/loaded-filter.kessstudy.json?raw';

export type StudyStep = 'method' | 'variables' | 'checks' | 'review' | 'results';

interface Props {
  open: boolean;
  source: string;
  name: string;
  resources: Record<string, number[]>;
  onClose: () => void;
  onApplySource: (source: string) => void;
}

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const probeSource =
  'net GND\nnet OUT\nsource V1 1V\nresistor R1 1k\nconnect V1.plus, R1.p1 to OUT\nconnect V1.minus, R1.p2 to GND\nsimulate op\n';
const signalLabel = (key: string) =>
  key.endsWith('#branch') ? `I(${key.slice(0, -7)})` : key.startsWith('@') ? key : `V(${key})`;
type Tolerance = NonNullable<StudySpec['tolerances']>;

function buildEditorSpec(source: string, name: string, resources: Record<string, number[]>): StudySpec {
  const next = newStudy(source, name);
  try {
    const roots = study_parameters(source, resources) as StudyParameter[];
    if (roots[0]) next.axes = [parameterAxis(roots[0])];
  } catch {
    /* Canonical planning owns diagnostics. */
  }
  return next;
}

export function StudyDialog({ open, source, name, resources, onClose, onApplySource }: Props) {
  const runner = useRef<BrowserSimulationRunner | null>(null);
  if (!runner.current) runner.current = new BrowserSimulationRunner();
  const cancelled = useRef(false);
  const active = useRef(false);
  const initialized = useRef(false);

  const [hasCustomSnapshot, setHasCustomSnapshot] = useState(false);
  const [spec, setSpec] = useState<StudySpec>(() => newStudy(source, name));
  const [parameters, setParameters] = useState<StudyParameter[]>([]);
  const [parameterError, setParameterError] = useState('');
  const [results, setResults] = useState<StudyResults | null>(null);
  const [previousResults, setPreviousResults] = useState<StudyResults | null>(null);
  const [comparison, setComparison] = useState<StudyResults | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState('');
  const [error, setError] = useState('');
  const [step, setStep] = useState<StudyStep>('method');
  const [selected, setSelected] = useState<string[]>([]);
  const [signal, setSignal] = useState('V(OUT)');
  const [analysis, setAnalysis] = useState(0);
  const [advancedJson, setAdvancedJson] = useState('');
  const [useAdvancedJson, setUseAdvancedJson] = useState(false);
  const [reqFileName, setReqFileName] = useState('study.kessreq');
  const [temperatureText, setTemperatureText] = useState('27');
  const [plot, setPlot] = useState('');
  const [plotError, setPlotError] = useState('');
  const [exportTarget, setExportTarget] = useState('json');

  const importInput = useRef<HTMLInputElement>(null);
  const compareInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open && (!initialized.current || (!results && !hasCustomSnapshot && spec.source !== source))) {
      initialized.current = true;
      setSpec(buildEditorSpec(source, name, resources));
      setStep('method');
    }
  }, [open, source, name, resources, results, hasCustomSnapshot, spec.source]);

  useEffect(() => () => {
    cancelled.current = true;
    runner.current?.dispose();
  }, []);

  useEffect(() => {
    if (!running) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [running]);

  useEffect(() => {
    if (!open && active.current) {
      cancelled.current = true;
      runner.current?.cancel();
    }
  }, [open]);

  useEffect(() => {
    if (active.current) {
      cancelled.current = true;
      runner.current?.cancel();
    }
    if (!open) return;
    try {
      setParameters(study_parameters(spec.source, resources) as StudyParameter[]);
      setParameterError('');
    } catch (cause) {
      setParameters([]);
      setParameterError(message(cause));
    }
  }, [spec.source, resources, open]);

  const update = (change: Partial<StudySpec>) => {
    setSpec(current => ({ ...current, ...change }));
    setError('');
  };

  const axisUpdate = (index: number, change: Partial<StudySpec['axes'][number]>) =>
    update({ axes: spec.axes.map((axis, i) => (i === index ? { ...axis, ...change } : axis)) });

  const toleranceUpdate = (index: number, change: Partial<Tolerance['parameters'][number]>) => {
    if (spec.tolerances) {
      update({
        tolerances: {
          ...spec.tolerances,
          parameters: spec.tolerances.parameters.map((p, i) => (i === index ? { ...p, ...change } : p)),
        },
      });
    }
  };

  const measureUpdate = (index: number, change: Partial<StudySpec['measurements'][number]>) =>
    update({ measurements: spec.measurements.map((m, i) => (i === index ? { ...m, ...change } : m)) });

  const acceptSpec = (next: StudySpec, customSnapshot = true) => {
    setSpec(next);
    setHasCustomSnapshot(customSnapshot);
    setTemperatureText(next.temperatures_c.join(', '));
    setAdvancedJson('');
    setError('');
  };

  const useEditorCircuit = () => {
    acceptSpec(buildEditorSpec(source, name, resources), false);
    setStep('variables');
  };

  const startNewStudy = () => {
    if (
      results &&
      !globalThis.confirm(
        'Start a new study from the current editor circuit? Retained study results in this dialog will be cleared.',
      )
    )
      return;
    acceptSpec(buildEditorSpec(source, name, resources), false);
    setResults(null);
    setPreviousResults(null);
    setComparison(null);
    setSelected([]);
    setAnalysis(0);
    setSignal('V(OUT)');
    setPlot('');
    setPlotError('');
    setProgress('');
    setStep('method');
  };

  const mode = spec.tolerances?.mode ?? 'sweep';

  const changeMode = (next: string) => {
    if (next === 'sweep') {
      update({ tolerances: null, axes: parameters[0] ? [parameterAxis(parameters[0])] : [] });
    } else {
      update({
        axes: [],
        tolerances: {
          mode: next as Tolerance['mode'],
          seed: 42,
          samples: 12,
          parameters: parameters[0]
            ? [{ parameter: parameters[0].name, relative: 0.05, distribution: 'uniform', group: null }]
            : [],
        },
      });
    }
  };

  const runStudy = async (resume: boolean) => {
    if (active.current) return;
    active.current = true;
    cancelled.current = false;
    setRunning(true);
    setError('');
    setProgress('Preparing study…');
    let current: StudyResults | null = null;
    const bound = resources;
    const snapshot = spec;
    const solver = runner.current!;
    try {
      const plan = plan_study(snapshot, bound) as StudyPlan;
      if (plan.cases.length > 32) {
        throw new Error(
          `This study has ${plan.cases.length} cases. Browser limit: 32. Download the specification and run it with the CLI (up to 256 cases).`,
        );
      }
      const response = await fetch(coreWasmUrl);
      if (!response.ok) throw new Error('Could not identify the Core runtime');
      const hash = await crypto.subtle.digest('SHA-256', await response.arrayBuffer());
      const fingerprint = `eecircuit-engine@1.7.0:sha256:${Array.from(new Uint8Array(hash), b =>
        b.toString(16).padStart(2, '0'),
      ).join('')}`;
      if (cancelled.current) throw new SimulationCancelledError();
      setProgress('Identifying local simulator…');
      const probe = await solver.run(prepare_browser_simulation(probeSource) as BrowserSimulationPlan, {
        timeoutMs: 30_000,
      });
      if (!resume && results) {
        setPreviousResults(results);
        setComparison(null);
      }
      current =
        resume && results
          ? (resume_study_results(results, snapshot, bound, probe.simulator, fingerprint) as StudyResults)
          : (start_study_results(plan, probe.simulator, fingerprint) as StudyResults);
      setResults(current);
      setStep('results');
      setSelected(plan.cases.slice(0, 12).map(c => c.id));
      for (let index = 0; index < plan.cases.length; index++) {
        if (cancelled.current) break;
        if (reusable(current.cases[index].status)) continue;
        setProgress(`Running case ${index + 1} of ${plan.cases.length}…`);
        let row: StudyCaseResult;
        try {
          const simulationPlan = prepare_study_case(plan, index, bound) as BrowserSimulationPlan;
          const simulation = await solver.run(simulationPlan, { timeoutMs: snapshot.timeout_ms });
          row = evaluate_study_case(plan, index, simulation, bound, '', false) as StudyCaseResult;
        } catch (cause) {
          row = evaluate_study_case(
            plan,
            index,
            null,
            bound,
            message(cause),
            cancelled.current || cause instanceof SimulationCancelledError,
          ) as StudyCaseResult;
        }
        current = { ...current, cases: current.cases.map((previous, i) => (i === index ? row : previous)) };
        if (JSON.stringify(current).length > 32 * 1024 * 1024) {
          current.cases[index] = evaluate_study_case(
            plan,
            index,
            null,
            bound,
            'Browser retained-data limit reached (32 MiB). Narrow the study or use the CLI.',
            false,
          ) as StudyCaseResult;
          cancelled.current = true;
        }
        setResults(current);
      }
      current = inspect_study_results(current) as StudyResults;
      setResults(current);
      setStep('results');
      setProgress(
        cancelled.current
          ? 'Study stopped. Partial results retained; use Resume or download Results JSON.'
          : 'Study complete. Every case remains in the report.',
      );
    } catch (cause) {
      setError(message(cause));
      setProgress(
        cancelled.current
          ? 'Study stopped. Existing partial results are retained.'
          : 'Study not completed. See the diagnostic above.',
      );
      if (current) {
        setResults(inspect_study_results(current) as StudyResults);
        setStep('results');
      }
    } finally {
      active.current = false;
      setRunning(false);
    }
  };

  const cancel = () => {
    cancelled.current = true;
    runner.current?.cancel();
    setProgress('Stopping study…');
  };

  const handleClose = () => {
    if (active.current) cancel();
    onClose();
  };

  const importFile = async (file: File, compare: boolean) => {
    try {
      if (file.size > (compare ? 32 : 64) * 1024 * 1024) throw new Error('Study file is too large');
      const parsed = JSON.parse(await file.text()) as StudySpec | StudyResults;
      if (compare) {
        if (parsed.schema_version !== 'kessetsu.experiment-results.v1') {
          throw new Error('Choose Results JSON to compare');
        }
        setComparison(inspect_study_results(parsed) as StudyResults);
        setStep('results');
      } else if (parsed.schema_version === 'kessetsu.experiment-results.v1') {
        const report = inspect_study_results(parsed) as StudyResults;
        setResults(report);
        acceptSpec(report.plan.spec, true);
        setSelected(report.plan.cases.slice(0, 12).map(c => c.id));
        setStep('results');
      } else {
        acceptSpec((plan_study(parsed, resources) as StudyPlan).spec, true);
        setStep('variables');
      }
      setError('');
    } catch (cause) {
      setError(message(cause));
    }
  };

  const exportFile = () => {
    if (!results) return;
    try {
      const content =
        exportTarget === 'svg' && comparison
          ? compare_study_plot(results, comparison, analysis, signal)
          : export_study_results(results, exportTarget, analysis, signal, selected);
      const extension = exportTarget === 'data_csv' ? 'csv' : exportTarget;
      downloadTextFile(
        content,
        `${sanitizeFileStem(results.plan.spec.name)}${exportTarget === 'data_csv' ? '-samples' : ''}.${extension}`,
      );
      setError('');
    } catch (cause) {
      setError(message(cause));
    }
  };

  const downloadResultsJson = () => {
    if (!results) return;
    try {
      const content = export_study_results(results, 'json', analysis, signal, selected);
      downloadTextFile(content, `${sanitizeFileStem(results.plan.spec.name)}.json`);
      setError('');
    } catch (cause) {
      setError(message(cause));
    }
  };

  const downloadSummaryCsv = () => {
    if (!results) return;
    try {
      const content = export_study_results(results, 'csv', analysis, signal, selected);
      downloadTextFile(content, `${sanitizeFileStem(results.plan.spec.name)}.csv`);
      setError('');
    } catch (cause) {
      setError(message(cause));
    }
  };

  useEffect(() => {
    if (!results || running || selected.length === 0) {
      setPlot('');
      return;
    }
    try {
      setPlot(
        comparison
          ? compare_study_plot(results, comparison, analysis, signal)
          : export_study_results(results, 'svg', analysis, signal, selected),
      );
      setPlotError('');
    } catch (cause) {
      setPlot('');
      setPlotError(message(cause));
    }
  }, [results, comparison, analysis, signal, selected, running]);

  const firstSimulation = results?.cases.find(row => row.simulation)?.simulation;
  const dataset = firstSimulation?.datasets.find(data => data.index === analysis)?.data;
  const signals = dataset && dataset.kind !== 'operating_point' ? Object.keys(dataset.signals).map(signalLabel) : [];
  const counts = results?.cases.reduce(
    (acc, row) => {
      acc[row.status] = (acc[row.status] ?? 0) + 1;
      return acc;
    },
    {} as Record<string, number>,
  );

  const parameterOptions = parameters.map(p => (
    <option key={p.name} value={p.name}>
      {p.name} ({suffix[p.declared_unit] || 'ratio'})
    </option>
  ));

  const tryExample = () => {
    if (
      results &&
      !globalThis.confirm(
        'Replace this study and its retained results with the loaded-filter example? The editor circuit will not change.',
      )
    )
      return;
    try {
      const planned = plan_study(JSON.parse(loadedFilterStudy), resources) as StudyPlan;
      acceptSpec(planned.spec, true);
      setResults(null);
      setPreviousResults(null);
      setComparison(null);
      setSelected([]);
      setAnalysis(0);
      setSignal('V(OUT)');
      setPlot('');
      setPlotError('');
      setProgress('Example ready: six cases. Run the study to see both passing and failing designs.');
      setStep('variables');
      setError('');
    } catch (cause) {
      setError(message(cause));
    }
  };

  const hasValidVariables =
    mode === 'sweep' ? spec.axes.length > 0 : (spec.tolerances?.parameters.length ?? 0) > 0;
  const hasStudyableInputs = parameters.length > 0 || spec.revisions.length > 0 || hasValidVariables;

  const resultsModeLabel = results?.plan.spec.tolerances
    ? results.plan.spec.tolerances.mode === 'monte_carlo'
      ? 'Monte Carlo'
      : 'Tolerance corners'
    : 'Parameter sweep';

  const plannedCasesPreview = useMemo(() => {
    if (!spec.source.trim()) return { count: 0, error: 'Empty circuit source' };
    try {
      const plan = plan_study(spec, resources) as StudyPlan;
      return { count: plan.cases.length, error: null };
    } catch (cause) {
      return { count: 0, error: message(cause) };
    }
  }, [spec, resources]);

  const steps: WorkflowStep<StudyStep>[] = [
    {
      id: 'method',
      number: 1,
      label: 'Method',
      done: true,
    },
    {
      id: 'variables',
      number: 2,
      label: 'Variables',
      done: hasValidVariables,
    },
    {
      id: 'checks',
      number: 3,
      label: 'Checks',
      done: spec.measurements.length > 0 || Boolean(spec.objective) || Boolean(spec.requirements?.trim()),
    },
    {
      id: 'review',
      number: 4,
      label: 'Review & Run',
      done: Boolean(results),
    },
    {
      id: 'results',
      number: 5,
      label: 'Results',
      disabled: !results,
      done: Boolean(results),
    },
  ];

  const renderFooterActions = () => {
    if (running) {
      return (
        <button type="button" onClick={cancel}>
          <Square size={14} /> Stop study
        </button>
      );
    }

    const runButton = (
      <button
        type="button"
        className="primary"
        disabled={!hasStudyableInputs}
        onClick={() => void runStudy(false)}
      >
        <Play size={14} /> {results ? 'Run new study' : 'Run study'}
      </button>
    );

    const resumeButton = results && (
      <button
        type="button"
        disabled={results.cases.every(row => reusable(row.status))}
        onClick={() => void runStudy(true)}
      >
        Resume
      </button>
    );

    switch (step) {
      case 'method':
        return (
          <>
            {resumeButton}
            <button type="button" onClick={() => setStep('variables')}>
              Next: Variables <ArrowRight size={14} />
            </button>
            {runButton}
          </>
        );
      case 'variables':
        return (
          <>
            <button type="button" onClick={() => setStep('method')}>
              <ArrowLeft size={14} /> Back
            </button>
            {resumeButton}
            <button type="button" onClick={() => setStep('checks')}>
              Next: Checks <ArrowRight size={14} />
            </button>
            {runButton}
          </>
        );
      case 'checks':
        return (
          <>
            <button type="button" onClick={() => setStep('variables')}>
              <ArrowLeft size={14} /> Back
            </button>
            {resumeButton}
            <button type="button" onClick={() => setStep('review')}>
              Next: Review &amp; Run <ArrowRight size={14} />
            </button>
            {runButton}
          </>
        );
      case 'review':
        return (
          <>
            <button type="button" onClick={() => setStep('checks')}>
              <ArrowLeft size={14} /> Back
            </button>
            {resumeButton}
            {runButton}
          </>
        );
      case 'results':
        return (
          <>
            <button type="button" onClick={() => setStep('review')}>
              <ArrowLeft size={14} /> Back to review
            </button>
            <button type="button" onClick={() => setStep('variables')}>
              Edit variables
            </button>
            {resumeButton}
            {runButton}
          </>
        );
    }
  };

  return (
    <WorkflowDialog<StudyStep>
      open={open}
      onClose={handleClose}
      ariaLabel="Parameter study"
      closeLabel="Close parameter study"
      className="study-dialog"
      icon={<Sliders size={18} />}
      title="Parameter study"
      subtitle="Sweep parameters or tolerances on a circuit snapshot without modifying the editor."
      headerExtras={
        <p className="dialog-help-link">
          <a
            href={`${import.meta.env.BASE_URL}docs/guides/parameter-studies/`}
            target="_blank"
            rel="noreferrer"
          >
            How to set up a study and read its outcomes
          </a>
        </p>
      }
      stepperLabel="Study steps"
      steps={steps}
      activeStep={step}
      onStepSelect={setStep}
      stepperActions={
        <>
          <button type="button" onClick={startNewStudy} disabled={running}>
            <RotateCcw size={14} /> New study
          </button>
          <button type="button" onClick={() => importInput.current?.click()} disabled={running}>
            <Upload size={14} /> Open study / results…
          </button>
          <button
            type="button"
            className="study-example-button"
            aria-label="Try loaded-filter study"
            onClick={tryExample}
            disabled={running}
          >
            Open example
          </button>
        </>
      }
      contextBanner={
        <div className={`study-snapshot-bar${spec.source !== source ? ' study-snapshot-diverged' : ''}`}>
          {spec.source === source ? (
            <span>
              <strong>Current editor circuit:</strong> {name} · {parameters.length} sweepable{' '}
              {parameters.length === 1 ? 'parameter' : 'parameters'}
            </span>
          ) : (
            <>
              <span>
                <strong>Studying snapshot:</strong> {spec.name}{' '}
                <small>(editor circuit “{name}” differs)</small>
              </span>
              <button type="button" onClick={useEditorCircuit} disabled={running}>
                <RotateCcw size={13} /> Use editor circuit
              </button>
            </>
          )}
        </div>
      }
      status={progress || 'Local execution: up to 32 browser cases (use the CLI for larger studies).'}
      footerActions={renderFooterActions()}
    >
      <input
        ref={importInput}
        type="file"
        className="sr-only"
        accept=".json"
        aria-label="Open study file"
        onChange={event => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) void importFile(file, false);
        }}
      />
      <input
        ref={compareInput}
        type="file"
        className="sr-only"
        accept=".json"
        aria-label="Compare study results file"
        onChange={event => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) void importFile(file, true);
        }}
      />

      {error && (
        <p className="study-error" role="alert">
          {error}
        </p>
      )}

      {step === 'method' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Study method</h3>
              <p>Choose whether to step through explicit parameter values, compute min/max tolerance corners, or sample random Monte Carlo variations.</p>
            </div>
          </div>

          <div className="study-row">
            <label>
              Study name
              <input
                aria-label="Study name"
                value={spec.name}
                onChange={event => update({ name: event.target.value })}
              />
            </label>
            <label>
              Mode
              <select
                aria-label="Mode"
                value={mode}
                onChange={event => changeMode(event.target.value)}
              >
                <option value="sweep">Parameter sweep</option>
                <option value="corners">Nominal + tolerance corners</option>
                <option value="monte_carlo">Nominal + Monte Carlo samples</option>
              </select>
            </label>
          </div>

          <div className="workflow-card-grid">
            <button
              type="button"
              className={`workflow-choice-card ${mode === 'sweep' ? 'is-selected' : ''}`}
              aria-pressed={mode === 'sweep'}
              onClick={() => changeMode('sweep')}
            >
              <strong>Parameter sweep</strong>
              <span>Step through explicit parameter values or multi-parameter combinations.</span>
            </button>
            <button
              type="button"
              className={`workflow-choice-card ${mode === 'corners' ? 'is-selected' : ''}`}
              aria-pressed={mode === 'corners'}
              onClick={() => changeMode('corners')}
            >
              <strong>Nominal + tolerance corners</strong>
              <span>Run the nominal case plus every min/max tolerance corner.</span>
            </button>
            <button
              type="button"
              className={`workflow-choice-card ${mode === 'monte_carlo' ? 'is-selected' : ''}`}
              aria-pressed={mode === 'monte_carlo'}
              onClick={() => changeMode('monte_carlo')}
            >
              <strong>Nominal + Monte Carlo samples</strong>
              <span>Sample random parameter variations around nominal with a deterministic seed.</span>
            </button>
          </div>

          {parameters.length === 0 && !parameterError && spec.revisions.length === 0 && (
            <p className="study-note">
              No sweepable param declarations in this circuit. Proceed to <strong>Variables</strong> to inspect details or open a preconfigured example.
            </p>
          )}
        </div>
      )}

      {step === 'variables' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Variables</h3>
              <p>Configure which parameters vary across cases and how values are stepped.</p>
            </div>
          </div>

          {parameterError && (
            <p role="alert">
              {parameterError} Open the study source in the editor and choose its matching model files.
            </p>
          )}

          {parameters.length === 0 && !parameterError && spec.revisions.length === 0 ? (
            <div className="study-empty-params">
              <strong>No sweepable <code>param</code> declarations in this circuit</strong>
              <p className="study-note">
                Parameter studies vary top-level <code>param</code> values across sweeps, tolerance corners, or Monte Carlo runs. Declare a parameter in the editor and reference it on a component:
              </p>
              <pre className="study-param-example">{`param R_load = 1kOhm\nresistor R1 R_load`}</pre>
              <p className="study-note">
                Or choose <strong>Open example</strong> to inspect a 6-case filter study, or <strong>Open study / results…</strong> to load a saved <code>.kessstudy.json</code> file.
              </p>
              <button
                type="button"
                className="study-example-button"
                onClick={tryExample}
              >
                Open example
              </button>
            </div>
          ) : (
            <>
              {mode === 'sweep' && (
                <section>
                  <h3>Sweep parameters</h3>
                  {spec.axes.map((axis, index) => (
                    <div className="study-axis" key={index}>
                      <label>
                        Parameter
                        <select
                          aria-label={`Sweep parameter ${index + 1}`}
                          value={axis.parameter}
                          onChange={event => {
                            const parameter = parameters.find(p => p.name === event.target.value);
                            if (parameter) axisUpdate(index, parameterAxis(parameter));
                          }}
                        >
                          {parameterOptions}
                        </select>
                      </label>
                      <label>
                        Values
                        <select
                          aria-label={`Sweep method ${index + 1}`}
                          value={axis.values.kind}
                          onChange={event => {
                            const p = parameters.find(p => p.name === axis.parameter);
                            const value = p?.resolved.value ?? 1;
                            const u = p ? suffix[p.declared_unit] : '';
                            const values: AxisValues =
                              event.target.value === 'list'
                                ? { kind: 'list', values: [`${value}${u}`] }
                                : {
                                    kind: event.target.value as 'linear' | 'log',
                                    start: `${value * 0.5}${u}`,
                                    stop: `${value * 2}${u}`,
                                    points: 5,
                                  };
                            axisUpdate(index, { values });
                          }}
                        >
                          <option value="list">Explicit list</option>
                          <option value="linear">Linear range</option>
                          <option value="log">Log range</option>
                        </select>
                      </label>
                      {axis.values.kind === 'list' ? (
                        <label className="study-values">
                          Comma-separated values
                          <input
                            aria-label={`Sweep values ${index + 1}`}
                            value={axis.values.values.join(', ')}
                            onChange={event =>
                              axisUpdate(index, {
                                values: {
                                  kind: 'list',
                                  values: event.target.value.split(',').map(v => v.trim()),
                                },
                              })
                            }
                          />
                        </label>
                      ) : (
                        <>
                          <label>
                            Start
                            <input
                              aria-label={`Sweep start ${index + 1}`}
                              value={axis.values.start}
                              onChange={event => {
                                if (axis.values.kind !== 'list')
                                  axisUpdate(index, { values: { ...axis.values, start: event.target.value } });
                              }}
                            />
                          </label>
                          <label>
                            Stop
                            <input
                              aria-label={`Sweep stop ${index + 1}`}
                              value={axis.values.stop}
                              onChange={event => {
                                if (axis.values.kind !== 'list')
                                  axisUpdate(index, { values: { ...axis.values, stop: event.target.value } });
                              }}
                            />
                          </label>
                          <label>
                            Points
                            <input
                              aria-label={`Sweep points ${index + 1}`}
                              type="number"
                              min="2"
                              max="256"
                              value={axis.values.points}
                              onChange={event => {
                                if (axis.values.kind !== 'list')
                                  axisUpdate(index, {
                                    values: { ...axis.values, points: Number(event.target.value) },
                                  });
                              }}
                            />
                          </label>
                        </>
                      )}
                      <button
                        type="button"
                        aria-label={`Remove sweep axis ${index + 1}`}
                        onClick={() => update({ axes: spec.axes.filter((_, i) => i !== index) })}
                      >
                        <X size={14} />
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    disabled={spec.axes.length >= 8 || parameters.length === 0}
                    onClick={() => {
                      const p = parameters.find(p => !spec.axes.some(a => a.parameter === p.name));
                      if (p) update({ axes: [...spec.axes, parameterAxis(p)] });
                    }}
                  >
                    <Plus size={14} /> Add parameter
                  </button>
                </section>
              )}

              {mode === 'monte_carlo' && spec.tolerances && (
                <section>
                  <h3>Monte Carlo sampling</h3>
                  <div className="study-row">
                    <label>
                      Sample count
                      <input
                        aria-label="Sample count"
                        type="number"
                        min="1"
                        max="255"
                        value={spec.tolerances.samples}
                        onChange={event =>
                          update({
                            tolerances: { ...spec.tolerances!, samples: Number(event.target.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Random seed
                      <input
                        aria-label="Random seed"
                        type="number"
                        min="0"
                        max="4294967295"
                        value={spec.tolerances.seed}
                        onChange={event =>
                          update({
                            tolerances: { ...spec.tolerances!, seed: Number(event.target.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                  <p className="study-note">
                    Same-group parameters share a signed deviation. Gaussian samples are unbounded; invalid values stay failed cases.
                  </p>
                </section>
              )}

              {(mode === 'corners' || mode === 'monte_carlo') && spec.tolerances && (
                <section>
                  <h3>Tolerance corners</h3>
                  {spec.tolerances.parameters.map((p, index) => (
                    <div className="study-axis" key={index}>
                      <label>
                        Parameter
                        <select
                          aria-label={`Corner parameter ${index + 1}`}
                          value={p.parameter}
                          onChange={event => toleranceUpdate(index, { parameter: event.target.value })}
                        >
                          {parameterOptions}
                        </select>
                      </label>
                      <label>
                        {p.distribution === 'normal' ? 'Standard deviation (%)' : 'Half-width (±%)'}
                        <input
                          aria-label={`Corner tolerance ${index + 1}`}
                          type="number"
                          min="0.001"
                          max="99"
                          value={p.relative * 100}
                          onChange={event =>
                            toleranceUpdate(index, { relative: Number(event.target.value) / 100 })
                          }
                        />
                      </label>
                      {mode === 'monte_carlo' && (
                        <>
                          <label>
                            Distribution
                            <select
                              aria-label={`Corner mode ${index + 1}`}
                              value={p.distribution}
                              onChange={event =>
                                toleranceUpdate(index, {
                                  distribution: event.target.value as 'uniform' | 'normal',
                                })
                              }
                            >
                              <option value="uniform">Uniform</option>
                              <option value="normal">Gaussian</option>
                            </select>
                          </label>
                          <label>
                            Correlation group (optional)
                            <input
                              aria-label={`Corner group ${index + 1}`}
                              value={p.group ?? ''}
                              onChange={event =>
                                toleranceUpdate(index, { group: event.target.value || null })
                              }
                            />
                          </label>
                        </>
                      )}
                      <button
                        type="button"
                        aria-label={`Remove tolerance ${index + 1}`}
                        onClick={() =>
                          update({
                            tolerances: {
                              ...spec.tolerances!,
                              parameters: spec.tolerances!.parameters.filter((_, i) => i !== index),
                            },
                          })
                        }
                      >
                        <X size={14} />
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    disabled={spec.tolerances.parameters.length >= 8 || parameters.length === 0}
                    onClick={() => {
                      const p = parameters.find(
                        param => !spec.tolerances!.parameters.some(t => t.parameter === param.name),
                      );
                      if (p)
                        update({
                          tolerances: {
                            ...spec.tolerances!,
                            parameters: [
                              ...spec.tolerances!.parameters,
                              { parameter: p.name, relative: 0.05, distribution: 'uniform', group: null },
                            ],
                          },
                        });
                    }}
                  >
                    <Plus size={14} /> Add tolerance
                  </button>
                </section>
              )}
            </>
          )}
        </div>
      )}

      {step === 'checks' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Checks and conditions</h3>
              <p>Encoded <code>assert</code> statements run automatically on every case. Define optional metric measurements, objectives, or requirement files below.</p>
            </div>
          </div>

          <section>
            <h3>Measurements (optional)</h3>
            <p className="study-note">
              Use metric expressions without comparisons to record numerical outputs across all evaluated cases.
            </p>
            {spec.measurements.map((m, index) => (
              <div className="study-axis" key={index}>
                <label>
                  Name
                  <input
                    aria-label={`Measurement id ${index + 1}`}
                    value={m.name}
                    onChange={event => measureUpdate(index, { name: event.target.value })}
                  />
                </label>
                <label className="study-values">
                  Metric expression
                  <input
                    aria-label={`Target ${index + 1}`}
                    placeholder="cutoff(V(OUT),V(IN))"
                    value={m.expression}
                    onChange={event => measureUpdate(index, { expression: event.target.value })}
                  />
                </label>
                <label>
                  Unit
                  <select
                    aria-label={`Unit ${index + 1}`}
                    value={m.unit}
                    onChange={event => measureUpdate(index, { unit: event.target.value as Unit })}
                  >
                    {Object.keys(suffix).map(unit => (
                      <option key={unit} value={unit}>
                        {unit}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  type="button"
                  aria-label={`Remove measurement ${index + 1}`}
                  onClick={() =>
                    update({
                      measurements: spec.measurements.filter((_, i) => i !== index),
                      objective: spec.objective?.measurement === m.name ? null : spec.objective,
                    })
                  }
                >
                  <X size={14} />
                </button>
              </div>
            ))}
            <button
              type="button"
              onClick={() =>
                update({
                  measurements: [
                    ...spec.measurements,
                    { name: `metric_${spec.measurements.length + 1}`, expression: 'cutoff(V(OUT),V(IN))', unit: 'Hertz' },
                  ],
                })
              }
            >
              <Plus size={14} /> Add measurement
            </button>
          </section>

          <section>
            <h3>Search objective (optional)</h3>
            <div className="study-row">
              <label>
                Objective measurement
                <select
                  aria-label="Objective measurement"
                  value={spec.objective?.measurement ?? ''}
                  onChange={event =>
                    update({
                      objective: event.target.value
                        ? { measurement: event.target.value, direction: 'minimize' }
                        : null,
                    })
                  }
                >
                  <option value="">None — compare all cases</option>
                  {spec.measurements.map(m => (
                    <option key={m.name} value={m.name}>
                      {m.name}
                    </option>
                  ))}
                </select>
              </label>
              {spec.objective && (
                <label>
                  Goal
                  <select
                    aria-label="Goal"
                    value={spec.objective.direction}
                    onChange={event =>
                      update({
                        objective: {
                          ...spec.objective!,
                          direction: event.target.value as 'minimize' | 'maximize',
                        },
                      })
                    }
                  >
                    <option value="minimize">Minimize</option>
                    <option value="maximize">Maximize</option>
                  </select>
                </label>
              )}
            </div>
            <p className="study-note">
              Search selects only the best feasible candidate. Only cases where all assertions and requirements pass are considered.
            </p>
          </section>

          <details>
            <summary>Independent requirement constraints (.kessreq)</summary>
            <div className="study-row">
              <label>
                Requirement file name
                <input
                  aria-label="Requirement file name"
                  value={reqFileName}
                  onChange={event => setReqFileName(event.target.value)}
                  placeholder="study.kessreq"
                />
              </label>
            </div>
            <label>
              Requirement statements (.kessreq)
              <textarea
                aria-label="Requirement statements (.kessreq)"
                rows={3}
                value={spec.requirements ?? ''}
                placeholder="assert gain_at(V(OUT),V(IN),1kHz) > 0.6"
                onChange={event => update({ requirements: event.target.value || null })}
              />
            </label>
            <p className="study-note">
              Independent requirements cannot mix with inline assertions or reference design parameters.
            </p>
          </details>

          <details>
            <summary>Execution conditions &amp; advanced specification</summary>
            <div className="study-row">
              <label>
                Temperatures (°C)
                <input
                  aria-label="Temperatures (°C)"
                  value={temperatureText}
                  onChange={event => {
                    setTemperatureText(event.target.value);
                    update({
                      temperatures_c: event.target.value.split(',').map(v => (v.trim() ? Number(v.trim()) : NaN)),
                    });
                  }}
                />
              </label>
              <label>
                Per-case timeout (ms)
                <input
                  aria-label="Per-case timeout (ms)"
                  type="number"
                  min="1"
                  max="120000"
                  value={spec.timeout_ms}
                  onChange={event => update({ timeout_ms: Math.round(Number(event.target.value)) })}
                />
              </label>
            </div>
            <p className="study-note">
              Temperature affects only models with temperature-dependent equations. Ideal passives and generic op-amps do not acquire new temperature coefficients.
            </p>
            <button
              type="button"
              onClick={() => {
                try {
                  const checked = (plan_study(spec, resources) as StudyPlan).spec;
                  downloadTextFile(
                    JSON.stringify(checked, null, 2),
                    `${sanitizeFileStem(checked.name)}.kessstudy.json`,
                  );
                  setError('');
                } catch (cause) {
                  setError(message(cause));
                }
              }}
            >
              <Download size={14} /> Download specification
            </button>

            <details>
              <summary>Advanced JSON: revisions and combined studies</summary>
              <label className="study-checkbox-label">
                <input
                  type="checkbox"
                  checked={useAdvancedJson}
                  onChange={e => setUseAdvancedJson(e.target.checked)}
                />
                Use advanced JSON specification
              </label>
              <textarea
                rows={10}
                aria-label="Advanced study specification"
                value={advancedJson || JSON.stringify(spec, null, 2)}
                onChange={event => setAdvancedJson(event.target.value)}
              />
              <button
                type="button"
                aria-label="Apply JSON"
                onClick={() => {
                  try {
                    acceptSpec(
                      (plan_study(JSON.parse(advancedJson || JSON.stringify(spec)), resources) as StudyPlan).spec,
                      true,
                    );
                  } catch (cause) {
                    setError(message(cause));
                  }
                }}
              >
                Apply JSON
              </button>
            </details>
          </details>
        </div>
      )}

      {step === 'review' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Review and run study</h3>
              <p>Verify planned cases, variable axes, and checks before running simulations.</p>
            </div>
          </div>

          <div className="study-review-grid">
            <div className="study-review-card">
              <div className="study-review-card-header">
                <strong>Circuit snapshot</strong>
              </div>
              <div className="study-review-card-body">
                {spec.source === source ? (
                  <p>
                    Current editor circuit: <strong>{name}</strong>
                  </p>
                ) : (
                  <div>
                    <p>
                      Studying snapshot: <strong>{spec.name}</strong>{' '}
                      <small>(differs from editor circuit “{name}”)</small>
                    </p>
                    <button type="button" onClick={useEditorCircuit} disabled={running}>
                      <RotateCcw size={13} /> Use editor circuit
                    </button>
                  </div>
                )}
              </div>
            </div>

            <div className="study-review-card">
              <div className="study-review-card-header">
                <strong>Method</strong>
              </div>
              <div className="study-review-card-body">
                <p>
                  <strong>
                    {mode === 'sweep'
                      ? 'Parameter sweep'
                      : mode === 'corners'
                      ? 'Nominal + tolerance corners'
                      : 'Nominal + Monte Carlo samples'}
                  </strong>
                </p>
                <p className="study-review-highlight">
                  {plannedCasesPreview.error ? (
                    <span className="study-error">{plannedCasesPreview.error}</span>
                  ) : (
                    <span>{plannedCasesPreview.count} planned cases</span>
                  )}
                </p>
                {plannedCasesPreview.count > 32 && (
                  <p className="study-note study-error">
                    Browser limit is 32 cases. Download the specification and run with the CLI for larger studies.
                  </p>
                )}
              </div>
            </div>

            <div className="study-review-card">
              <div className="study-review-card-header">
                <strong>Variables</strong>
                <button type="button" onClick={() => setStep('variables')} disabled={running}>
                  Edit variables
                </button>
              </div>
              <div className="study-review-card-body">
                {mode === 'sweep' ? (
                  spec.axes.length > 0 ? (
                    <ul className="study-review-list">
                      {spec.axes.map((axis, i) => (
                        <li key={i}>
                          <code>{axis.parameter}</code>:{' '}
                          {axis.values.kind === 'list'
                            ? `${axis.values.values.length} explicit values (${axis.values.values.join(', ')})`
                            : `${axis.values.points} points (${axis.values.start} to ${axis.values.stop})`}
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="study-note">No sweep axes configured.</p>
                  )
                ) : spec.tolerances && spec.tolerances.parameters.length > 0 ? (
                  <ul className="study-review-list">
                    {spec.tolerances.parameters.map((p, i) => (
                      <li key={i}>
                        <code>{p.parameter}</code>: ±{(p.relative * 100).toFixed(1)}%{' '}
                        {mode === 'monte_carlo' ? `(${p.distribution})` : ''}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="study-note">No tolerances configured.</p>
                )}
              </div>
            </div>

            <div className="study-review-card">
              <div className="study-review-card-header">
                <strong>Checks &amp; conditions</strong>
                <button type="button" onClick={() => setStep('checks')} disabled={running}>
                  Edit checks
                </button>
              </div>
              <div className="study-review-card-body">
                <ul className="study-review-list">
                  <li>
                    <strong>Measurements:</strong>{' '}
                    {spec.measurements.length > 0
                      ? spec.measurements.map(m => m.name).join(', ')
                      : 'None (circuit assertions only)'}
                  </li>
                  <li>
                    <strong>Objective:</strong>{' '}
                    {spec.objective
                      ? `${spec.objective.direction} ${spec.objective.measurement}`
                      : 'None — compare all cases'}
                  </li>
                  <li>
                    <strong>Constraints:</strong>{' '}
                    {spec.requirements?.trim() ? 'Literal constraints defined' : 'Circuit assertions only'}
                  </li>
                  <li>
                    <strong>Temperatures:</strong> {spec.temperatures_c.join(', ')} °C
                  </li>
                  <li>
                    <strong>Per-case timeout:</strong> {spec.timeout_ms} ms
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      )}

      {step === 'results' && results && (
        <section className="study-results" aria-label="Study results">
          <div className="study-results-header">
            <div className="study-results-provenance">
              <strong>{results.plan.spec.name}</strong>
              <span className="study-badge">{resultsModeLabel}</span>
              <span className="study-badge">
                {results.cases.filter(c => c.status !== 'pending').length}/{results.plan.cases.length} cases
              </span>
              <small>
                {results.simulator.executable} {results.simulator.version}
              </small>
            </div>
            <div className="study-results-actions">
              <button type="button" disabled={running} onClick={() => setStep('variables')}>
                Edit setup
              </button>
              {previousResults && !comparison && (
                <button type="button" disabled={running} onClick={() => setComparison(previousResults)}>
                  Compare with previous run
                </button>
              )}
              <button type="button" disabled={running} onClick={() => compareInput.current?.click()}>
                Compare another Results JSON…
              </button>
              <button type="button" disabled={running} onClick={downloadResultsJson}>
                Download Results JSON
              </button>
              <button type="button" disabled={running} onClick={downloadSummaryCsv}>
                Download CSV
              </button>
            </div>
          </div>

          <p className="study-summary" data-testid="study-summary">
            {results.plan.cases.length} total · {counts?.passed ?? 0} passed ·{' '}
            {counts?.completed ?? 0} completed without constraints · {counts?.failed ?? 0} failed ·{' '}
            {counts?.error ?? 0} errors · {counts?.cancelled ?? 0} cancelled · {counts?.pending ?? 0} pending
          </p>

          {!running && results.summary.best_case && (
            <p>
              Best feasible evaluated case: <strong>{results.summary.best_case}</strong>. Search budget:{' '}
              {results.summary.total} cases.
            </p>
          )}

          {!running && results.plan.spec.objective && !results.summary.best_case && (
            <p>No feasible evaluated candidate met the study constraints.</p>
          )}

          <div className="study-row">
            <label>
              Analysis
              <select
                aria-label="Analysis"
                value={analysis}
                onChange={event => setAnalysis(Number(event.target.value))}
              >
                {firstSimulation?.datasets.map(d => (
                  <option key={d.index} value={d.index}>
                    {d.index + 1}: {d.analysis.kind}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Overlay signal
              <select
                aria-label="Overlay signal"
                value={signal}
                onChange={event => setSignal(event.target.value)}
              >
                <option value={signal}>{signal}</option>
                {signals
                  .filter(s => s.toLowerCase() !== signal.toLowerCase())
                  .map(s => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
              </select>
            </label>
            {comparison && (
              <button type="button" onClick={() => setComparison(null)}>
                Clear comparison
              </button>
            )}
          </div>

          {comparison && (
            <p>
              Comparing with {comparison.plan.spec.name}. Up to six cases per report are plotted; original files retain every case.
            </p>
          )}

          {plot && <div className="study-plot" data-testid="study-plot" dangerouslySetInnerHTML={{ __html: plot }} />}
          {!running && plotError && <p className="study-note">{plotError}</p>}

          <div className="study-table">
            <table>
              <thead>
                <tr>
                  <th>Overlay</th>
                  <th>Case / conditions</th>
                  <th>Status</th>
                  <th>Measured outcomes</th>
                  <th>Continue</th>
                </tr>
              </thead>
              <tbody>
                {results.plan.cases.map((point, index) => {
                  const row = results.cases[index];
                  return (
                    <tr key={point.id} className={`study-case-${row.status}`}>
                      <td>
                        <input
                          type="checkbox"
                          aria-label={`Overlay ${point.id}`}
                          title={`Plot ${point.id}`}
                          checked={selected.includes(point.id)}
                          disabled={running || comparison !== null}
                          onChange={event =>
                            setSelected(current =>
                              event.target.checked
                                ? [...current, point.id].slice(0, 12)
                                : current.filter(id => id !== point.id),
                            )
                          }
                        />
                      </td>
                      <td>
                        <strong>{point.id}</strong>
                        <br />
                        {point.name} · {point.temperature_c} °C
                        <br />
                        <code>
                          {Object.entries(point.parameters)
                            .map(([k, v]) => `${k}=${v}`)
                            .join(', ') || 'Nominal parameters'}
                        </code>
                      </td>
                      <td>{row.status}</td>
                      <td>
                        {Object.entries(row.measurements).map(([mName, m]) => (
                          <div key={mName}>
                            {mName}:{' '}
                            {m.value === null ? m.error : `${m.value.toPrecision(6)} ${suffix[m.unit]}`}
                          </div>
                        ))}
                        {row.assertions?.assertions.map(a => (
                          <div key={a.code}>
                            {a.metric}({a.signal}): {a.actual?.toPrecision(6) ?? 'unavailable'} · {a.status}
                            {a.message && ` — ${a.message}`}
                          </div>
                        ))}
                        {row.errors.map((e, i) => (
                          <div key={i}>{e}</div>
                        ))}
                      </td>
                      <td>
                        <button
                          type="button"
                          title={`Open case ${point.id} in editor`}
                          disabled={running || row.status === 'pending' || row.status === 'error'}
                          onClick={() => {
                            try {
                              if (
                                !globalThis.confirm(
                                  `Apply this case's parameters to the open source? Unsaved source will be replaced. Study temperature (${point.temperature_c} °C), independent constraints and results stay in the study, not ordinary circuit simulation.`,
                                )
                              )
                                return;
                              onApplySource(study_effective_source(results.plan, index, resources));
                              handleClose();
                            } catch (cause) {
                              setError(message(cause));
                            }
                          }}
                        >
                          Apply parameters
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="study-row">
            <label>
              Export current study
              <select
                aria-label="Export current study"
                value={exportTarget}
                onChange={event => setExportTarget(event.target.value)}
              >
                <option value="json">Results JSON — full data + resume</option>
                <option value="csv">Measurement summary CSV</option>
                <option value="data_csv">Full numeric samples CSV</option>
                <option value="svg">Visible overlay SVG</option>
                <option value="html">Printable HTML report</option>
              </select>
            </label>
            <button type="button" disabled={running} onClick={exportFile}>
              <Download size={14} /> Download
            </button>
          </div>
        </section>
      )}
    </WorkflowDialog>
  );
}
