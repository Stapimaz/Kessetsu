import { BarChart3, Plus, RotateCcw, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import {
  compare_research_data,
  import_research_csv,
  import_simulation_research_data,
  preview_research_csv,
} from 'kessetsu-core';
import { downloadTextFile, sanitizeFileStem } from '../document';
import {
  emptySlot,
  engineering,
  logicalName,
  mappingSpec,
  parseOptional,
  type CsvDialect,
  type CsvPreview,
  type DataComparison,
  type DatasetSlot,
  type ResearchData,
  type SignalPair,
} from '../research';
import type { BrowserEvaluation, Dataset } from '../simulation/types';
import { WorkflowDialog, type WorkflowStep } from './WorkflowDialog';
import { ComparisonPlot, CsvDatasetImporter } from './ResearchDataParts';
import './ResearchDataDialog.css';

type SlotId = 'data' | 'reference';
export type Step = 'data' | 'reference' | 'match' | 'results';

interface Props {
  open: boolean;
  evaluation: BrowserEvaluation | null;
  circuitName: string;
  onClose: () => void;
}

function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function seriesSignals(dataset: Dataset | undefined): string[] {
  if (!dataset || dataset.kind === 'operating_point') return [];
  return Object.keys(dataset.signals);
}

function analysisLabel(dataset: Dataset, index: number) {
  if (dataset.kind === 'transient') return `${index + 1}: Transient`;
  if (dataset.kind === 'ac') return `${index + 1}: AC magnitude`;
  if (dataset.kind === 'dc_sweep') return `${index + 1}: DC sweep`;
  return `${index + 1}: Operating point`;
}

export function ResearchDataDialog({ open, evaluation, circuitName, onClose }: Props) {
  const [step, setStep] = useState<Step>('data');
  const [observedSlot, setObservedSlot] = useState(() => emptySlot('Observed dataset'));
  const [referenceSlot, setReferenceSlot] = useState(() => emptySlot('Reference dataset'));
  const [pairs, setPairs] = useState<SignalPair[]>([{ data_signal: '', reference_signal: '' }]);
  const [comparisonName, setComparisonName] = useState('Observed data versus reference');
  const [coverage, setCoverage] = useState<'require_full' | 'overlap_only'>('require_full');
  const [interpolation, setInterpolation] = useState<'linear' | 'log_axis'>('linear');
  const [shift, setShift] = useState('0');
  const [windowStart, setWindowStart] = useState('');
  const [windowStop, setWindowStop] = useState('');
  const [maxGap, setMaxGap] = useState('');
  const [comparisonResult, setComparisonResult] = useState<DataComparison | null>(null);
  const [comparisonError, setComparisonError] = useState('');
  const [selectedOverlayIndex, setSelectedOverlayIndex] = useState(0);
  const [observedNotes, setObservedNotes] = useState('');
  const [referenceNotes, setReferenceNotes] = useState('');

  const eligibleAnalyses = useMemo(
    () => evaluation?.simulation.datasets.filter((item) => item.data.kind !== 'operating_point') ?? [],
    [evaluation]
  );
  const [simulationAnalysis, setSimulationAnalysis] = useState(0);
  const selectedAnalysis = useMemo(
    () => eligibleAnalyses.find((item) => item.index === simulationAnalysis) ?? eligibleAnalyses[0],
    [eligibleAnalyses, simulationAnalysis]
  );
  const availableSimulationSignals = useMemo(() => seriesSignals(selectedAnalysis?.data), [selectedAnalysis]);
  const [simulationSignals, setSimulationSignals] = useState<Array<{ vector: string; name: string }>>([]);
  const [simulationError, setSimulationError] = useState('');

  const observed = useMemo(() => ({
    ...observedSlot,
    rawText: observedSlot.csv,
  }), [observedSlot]);

  const reference = useMemo(() => ({
    ...referenceSlot,
    rawText: referenceSlot.csv,
  }), [referenceSlot]);

  useEffect(() => {
    if (!selectedAnalysis) {
      setSimulationSignals([]);
      return;
    }
    if (selectedAnalysis.index !== simulationAnalysis) {
      setSimulationAnalysis(selectedAnalysis.index);
    }
    setSimulationSignals((current) => {
      const valid = current.filter((mapping) => availableSimulationSignals.includes(mapping.vector));
      if (valid.length > 0) return valid;
      const vector = availableSimulationSignals[0];
      return vector ? [{ vector, name: logicalName(vector.replace(/[()]/g, '_'), 'signal') }] : [];
    });
  }, [selectedAnalysis, simulationAnalysis, availableSimulationSignals]);

  const setSlot = (id: SlotId, next: DatasetSlot | ((current: DatasetSlot) => DatasetSlot)) => {
    const setter = id === 'data' ? setObservedSlot : setReferenceSlot;
    setter(next);
    setComparisonResult(null);
    setComparisonError('');
  };

  const startNewComparison = () => {
    setObservedSlot(emptySlot('Observed dataset'));
    setReferenceSlot(emptySlot('Reference dataset'));
    setPairs([{ data_signal: '', reference_signal: '' }]);
    setComparisonName('Observed data versus reference');
    setCoverage('require_full');
    setInterpolation('linear');
    setShift('0');
    setWindowStart('');
    setWindowStop('');
    setMaxGap('');
    setComparisonResult(null);
    setComparisonError('');
    setSimulationError('');
    setSelectedOverlayIndex(0);
    setObservedNotes('');
    setReferenceNotes('');
    setStep('data');
  };

  const preview = (id: SlotId, dialect: CsvDialect, csv = (id === 'data' ? observedSlot.csv : referenceSlot.csv)) => {
    if (!csv) return;
    try {
      const result = preview_research_csv(csv, dialect) as CsvPreview;
      setSlot(id, (current) => ({ ...current, dialect, preview: result, error: '' }));
    } catch (cause) {
      setSlot(id, (current) => ({ ...current, dialect, preview: null, dataset: null, error: message(cause) }));
    }
  };

  const read = async (id: SlotId, file: File) => {
    if (file.size === 0 || file.size > 8 * 1024 * 1024) {
      setSlot(id, (current) => ({
        ...current,
        file,
        csv: '',
        preview: null,
        dataset: null,
        error: 'CSV must contain 1 byte-8 MiB',
      }));
      return;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const csv = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes);
      const current = id === 'data' ? observedSlot : referenceSlot;
      const result = preview_research_csv(csv, current.dialect) as CsvPreview;
      const headers = result.headers.length
        ? result.headers
        : (result.sample[0]?.fields ?? []).map((_, index) => `column_${index + 1}`);
      const axisName = logicalName(headers[0] ?? '', 'axis');
      const signalName = logicalName(headers[1] ?? '', 'signal');
      setSlot(id, {
        ...current,
        file,
        csv,
        preview: result,
        dataset: null,
        error: '',
        draft: {
          ...current.draft,
          name: file.name.replace(/\.csv$/i, '') || current.draft.name,
          axis: { ...current.draft.axis, column: 0, name: axisName },
          signals: [{ ...current.draft.signals[0], column: headers.length > 1 ? 1 : 0, name: signalName }],
        },
      });
    } catch (cause) {
      setSlot(id, (current) => ({
        ...current,
        file,
        csv: '',
        preview: null,
        dataset: null,
        error: `Could not read CSV: ${message(cause)}`,
      }));
    }
  };

  const importCsv = (target: 'observed' | 'reference') => {
    const id: SlotId = target === 'observed' ? 'data' : 'reference';
    const current = target === 'observed' ? observedSlot : referenceSlot;
    try {
      const dataset = import_research_csv(current.csv, mappingSpec(current)) as ResearchData;
      setSlot(id, { ...current, dataset, error: '' });
      if (target === 'observed') {
        setStep('reference');
      } else {
        setStep('match');
      }
    } catch (cause) {
      setSlot(id, { ...current, dataset: null, error: message(cause) });
    }
  };

  const useSimulationAsReference = () => {
    if (!evaluation || !selectedAnalysis) return;
    try {
      const dataset = import_simulation_research_data(evaluation.simulation, {
        schema_version: 'kessetsu.simulation-data-import.v1',
        name: `${circuitName} simulation`,
        analysis_index: selectedAnalysis.index,
        signals: simulationSignals,
      }) as ResearchData;
      setReferenceSlot((current) => ({
        ...current,
        file: null,
        csv: '',
        preview: null,
        dataset,
        error: '',
        draft: { ...current.draft, name: dataset.spec.name, origin: 'simulation' },
      }));
      setComparisonResult(null);
      setComparisonError('');
      setSimulationError('');
      setStep('match');
    } catch (cause) {
      setSimulationError(message(cause));
    }
  };

  useEffect(() => {
    if (!observed.dataset || !reference.dataset) return;
    const first = pairs[0];
    if (
      !first ||
      !first.data_signal ||
      !observed.dataset.signals.some((signal) => signal.name === first.data_signal) ||
      !reference.dataset.signals.some((signal) => signal.name === first.reference_signal)
    ) {
      setPairs([
        {
          data_signal: observed.dataset.signals[0]?.name ?? '',
          reference_signal: reference.dataset.signals[0]?.name ?? '',
        },
      ]);
    }
  }, [observed.dataset, reference.dataset, pairs]);

  const canCompare = Boolean(observed.dataset && reference.dataset);

  const runComparison = () => {
    if (!observed.dataset || !reference.dataset) return;
    try {
      const start = parseOptional(windowStart, 'Window start');
      const stop = parseOptional(windowStop, 'Window stop');
      if ((start === null) !== (stop === null)) {
        throw new Error('Set both window start and stop, or leave both empty');
      }
      const spec = {
        schema_version: 'kessetsu.data-comparison.v1',
        name: comparisonName,
        signals: pairs,
        coverage,
        interpolation,
        reference_axis_shift: parseOptional(shift, 'Reference axis shift') ?? 0,
        window: start === null ? null : [start, stop],
        max_reference_gap: parseOptional(maxGap, 'Maximum reference gap'),
      };
      const rawResult = compare_research_data(observed.dataset, reference.dataset, spec);
      const result = (typeof rawResult === 'string' ? JSON.parse(rawResult) : rawResult) as DataComparison;
      setComparisonResult(result);
      setComparisonError('');
      setSelectedOverlayIndex(0);
      setStep('results');
    } catch (cause) {
      setComparisonResult(null);
      setComparisonError(message(cause));
    }
  };

  const downloadEvidence = () => {
    if (!comparisonResult) return;
    downloadTextFile(
      JSON.stringify(comparisonResult, null, 2),
      `${sanitizeFileStem(comparisonResult.spec.name)}.kesscompare.json`
    );
  };

  const updateDraft = (id: SlotId, change: Partial<DatasetSlot['draft']>) => {
    setSlot(id, (current) => ({
      ...current,
      dataset: null,
      error: '',
      draft: { ...current.draft, ...change },
    }));
  };

  const updateDialect = (id: SlotId, change: Partial<CsvDialect>) => {
    const current = id === 'data' ? observedSlot : referenceSlot;
    const dialect = { ...current.dialect, ...change };
    setSlot(id, (prev) => ({ ...prev, dialect, dataset: null, error: '' }));
    if (current.csv) preview(id, dialect);
  };

  const observedHeaders = observed.preview?.headers.length
    ? observed.preview.headers
    : (observed.preview?.sample[0]?.fields ?? []).map((_, index) => `Column ${index + 1}`);

  const referenceHeaders = reference.preview?.headers.length
    ? reference.preview.headers
    : (reference.preview?.sample[0]?.fields ?? []).map((_, index) => `Column ${index + 1}`);

  const totalRetained = useMemo(
    () => (observed.dataset?.axis.values.length ?? 0) + (reference.dataset?.axis.values.length ?? 0),
    [observed.dataset, reference.dataset]
  );

  const selectedSignal = comparisonResult?.signals[selectedOverlayIndex];
  const hasSimDatasets = eligibleAnalyses.length > 0;

  const observedName = (observed.dataset as unknown as { dataset_name?: string })?.dataset_name ?? observed.dataset?.spec.name;
  const referenceName = (reference.dataset as unknown as { dataset_name?: string })?.dataset_name ?? reference.dataset?.spec.name;
  const comparisonTitle = (comparisonResult as unknown as { comparison_name?: string })?.comparison_name ?? comparisonResult?.spec.name;

  const steps: WorkflowStep<Step>[] = [
    {
      id: 'data',
      number: 1,
      label: 'Observed',
      done: Boolean(observed.dataset),
      badge: observedName,
    },
    {
      id: 'reference',
      number: 2,
      label: 'Reference',
      done: Boolean(reference.dataset),
      badge: referenceName,
    },
    {
      id: 'match',
      number: 3,
      label: 'Match',
      disabled: !canCompare,
      done: Boolean(comparisonResult),
    },
    {
      id: 'results',
      number: 4,
      label: 'Results',
      disabled: !comparisonResult,
      done: Boolean(comparisonResult),
      badge: comparisonResult ? `${comparisonResult.signals.length} paired` : undefined,
    },
  ];

  const stepperActions = (observed.dataset || reference.dataset || observed.rawText || reference.rawText) ? (
    <button type="button" onClick={startNewComparison}>
      <RotateCcw size={13} /> New comparison
    </button>
  ) : null;

  const status = useMemo(() => {
    if (step === 'data') {
      if (observed.error) return <span className="research-error-inline">{observed.error}</span>;
      if (observed.dataset) return `${observed.dataset.axis.values.length.toLocaleString()} rows retained in observed dataset`;
      return observed.rawText ? 'CSV loaded · Configure mapping and import' : 'Choose an observed CSV file to begin';
    }
    if (step === 'reference') {
      const err = reference.error || simulationError;
      if (err) return <span className="research-error-inline">{err}</span>;
      if (reference.dataset) return `${reference.dataset.axis.values.length.toLocaleString()} rows retained in reference dataset`;
      return reference.rawText ? 'Reference CSV loaded · Configure mapping and import' : 'Select simulation or choose a reference CSV';
    }
    if (step === 'match') {
      if (comparisonError) return <span className="research-error-inline">{comparisonError}</span>;
      return canCompare ? `${pairs.length} signal pair(s) configured · Ready to compare` : 'Both datasets must be imported to compare';
    }
    if (step === 'results') {
      if (comparisonError) return <span className="research-error-inline">{comparisonError}</span>;
      return comparisonResult ? `Comparison complete · ${comparisonResult.signals.length} signal pair(s) analyzed` : '';
    }
    return '';
  }, [
    step,
    observed.error,
    observed.dataset,
    observed.rawText,
    reference.error,
    simulationError,
    reference.dataset,
    reference.rawText,
    comparisonError,
    canCompare,
    pairs.length,
    comparisonResult,
  ]);

  const footerActions = (
    <>
      {step === 'data' && (
        <>
          {observed.dataset && (
            <button type="button" onClick={() => setStep('reference')}>
              Next: Reference
            </button>
          )}
          <button
            type="button"
            className="primary"
            disabled={!observed.rawText.trim()}
            onClick={() => importCsv('observed')}
          >
            Use observed data
          </button>
        </>
      )}
      {step === 'reference' && (
        <>
          <button type="button" onClick={() => setStep('data')}>
            Back
          </button>
          {reference.dataset && (
            <button type="button" onClick={() => setStep('match')}>
              Next: Match
            </button>
          )}
          {reference.rawText.trim() ? (
            <button
              type="button"
              className="primary"
              onClick={() => importCsv('reference')}
            >
              Use reference CSV
            </button>
          ) : hasSimDatasets ? (
            <button
              type="button"
              className="primary"
              disabled={simulationSignals.length === 0}
              onClick={useSimulationAsReference}
            >
              Use simulation as reference
            </button>
          ) : null}
        </>
      )}
      {step === 'match' && (
        <>
          <button type="button" onClick={() => setStep('reference')}>
            Back
          </button>
          <button
            type="button"
            className="primary"
            disabled={!canCompare}
            onClick={runComparison}
          >
            Compare datasets
          </button>
        </>
      )}
      {step === 'results' && (
        <>
          <button type="button" onClick={() => setStep('match')}>
            Back to match
          </button>
          <button type="button" onClick={startNewComparison}>
            New comparison
          </button>
          <button
            type="button"
            className="primary"
            disabled={!comparisonResult}
            onClick={downloadEvidence}
          >
            Download evidence JSON
          </button>
        </>
      )}
    </>
  );

  return (
    <WorkflowDialog<Step>
      open={open}
      onClose={onClose}
      ariaLabel="Research data"
      closeLabel="Close research data"
      className="research-dialog"
      icon={<BarChart3 size={18} />}
      title="Research data"
      subtitle="Compare observed CSV measurements against a simulation or reference CSV. Files stay in this browser tab."
      headerExtras={
        <p className="dialog-help-link">
          <a href={`${import.meta.env.BASE_URL}docs/guides/research-data/`} target="_blank" rel="noreferrer">
            CSV inputs, mapping and comparison walkthrough
          </a>{' '}
          · Sample CSVs:{' '}
          <a href={`${import.meta.env.BASE_URL}examples/research/scope-style.csv`} download>
            observed
          </a>{' '}
          /{' '}
          <a href={`${import.meta.env.BASE_URL}examples/research/rc-reference.csv`} download>
            reference
          </a>
        </p>
      }
      stepperLabel="Research data steps"
      steps={steps}
      activeStep={step}
      onStepSelect={setStep}
      stepperActions={stepperActions}
      status={status}
      footerActions={footerActions}
    >
      {step === 'data' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Observed dataset</h3>
              <p>Import bench oscilloscope, analyzer, or published CSV measurements.</p>
            </div>
          </div>

          <CsvDatasetImporter
            fileInputAriaLabel="Observed CSV file"
            fileButtonText="Choose observed CSV"
            fileTitle={observed.file?.name ?? 'No CSV selected'}
            slot={observed}
            headers={observedHeaders}
            notes={observedNotes}
            onFileSelect={(file) => void read('data', file)}
            onUpdateDraft={(change) => updateDraft('data', change)}
            onUpdateDialect={(change) => updateDialect('data', change)}
            onNotesChange={setObservedNotes}
          />
        </div>
      )}

      {step === 'reference' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Reference dataset</h3>
              <p>Compare against the current Kessetsu simulation or import a second reference CSV.</p>
            </div>
          </div>

          <section className="research-simulation-source" aria-label="Current simulation reference">
            <div>
              <h3>Use the current simulation</h3>
              <p>Project one completed transient, AC or DC dataset from the workspace as your reference.</p>
            </div>

            {selectedAnalysis ? (
              <>
                <div className="research-simulation-controls">
                  <label>
                    Analysis
                    <select
                      aria-label="Analysis"
                      value={selectedAnalysis.index}
                      onChange={(event) => {
                        setSimulationAnalysis(Number(event.target.value));
                        setSimulationSignals([]);
                        setSimulationError('');
                      }}
                    >
                      {eligibleAnalyses.map((item) => (
                        <option key={item.index} value={item.index}>
                          {analysisLabel(item.data, item.index)}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>

                <details open className="research-simulation-details">
                  <summary>Edit simulation signal mapping ({simulationSignals.length})</summary>
                  <div className="research-section-heading">
                    <h3>Simulation signals</h3>
                    <button
                      type="button"
                      disabled={simulationSignals.length >= Math.min(16, availableSimulationSignals.length)}
                      onClick={() => {
                        const used = new Set(simulationSignals.map((mapping) => mapping.vector));
                        const vector = availableSimulationSignals.find((candidate) => !used.has(candidate));
                        if (vector) {
                          setSimulationSignals((current) => [
                            ...current,
                            {
                              vector,
                              name: logicalName(vector.replace(/[()]/g, '_'), `signal_${current.length + 1}`),
                            },
                          ]);
                        }
                      }}
                    >
                      <Plus size={14} /> Add signal
                    </button>
                  </div>
                  {simulationSignals.map((mapping, index) => (
                    <div className="research-pair" key={index}>
                      <label>
                        Simulation vector
                        <select
                          aria-label="Simulation vector"
                          value={mapping.vector}
                          onChange={(event) =>
                            setSimulationSignals((current) =>
                              current.map((item, i) =>
                                i === index ? { ...item, vector: event.target.value } : item
                              )
                            )
                          }
                        >
                          {availableSimulationSignals.map((sig) => (
                            <option key={sig} value={sig}>
                              {sig}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        Logical name
                        <input
                          aria-label="Logical name"
                          value={mapping.name}
                          onChange={(event) =>
                            setSimulationSignals((current) =>
                              current.map((item, i) =>
                                i === index ? { ...item, name: event.target.value } : item
                              )
                            )
                          }
                        />
                      </label>
                      {simulationSignals.length > 1 && (
                        <button
                          type="button"
                          className="research-icon-button"
                          aria-label={`Remove simulation signal ${index + 1}`}
                          onClick={() =>
                            setSimulationSignals((current) => current.filter((_, i) => i !== index))
                          }
                        >
                          <Trash2 size={14} />
                        </button>
                      )}
                    </div>
                  ))}
                </details>

                {Boolean(reference.rawText.trim()) && (
                  <button
                    type="button"
                    className="research-primary"
                    disabled={simulationSignals.length === 0}
                    onClick={useSimulationAsReference}
                  >
                    Use simulation as reference
                  </button>
                )}
              </>
            ) : (
              <p className="research-note">
                Run a transient, AC or DC simulation in the Simulation panel first, or import a reference CSV below.
              </p>
            )}

            {simulationError && (
              <p className="research-error" role="alert">
                {simulationError}
              </p>
            )}

            <div className="research-or">
              <span>or import a reference CSV</span>
            </div>
          </section>

          <CsvDatasetImporter
            fileInputAriaLabel="Reference CSV file"
            fileButtonText="Choose reference CSV"
            fileTitle={
              reference.file?.name ??
              (reference.dataset?.spec.metadata.origin === 'simulation'
                ? 'Current simulation projection'
                : 'No CSV selected')
            }
            slot={reference}
            headers={referenceHeaders}
            notes={referenceNotes}
            onFileSelect={(file) => void read('reference', file)}
            onUpdateDraft={(change) => updateDraft('reference', change)}
            onUpdateDialect={(change) => updateDialect('reference', change)}
            onNotesChange={setReferenceNotes}
          />
        </div>
      )}

      {step === 'match' && (
        <div className="workflow-step-pane">
          <div className="workflow-step-intro">
            <div>
              <h3>Match & compare</h3>
              <p>Verify dataset provenance, pair observed signals with reference signals, and choose axis interpolation.</p>
            </div>
          </div>

          <div className="research-comparison-summary">
            <article>
              <div>
                <span>Observed</span>
                <strong>{observed.dataset?.spec.name}</strong>
                <small>
                  {observed.dataset?.spec.metadata.origin} · {observed.dataset?.axis.unit} ·{' '}
                  {observed.dataset?.axis.values.length.toLocaleString()} points
                </small>
              </div>
              <button type="button" onClick={() => setStep('data')}>
                Change
              </button>
            </article>
            <article>
              <div>
                <span>Reference</span>
                <strong>{reference.dataset?.spec.name}</strong>
                <small>
                  {reference.dataset?.spec.metadata.origin} · {reference.dataset?.axis.unit} ·{' '}
                  {reference.dataset?.axis.values.length.toLocaleString()} points
                </small>
              </div>
              <button type="button" onClick={() => setStep('reference')}>
                Change
              </button>
            </article>
          </div>

          <div className="research-section-heading">
            <h3>Signal pairs</h3>
            <button
              type="button"
              disabled={pairs.length >= 16}
              onClick={() =>
                setPairs((current) => [
                  ...current,
                  {
                    data_signal: observed.dataset?.signals[0]?.name ?? '',
                    reference_signal: reference.dataset?.signals[0]?.name ?? '',
                  },
                ])
              }
            >
              <Plus size={14} /> Add signal pair
            </button>
          </div>

          {pairs.map((pair, index) => (
            <div className="research-pair" key={index}>
              <label>
                Observed signal {pairs.length > 1 ? index + 1 : ''}
                <select
                  aria-label={`Observed signal ${index + 1}`}
                  value={pair.data_signal}
                  onChange={(event) => {
                    setPairs((current) =>
                      current.map((item, i) =>
                        i === index ? { ...item, data_signal: event.target.value } : item
                      )
                    );
                    setComparisonResult(null);
                  }}
                >
                  {observed.dataset?.signals.map((signal) => (
                    <option key={signal.name} value={signal.name}>
                      {signal.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Reference signal {pairs.length > 1 ? index + 1 : ''}
                <select
                  aria-label={`Reference signal ${index + 1}`}
                  value={pair.reference_signal}
                  onChange={(event) => {
                    setPairs((current) =>
                      current.map((item, i) =>
                        i === index ? { ...item, reference_signal: event.target.value } : item
                      )
                    );
                    setComparisonResult(null);
                  }}
                >
                  {reference.dataset?.signals.map((signal) => (
                    <option key={signal.name} value={signal.name}>
                      {signal.name}
                    </option>
                  ))}
                </select>
              </label>
              {pairs.length > 1 && (
                <button
                  type="button"
                  className="research-icon-button"
                  aria-label={`Remove signal pair ${index + 1}`}
                  onClick={() => {
                    setPairs((current) => current.filter((_, i) => i !== index));
                    setComparisonResult(null);
                  }}
                >
                  <Trash2 size={14} />
                </button>
              )}
            </div>
          ))}

          <div className="research-compare-primary">
            <label>
              Interpolation
              <select
                aria-label="Interpolation"
                value={interpolation}
                onChange={(event) => {
                  setInterpolation(event.target.value as typeof interpolation);
                  setComparisonResult(null);
                }}
              >
                <option value="linear">Linear axis</option>
                <option value="log_axis">Log-frequency axis</option>
              </select>
            </label>
          </div>

          <details>
            <summary>Advanced comparison settings</summary>
            <div className="research-form-grid">
              <label className="research-wide">
                Comparison name
                <input
                  value={comparisonName}
                  onChange={(event) => {
                    setComparisonName(event.target.value);
                    setComparisonResult(null);
                  }}
                />
              </label>
              <label>
                Coverage
                <select
                  value={coverage}
                  onChange={(event) => {
                    setCoverage(event.target.value as typeof coverage);
                    setComparisonResult(null);
                  }}
                >
                  <option value="require_full">Require full observed coverage</option>
                  <option value="overlap_only">Overlap window only</option>
                </select>
              </label>
              <label>
                Reference axis shift
                <input
                  value={shift}
                  onChange={(event) => {
                    setShift(event.target.value);
                    setComparisonResult(null);
                  }}
                />
              </label>
              <label>
                Maximum reference gap
                <input
                  placeholder="No limit"
                  value={maxGap}
                  onChange={(event) => {
                    setMaxGap(event.target.value);
                    setComparisonResult(null);
                  }}
                />
              </label>
              <label>
                Window start
                <input
                  placeholder="All data"
                  value={windowStart}
                  onChange={(event) => {
                    setWindowStart(event.target.value);
                    setComparisonResult(null);
                  }}
                />
              </label>
              <label>
                Window stop
                <input
                  placeholder="All data"
                  value={windowStop}
                  onChange={(event) => {
                    setWindowStop(event.target.value);
                    setComparisonResult(null);
                  }}
                />
              </label>
            </div>
            <p className="research-note">
              Residual means reference minus observed. Kessetsu never extrapolates or auto-aligns axes.
              Log interpolation is available only for positive frequency axes.
            </p>
          </details>

          {comparisonError && (
            <p className="research-error" role="alert">
              {comparisonError}
            </p>
          )}
        </div>
      )}

      {step === 'results' && (
        <div className="workflow-step-pane is-wide">
          <div className="workflow-step-intro">
            <div>
              <h3>Comparison results</h3>
              <p>Overlay plot, paired metrics, and reproducible evidence bundle.</p>
            </div>
          </div>

          {comparisonResult ? (
            <div className="research-results">
              <div className="research-result-header">
                <div>
                  <strong>Comparison complete: {comparisonTitle}</strong>
                  <small>
                    Observed sha256:{observed.dataset?.raw_sha256 ? `${observed.dataset.raw_sha256.slice(0, 16)}…` : '—'} ·
                    Reference sha256:{reference.dataset?.raw_sha256 ? `${reference.dataset.raw_sha256.slice(0, 16)}…` : '—'}
                  </small>
                  <small>
                    {totalRetained.toLocaleString()} source rows retained across both datasets ·{' '}
                    {comparisonResult.identity}
                  </small>
                </div>
              </div>

              <div className="research-metrics">
                <table>
                  <thead>
                    <tr>
                      <th>Signal</th>
                      <th>Overlap</th>
                      <th>Samples</th>
                      <th>RMSE</th>
                      <th>MAE</th>
                      <th>Max abs error</th>
                      <th>Bias</th>
                      <th>Pearson r</th>
                      <th>Peak delta</th>
                      <th>-3 dB ratio</th>
                    </tr>
                  </thead>
                  <tbody>
                    {comparisonResult.signals.map((signal, index) => (
                      <tr
                        key={signal.mapping.data_signal}
                        className={selectedOverlayIndex === index ? 'selected' : ''}
                        onClick={() => setSelectedOverlayIndex(index)}
                      >
                        <td>
                          {signal.mapping.data_signal} vs {signal.mapping.reference_signal}
                          <small>{signal.unit}</small>
                        </td>
                        <td>
                          {signal.metrics.matched}/{signal.metrics.total}
                          <small>
                            {signal.metrics.unmatched} unmatched · {signal.metrics.excluded_by_window} excluded
                          </small>
                        </td>
                        <td>{signal.metrics.matched}</td>
                        <td>{engineering(signal.metrics.rmse)}</td>
                        <td>{engineering(signal.metrics.mae)}</td>
                        <td>{engineering(signal.metrics.max_absolute)}</td>
                        <td>{engineering(signal.metrics.bias)}</td>
                        <td>
                          {(signal.metrics as unknown as { pearson_r?: number }).pearson_r !== undefined
                            ? engineering((signal.metrics as unknown as { pearson_r: number }).pearson_r)
                            : '—'}
                        </td>
                        <td>
                          {(signal.metrics as unknown as { peak_delta?: number }).peak_delta !== undefined
                            ? engineering((signal.metrics as unknown as { peak_delta: number }).peak_delta)
                            : '—'}
                        </td>
                        <td>
                          {(signal.metrics as unknown as { three_db_ratio?: number }).three_db_ratio !== undefined
                            ? engineering((signal.metrics as unknown as { three_db_ratio: number }).three_db_ratio)
                            : '—'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {selectedSignal && (
                <ComparisonPlot comparison={comparisonResult} signalIndex={selectedOverlayIndex} />
              )}
            </div>
          ) : (
            <p className="research-note">
              No comparison results yet. Go to Match & compare to run a comparison.
            </p>
          )}
        </div>
      )}
    </WorkflowDialog>
  );
}
