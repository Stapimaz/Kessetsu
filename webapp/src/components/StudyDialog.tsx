import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Play,
  Square,
  RotateCcw,
  Upload,
  ArrowRight,
  ArrowLeft,
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
  type StudySpec,
  type StudyPlan,
  type StudyResults,
  type StudyParameter,
  type StudyCaseResult,
} from '../experiment';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserSimulationPlan } from '../simulation/types';
import { WorkflowDialog, type WorkflowStep } from './WorkflowDialog';
import {
  StudyMethodStep,
  StudyVariablesStep,
  StudyChecksStep,
  StudyReviewStep,
  StudyResultsStep,
  type StudyStep,
} from './StudySteps';
import './StudyDialog.css';
import loadedFilterStudy from '../../../examples/studies/loaded-filter.kessstudy.json?raw';

export type { StudyStep };

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

  const handleDownloadSpec = () => {
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
  };

  const handleApplyJson = () => {
    try {
      acceptSpec(
        (plan_study(JSON.parse(advancedJson || JSON.stringify(spec)), resources) as StudyPlan).spec,
        true,
      );
    } catch (cause) {
      setError(message(cause));
    }
  };

  const handleApplyCase = (index: number) => {
    if (!results) return;
    const point = results.plan.cases[index];
    if (!point) return;
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
  };

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
        <StudyMethodStep
          spec={spec}
          mode={mode}
          parameters={parameters}
          parameterError={parameterError}
          onUpdate={update}
          onChangeMode={changeMode}
        />
      )}

      {step === 'variables' && (
        <StudyVariablesStep
          spec={spec}
          mode={mode}
          parameters={parameters}
          parameterError={parameterError}
          onUpdate={update}
          onAxisUpdate={axisUpdate}
          onToleranceUpdate={toleranceUpdate}
          onTryExample={tryExample}
        />
      )}

      {step === 'checks' && (
        <StudyChecksStep
          spec={spec}
          reqFileName={reqFileName}
          setReqFileName={setReqFileName}
          temperatureText={temperatureText}
          setTemperatureText={setTemperatureText}
          useAdvancedJson={useAdvancedJson}
          setUseAdvancedJson={setUseAdvancedJson}
          advancedJson={advancedJson}
          setAdvancedJson={setAdvancedJson}
          onUpdate={update}
          onMeasureUpdate={measureUpdate}
          onDownloadSpec={handleDownloadSpec}
          onApplyJson={handleApplyJson}
        />
      )}

      {step === 'review' && (
        <StudyReviewStep
          spec={spec}
          source={source}
          name={name}
          running={running}
          mode={mode}
          plannedCasesPreview={plannedCasesPreview}
          onUseEditorCircuit={useEditorCircuit}
          onSetStep={setStep}
        />
      )}

      {step === 'results' && results && (
        <StudyResultsStep
          results={results}
          running={running}
          resultsModeLabel={resultsModeLabel}
          previousResults={previousResults}
          comparison={comparison}
          counts={counts}
          firstSimulation={firstSimulation}
          analysis={analysis}
          setAnalysis={setAnalysis}
          signal={signal}
          setSignal={setSignal}
          signals={signals}
          setComparison={setComparison}
          plot={plot}
          plotError={plotError}
          selected={selected}
          setSelected={setSelected}
          exportTarget={exportTarget}
          setExportTarget={setExportTarget}
          onSetStep={setStep}
          onCompareInputClick={() => compareInput.current?.click()}
          onDownloadResultsJson={downloadResultsJson}
          onDownloadSummaryCsv={downloadSummaryCsv}
          onExportFile={exportFile}
          onApplyCase={handleApplyCase}
        />
      )}
    </WorkflowDialog>
  );
}
