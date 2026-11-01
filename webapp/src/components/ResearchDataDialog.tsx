import { BarChart3, CheckCircle2, Download, Plus, RotateCcw, Trash2, Upload } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import {
  compare_research_data,
  import_research_csv,
  import_simulation_research_data,
  preview_research_csv,
} from 'kessetsu-core';
import { downloadTextFile, sanitizeFileStem } from '../document';
import {
  column,
  emptySlot,
  engineering,
  logicalName,
  mappingSpec,
  parseOptional,
  researchUnits,
  sourceUnit,
  type ColumnMapping,
  type CsvDialect,
  type CsvPreview,
  type DataComparison,
  type DatasetSlot,
  type ResearchData,
  type ResearchUnit,
  type SignalPair,
} from '../research';
import type { BrowserEvaluation, Dataset } from '../simulation/types';
import { WorkflowDialog, type WorkflowStep } from './WorkflowDialog';
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

function PrimaryMappingRow({
  mapping,
  headers,
  onChange,
  onRemove,
}: {
  mapping: ColumnMapping;
  headers: string[];
  onChange: (mapping: ColumnMapping) => void;
  onRemove?: () => void;
}) {
  const changeUnit = (unit: ResearchUnit) => onChange({ ...mapping, unit, source_unit: sourceUnit(unit) });
  return (
    <div className="research-mapping-row">
      <label>
        CSV column
        <select
          aria-label="CSV column"
          value={mapping.column}
          onChange={(event) => onChange({ ...mapping, column: Number(event.target.value) })}
        >
          {headers.map((header, index) => (
            <option key={index} value={index}>
              {index + 1}: {header || `Column ${index + 1}`}
            </option>
          ))}
        </select>
      </label>
      <label>
        Logical name
        <input
          aria-label="Logical name"
          value={mapping.name}
          onChange={(event) => onChange({ ...mapping, name: event.target.value })}
        />
      </label>
      <label>
        Quantity
        <select
          aria-label="Quantity"
          value={mapping.unit}
          onChange={(event) => changeUnit(event.target.value as ResearchUnit)}
        >
          {researchUnits.map((unit) => (
            <option key={unit} value={unit}>
              {unit}
            </option>
          ))}
        </select>
      </label>
      {onRemove && (
        <button
          type="button"
          className="research-icon-button"
          aria-label={`Remove signal ${mapping.name}`}
          onClick={onRemove}
        >
          <Trash2 size={14} />
        </button>
      )}
    </div>
  );
}

function CalibrationRow({
  label,
  mapping,
  axis,
  onChange,
}: {
  label: string;
  mapping: ColumnMapping;
  axis?: boolean;
  onChange: (mapping: ColumnMapping) => void;
}) {
  return (
    <div className="research-calibration-row">
      <span className="research-calibration-target">
        {label} (<code>{mapping.name}</code>)
      </span>
      <label>
        CSV unit
        <input
          value={mapping.source_unit}
          aria-label={`${axis ? 'Axis' : 'Signal'} source unit`}
          onChange={(event) => onChange({ ...mapping, source_unit: event.target.value })}
        />
      </label>
      <label>
        Gain
        <input
          type="number"
          step="any"
          value={mapping.gain}
          onChange={(event) => onChange({ ...mapping, gain: Number(event.target.value) })}
        />
      </label>
      <label>
        Offset
        <input
          type="number"
          step="any"
          value={mapping.offset}
          onChange={(event) => onChange({ ...mapping, offset: Number(event.target.value) })}
        />
      </label>
    </div>
  );
}

function polyline(
  points: Array<{ axis: number; value: number }>,
  x: (value: number) => number,
  y: (value: number) => number
) {
  if (points.length > 2400) {
    const stride = Math.ceil(points.length / 2400);
    points = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
  }
  return points.map((point) => `${x(point.axis).toFixed(2)},${y(point.value).toFixed(2)}`).join(' ');
}

function ComparisonPlot({ comparison, signalIndex }: { comparison: DataComparison; signalIndex: number }) {
  const signal = comparison.signals[signalIndex];
  if (!signal) return null;
  const matched = signal.points.filter((point) => point.predicted !== null);
  if (matched.length === 0) return <p className="research-note">No matched points are available to plot.</p>;
  const axes = matched.map((point) => point.axis);
  const values = matched.flatMap((point) => [point.observed, point.predicted!]);
  let minX = Math.min(...axes);
  let maxX = Math.max(...axes);
  let minY = Math.min(...values);
  let maxY = Math.max(...values);
  if (minX === maxX) {
    minX -= 0.5;
    maxX += 0.5;
  }
  if (minY === maxY) {
    const pad = Math.abs(minY) * 0.05 || 0.5;
    minY -= pad;
    maxY += pad;
  }
  const x = (value: number) => 62 + ((value - minX) / (maxX - minX)) * 786;
  const y = (value: number) => 220 - ((value - minY) / (maxY - minY)) * 184;
  const observed = matched.map((point) => ({ axis: point.axis, value: point.observed }));
  const predicted = matched.map((point) => ({ axis: point.axis, value: point.predicted! }));
  return (
    <figure className="research-plot">
      <svg viewBox="0 0 900 260" role="img" aria-label={`${signal.mapping.data_signal} observed and reference overlay`}>
        <rect x="62" y="36" width="786" height="184" />
        {[0, 0.25, 0.5, 0.75, 1].map((part) => (
          <g key={part}>
            <line x1="62" x2="848" y1={36 + part * 184} y2={36 + part * 184} />
            <text x="54" y={40 + part * 184} textAnchor="end">
              {engineering(maxY - part * (maxY - minY))}
            </text>
          </g>
        ))}
        <polyline className="research-observed" points={polyline(observed, x, y)} />
        <polyline className="research-reference" points={polyline(predicted, x, y)} />
        <text x="62" y="242">
          {engineering(minX)}
        </text>
        <text x="848" y="242" textAnchor="end">
          {engineering(maxX)} {comparison.axis_unit}
        </text>
      </svg>
      <figcaption>
        <span className="observed-key" /> Observed: {signal.mapping.data_signal}
        <span className="reference-key" /> Reference: {signal.mapping.reference_signal} · {signal.unit}
      </figcaption>
    </figure>
  );
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

          <div className="research-file-row">
            <label className="research-file-button">
              <Upload size={15} /> Choose observed CSV
              <input
                className="sr-only"
                type="file"
                accept=".csv,text/csv,text/plain"
                aria-label="Observed CSV file"
                onChange={(event) => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = '';
                  if (file) void read('data', file);
                }}
              />
            </label>
            <span title={observed.file?.name ?? 'No CSV selected'}>
              {observed.file?.name ?? 'No CSV selected'}
            </span>
            {observed.dataset && (
              <span className="research-ready">
                <CheckCircle2 size={14} /> Ready ({observed.dataset.spec.name})
              </span>
            )}
          </div>

          {observed.rawText && (
            <>
              {observed.preview && (
                <div className="research-preview">
                  <div>
                    <strong>Preview</strong> · {observed.preview.data_records.toLocaleString()} data records ·{' '}
                    {observedHeaders.length} columns · {observed.preview.blank_records} blank
                  </div>
                  <div className="research-preview-table">
                    <table>
                      <thead>
                        <tr>
                          <th>Record</th>
                          {observedHeaders.map((header, index) => (
                            <th key={index}>{header || `Column ${index + 1}`}</th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {observed.preview.sample.map((row) => (
                          <tr key={`${row.record}-${row.line}`}>
                            <td>{row.record}</td>
                            {row.fields.map((field, index) => (
                              <td key={index} title={field}>
                                {field}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}

              {observed.preview && observedHeaders.length > 0 && (
                <div className="research-import-form">
                  <div className="research-primary-meta">
                    <label>
                      Dataset name
                      <input
                        value={observed.draft.name}
                        onChange={(event) => updateDraft('data', { name: event.target.value })}
                      />
                    </label>
                    <label>
                      Origin
                      <select
                        value={observed.draft.origin}
                        onChange={(event) =>
                          updateDraft('data', {
                            origin: event.target.value as DatasetSlot['draft']['origin'],
                          })
                        }
                      >
                        <option value="measured">Measured</option>
                        <option value="published_simulation">Published simulation</option>
                        <option value="simulation">Simulation export</option>
                        <option value="synthetic">Synthetic</option>
                        <option value="unspecified">Unspecified</option>
                      </select>
                    </label>
                  </div>

                  <h3>Comparison axis</h3>
                  <PrimaryMappingRow
                    mapping={observed.draft.axis}
                    headers={observedHeaders}
                    onChange={(axis) => updateDraft('data', { axis })}
                  />

                  <div className="research-section-heading">
                    <h3>Signals</h3>
                    <button
                      type="button"
                      disabled={observed.draft.signals.length >= 32}
                      onClick={() => {
                        const next = observed.draft.signals.length + 1;
                        const used = new Set([
                          observed.draft.axis.column,
                          ...observed.draft.signals.map((s) => s.column),
                        ]);
                        const available = observedHeaders.findIndex((_, index) => !used.has(index));
                        updateDraft('data', {
                          signals: [
                            ...observed.draft.signals,
                            column(available >= 0 ? available : 0, `signal_${next}`, 'Volt'),
                          ],
                        });
                      }}
                    >
                      <Plus size={14} /> Add signal
                    </button>
                  </div>
                  {observed.draft.signals.map((signal, index) => (
                    <PrimaryMappingRow
                      key={index}
                      mapping={signal}
                      headers={observedHeaders}
                      onChange={(next) =>
                        updateDraft('data', {
                          signals: observed.draft.signals.map((item, i) => (i === index ? next : item)),
                        })
                      }
                      onRemove={
                        observed.draft.signals.length > 1
                          ? () =>
                              updateDraft('data', {
                                signals: observed.draft.signals.filter((_, i) => i !== index),
                              })
                          : undefined
                      }
                    />
                  ))}

                  <details>
                    <summary>Advanced import settings</summary>
                    <div className="research-dialect">
                      <label>
                        Delimiter
                        <select
                          value={observed.dialect.delimiter}
                          onChange={(event) =>
                            updateDialect('data', {
                              delimiter: event.target.value as CsvDialect['delimiter'],
                            })
                          }
                        >
                          <option value="comma">Comma</option>
                          <option value="semicolon">Semicolon</option>
                          <option value="tab">Tab</option>
                        </select>
                      </label>
                      <label>
                        Decimal
                        <select
                          value={observed.dialect.decimal}
                          onChange={(event) =>
                            updateDialect('data', {
                              decimal: event.target.value as CsvDialect['decimal'],
                            })
                          }
                        >
                          <option value="dot">Dot</option>
                          <option value="comma">Comma</option>
                        </select>
                      </label>
                      <label>
                        Header row
                        <select
                          value={observed.dialect.header ? 'yes' : 'no'}
                          onChange={(event) =>
                            updateDialect('data', { header: event.target.value === 'yes' })
                          }
                        >
                          <option value="yes">First record</option>
                          <option value="no">No header</option>
                        </select>
                      </label>
                      <label>
                        Skip records
                        <input
                          type="number"
                          min="0"
                          max="128"
                          value={observed.dialect.preamble_records}
                          onChange={(event) =>
                            updateDialect('data', { preamble_records: Number(event.target.value) })
                          }
                        />
                      </label>
                      <label>
                        Axis order
                        <select
                          value={observed.draft.axisOrder}
                          onChange={(event) =>
                            updateDraft('data', {
                              axisOrder: event.target.value as DatasetSlot['draft']['axisOrder'],
                            })
                          }
                        >
                          <option value="increasing">Increasing</option>
                          <option value="decreasing">Decreasing</option>
                        </select>
                      </label>
                      <label>
                        Missing values
                        <select
                          value={observed.draft.missing}
                          onChange={(event) =>
                            updateDraft('data', {
                              missing: event.target.value as DatasetSlot['draft']['missing'],
                            })
                          }
                        >
                          <option value="error">Stop with error</option>
                          <option value="skip_row">Skip whole row and record it</option>
                        </select>
                      </label>
                      <label className="research-wide">
                        Missing tokens
                        <input
                          value={observed.draft.missingTokens}
                          onChange={(event) => updateDraft('data', { missingTokens: event.target.value })}
                        />
                      </label>
                    </div>

                    <h4>Unit scaling and offset</h4>
                    <CalibrationRow
                      label="Axis"
                      mapping={observed.draft.axis}
                      axis
                      onChange={(axis) => updateDraft('data', { axis })}
                    />
                    {observed.draft.signals.map((signal, index) => (
                      <CalibrationRow
                        key={index}
                        label={`Signal ${index + 1}`}
                        mapping={signal}
                        onChange={(next) =>
                          updateDraft('data', {
                            signals: observed.draft.signals.map((item, i) => (i === index ? next : item)),
                          })
                        }
                      />
                    ))}

                    <h4>Provenance & citation</h4>
                    <div className="research-form-grid">
                      <label>
                        Device / bench setup
                        <input
                          value={observed.draft.device}
                          onChange={(event) => updateDraft('data', { device: event.target.value })}
                        />
                      </label>
                      <label>
                        Sample / run ID
                        <input
                          value={observed.draft.sample}
                          onChange={(event) => updateDraft('data', { sample: event.target.value })}
                        />
                      </label>
                      <label className="research-wide">
                        Citation / source URL
                        <textarea
                          rows={2}
                          value={observed.draft.citation}
                          onChange={(event) => updateDraft('data', { citation: event.target.value })}
                        />
                      </label>
                      <label className="research-wide">
                        Import notes
                        <textarea
                          rows={2}
                          value={observedNotes}
                          onChange={(event) => setObservedNotes(event.target.value)}
                        />
                      </label>
                    </div>
                  </details>
                </div>
              )}
            </>
          )}

          {observed.error && (
            <p className="research-error" role="alert">
              {observed.error}
            </p>
          )}

          {observed.dataset && (
            <div className="research-import-summary">
              <div>
                <strong>{observed.dataset.axis.values.length.toLocaleString()}</strong> retained rows ·{' '}
                {observed.dataset.signals.length} signals · {observed.dataset.skipped.length} skipped
              </div>
              <code>
                {observed.dataset.raw_sha256
                  ? `sha256:${observed.dataset.raw_sha256.slice(0, 16)}…`
                  : observed.dataset.identity}
              </code>
              <button
                type="button"
                onClick={() =>
                  downloadTextFile(
                    JSON.stringify(observed.dataset, null, 2),
                    `${sanitizeFileStem(observed.dataset!.spec.name)}.kessdata.json`
                  )
                }
              >
                <Download size={14} /> Download dataset JSON
              </button>
            </div>
          )}
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

          <div className="research-file-row">
            <label className="research-file-button">
              <Upload size={15} /> Choose reference CSV
              <input
                className="sr-only"
                type="file"
                accept=".csv,text/csv,text/plain"
                aria-label="Reference CSV file"
                onChange={(event) => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = '';
                  if (file) void read('reference', file);
                }}
              />
            </label>
            <span title={reference.file?.name ?? (reference.dataset?.spec.metadata.origin === 'simulation' ? 'Current simulation projection' : 'No CSV selected')}>
              {reference.file?.name ?? (reference.dataset?.spec.metadata.origin === 'simulation' ? 'Current simulation projection' : 'No CSV selected')}
            </span>
            {reference.dataset && (
              <span className="research-ready">
                <CheckCircle2 size={14} /> Ready ({reference.dataset.spec.name})
              </span>
            )}
          </div>

          {reference.rawText && (
            <>
              {reference.preview && (
                <div className="research-preview">
                  <div>
                    <strong>Preview</strong> · {reference.preview.data_records.toLocaleString()} data records ·{' '}
                    {referenceHeaders.length} columns · {reference.preview.blank_records} blank
                  </div>
                  <div className="research-preview-table">
                    <table>
                      <thead>
                        <tr>
                          <th>Record</th>
                          {referenceHeaders.map((header, index) => (
                            <th key={index}>{header || `Column ${index + 1}`}</th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {reference.preview.sample.map((row) => (
                          <tr key={`${row.record}-${row.line}`}>
                            <td>{row.record}</td>
                            {row.fields.map((field, index) => (
                              <td key={index} title={field}>
                                {field}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}

              {reference.preview && referenceHeaders.length > 0 && (
                <div className="research-import-form">
                  <div className="research-primary-meta">
                    <label>
                      Dataset name
                      <input
                        value={reference.draft.name}
                        onChange={(event) => updateDraft('reference', { name: event.target.value })}
                      />
                    </label>
                    <label>
                      Origin
                      <select
                        value={reference.draft.origin}
                        onChange={(event) =>
                          updateDraft('reference', {
                            origin: event.target.value as DatasetSlot['draft']['origin'],
                          })
                        }
                      >
                        <option value="measured">Measured</option>
                        <option value="published_simulation">Published simulation</option>
                        <option value="simulation">Simulation export</option>
                        <option value="synthetic">Synthetic</option>
                        <option value="unspecified">Unspecified</option>
                      </select>
                    </label>
                  </div>

                  <h3>Comparison axis</h3>
                  <PrimaryMappingRow
                    mapping={reference.draft.axis}
                    headers={referenceHeaders}
                    onChange={(axis) => updateDraft('reference', { axis })}
                  />

                  <div className="research-section-heading">
                    <h3>Signals</h3>
                    <button
                      type="button"
                      disabled={reference.draft.signals.length >= 32}
                      onClick={() => {
                        const next = reference.draft.signals.length + 1;
                        const used = new Set([
                          reference.draft.axis.column,
                          ...reference.draft.signals.map((s) => s.column),
                        ]);
                        const available = referenceHeaders.findIndex((_, index) => !used.has(index));
                        updateDraft('reference', {
                          signals: [
                            ...reference.draft.signals,
                            column(available >= 0 ? available : 0, `signal_${next}`, 'Volt'),
                          ],
                        });
                      }}
                    >
                      <Plus size={14} /> Add signal
                    </button>
                  </div>
                  {reference.draft.signals.map((signal, index) => (
                    <PrimaryMappingRow
                      key={index}
                      mapping={signal}
                      headers={referenceHeaders}
                      onChange={(next) =>
                        updateDraft('reference', {
                          signals: reference.draft.signals.map((item, i) => (i === index ? next : item)),
                        })
                      }
                      onRemove={
                        reference.draft.signals.length > 1
                          ? () =>
                              updateDraft('reference', {
                                signals: reference.draft.signals.filter((_, i) => i !== index),
                              })
                          : undefined
                      }
                    />
                  ))}

                  <details>
                    <summary>Advanced import settings</summary>
                    <div className="research-dialect">
                      <label>
                        Delimiter
                        <select
                          value={reference.dialect.delimiter}
                          onChange={(event) =>
                            updateDialect('reference', {
                              delimiter: event.target.value as CsvDialect['delimiter'],
                            })
                          }
                        >
                          <option value="comma">Comma</option>
                          <option value="semicolon">Semicolon</option>
                          <option value="tab">Tab</option>
                        </select>
                      </label>
                      <label>
                        Decimal
                        <select
                          value={reference.dialect.decimal}
                          onChange={(event) =>
                            updateDialect('reference', {
                              decimal: event.target.value as CsvDialect['decimal'],
                            })
                          }
                        >
                          <option value="dot">Dot</option>
                          <option value="comma">Comma</option>
                        </select>
                      </label>
                      <label>
                        Header row
                        <select
                          value={reference.dialect.header ? 'yes' : 'no'}
                          onChange={(event) =>
                            updateDialect('reference', { header: event.target.value === 'yes' })
                          }
                        >
                          <option value="yes">First record</option>
                          <option value="no">No header</option>
                        </select>
                      </label>
                      <label>
                        Skip records
                        <input
                          type="number"
                          min="0"
                          max="128"
                          value={reference.dialect.preamble_records}
                          onChange={(event) =>
                            updateDialect('reference', { preamble_records: Number(event.target.value) })
                          }
                        />
                      </label>
                      <label>
                        Axis order
                        <select
                          value={reference.draft.axisOrder}
                          onChange={(event) =>
                            updateDraft('reference', {
                              axisOrder: event.target.value as DatasetSlot['draft']['axisOrder'],
                            })
                          }
                        >
                          <option value="increasing">Increasing</option>
                          <option value="decreasing">Decreasing</option>
                        </select>
                      </label>
                      <label>
                        Missing values
                        <select
                          value={reference.draft.missing}
                          onChange={(event) =>
                            updateDraft('reference', {
                              missing: event.target.value as DatasetSlot['draft']['missing'],
                            })
                          }
                        >
                          <option value="error">Stop with error</option>
                          <option value="skip_row">Skip whole row and record it</option>
                        </select>
                      </label>
                      <label className="research-wide">
                        Missing tokens
                        <input
                          value={reference.draft.missingTokens}
                          onChange={(event) =>
                            updateDraft('reference', { missingTokens: event.target.value })
                          }
                        />
                      </label>
                    </div>

                    <h4>Unit scaling and offset</h4>
                    <CalibrationRow
                      label="Axis"
                      mapping={reference.draft.axis}
                      axis
                      onChange={(axis) => updateDraft('reference', { axis })}
                    />
                    {reference.draft.signals.map((signal, index) => (
                      <CalibrationRow
                        key={index}
                        label={`Signal ${index + 1}`}
                        mapping={signal}
                        onChange={(next) =>
                          updateDraft('reference', {
                            signals: reference.draft.signals.map((item, i) => (i === index ? next : item)),
                          })
                        }
                      />
                    ))}

                    <h4>Provenance & citation</h4>
                    <div className="research-form-grid">
                      <label>
                        Device / bench setup
                        <input
                          value={reference.draft.device}
                          onChange={(event) => updateDraft('reference', { device: event.target.value })}
                        />
                      </label>
                      <label>
                        Sample / run ID
                        <input
                          value={reference.draft.sample}
                          onChange={(event) => updateDraft('reference', { sample: event.target.value })}
                        />
                      </label>
                      <label className="research-wide">
                        Citation / source URL
                        <textarea
                          rows={2}
                          value={reference.draft.citation}
                          onChange={(event) => updateDraft('reference', { citation: event.target.value })}
                        />
                      </label>
                      <label className="research-wide">
                        Import notes
                        <textarea
                          rows={2}
                          value={referenceNotes}
                          onChange={(event) => setReferenceNotes(event.target.value)}
                        />
                      </label>
                    </div>
                  </details>
                </div>
              )}
            </>
          )}

          {reference.error && (
            <p className="research-error" role="alert">
              {reference.error}
            </p>
          )}

          {reference.dataset && (
            <div className="research-import-summary">
              <div>
                <strong>{reference.dataset.axis.values.length.toLocaleString()}</strong> retained rows ·{' '}
                {reference.dataset.signals.length} signals · {reference.dataset.skipped.length} skipped
              </div>
              <code>
                {reference.dataset.raw_sha256
                  ? `sha256:${reference.dataset.raw_sha256.slice(0, 16)}…`
                  : reference.dataset.identity}
              </code>
              <button
                type="button"
                onClick={() =>
                  downloadTextFile(
                    JSON.stringify(reference.dataset, null, 2),
                    `${sanitizeFileStem(reference.dataset!.spec.name)}.kessdata.json`
                  )
                }
              >
                <Download size={14} /> Download dataset JSON
              </button>
            </div>
          )}
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
