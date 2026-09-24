import { BarChart3, CheckCircle2, Download, Plus, Trash2, Upload, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { compare_research_data, import_research_csv, preview_research_csv } from 'kessetsu-core';
import { downloadTextFile, sanitizeFileStem } from '../document';
import {
  column, emptySlot, engineering, logicalName, mappingSpec, parseOptional, researchUnits,
  sourceUnit, type ColumnMapping, type CsvDialect, type CsvPreview, type DataComparison, type DatasetSlot,
  type ResearchData, type ResearchUnit, type SignalPair,
} from '../research';
import './ResearchDataDialog.css';

type SlotId = 'data' | 'reference';
type Step = SlotId | 'compare';

interface Props { open: boolean; onClose: () => void }

function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function MappingRow({ mapping, headers, axis, onChange, onRemove }: {
  mapping: ColumnMapping;
  headers: string[];
  axis?: boolean;
  onChange: (mapping: ColumnMapping) => void;
  onRemove?: () => void;
}) {
  const changeUnit = (unit: ResearchUnit) => onChange({ ...mapping, unit, source_unit: sourceUnit(unit) });
  return <div className="research-mapping-row">
    <label>CSV column<select value={mapping.column} onChange={(event) => onChange({ ...mapping, column: Number(event.target.value) })}>
      {headers.map((header, index) => <option key={index} value={index}>{index + 1}: {header || `Column ${index + 1}`}</option>)}
    </select></label>
    <label>Logical name<input value={mapping.name} onChange={(event) => onChange({ ...mapping, name: event.target.value })} /></label>
    <label>Quantity<select value={mapping.unit} onChange={(event) => changeUnit(event.target.value as ResearchUnit)}>
      {researchUnits.map((unit) => <option key={unit}>{unit}</option>)}
    </select></label>
    <label>CSV unit<input value={mapping.source_unit} aria-label={`${axis ? 'Axis' : 'Signal'} source unit`} onChange={(event) => onChange({ ...mapping, source_unit: event.target.value })} /></label>
    <label>Gain<input type="number" step="any" value={mapping.gain} onChange={(event) => onChange({ ...mapping, gain: Number(event.target.value) })} /></label>
    <label>Offset<input type="number" step="any" value={mapping.offset} onChange={(event) => onChange({ ...mapping, offset: Number(event.target.value) })} /></label>
    {onRemove && <button className="research-icon-button" aria-label={`Remove signal ${mapping.name}`} onClick={onRemove}><Trash2 size={14} /></button>}
  </div>;
}

function DatasetEditor({ slot, role, onRead, onUpdate, onPreview, onImport }: {
  slot: DatasetSlot;
  role: SlotId;
  onRead: (file: File) => void;
  onUpdate: (change: (slot: DatasetSlot) => DatasetSlot) => void;
  onPreview: (dialect: CsvDialect) => void;
  onImport: () => void;
}) {
  const title = role === 'data' ? 'Observed data' : 'Reference data';
  const headers = slot.preview?.headers.length
    ? slot.preview.headers
    : (slot.preview?.sample[0]?.fields ?? []).map((_, index) => `Column ${index + 1}`);
  const updateDraft = (change: Partial<DatasetSlot['draft']>) => onUpdate((current) => ({
    ...current, dataset: null, error: '', draft: { ...current.draft, ...change },
  }));
  const updateDialect = (change: Partial<CsvDialect>) => {
    const dialect = { ...slot.dialect, ...change };
    onUpdate((current) => ({ ...current, dialect, dataset: null, error: '' }));
    if (slot.csv) onPreview(dialect);
  };
  const fileName = slot.file?.name ?? 'No CSV selected';
  return <section className="research-dataset" aria-label={title}>
    <div className="research-file-row">
      <label className="research-file-button"><Upload size={15} /> Choose CSV
        <input className="sr-only" type="file" accept=".csv,text/csv,text/plain" aria-label={`Choose ${title} CSV`}
          onChange={(event) => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (file) onRead(file); }} />
      </label>
      <span title={fileName}>{fileName}</span>
      {slot.dataset && <span className="research-ready"><CheckCircle2 size={14} /> Imported</span>}
    </div>

    {slot.file && <>
      <div className="research-dialect">
        <label>Delimiter<select value={slot.dialect.delimiter} onChange={(event) => updateDialect({ delimiter: event.target.value as CsvDialect['delimiter'] })}>
          <option value="comma">Comma</option><option value="semicolon">Semicolon</option><option value="tab">Tab</option>
        </select></label>
        <label>Decimal<select value={slot.dialect.decimal} onChange={(event) => updateDialect({ decimal: event.target.value as CsvDialect['decimal'] })}>
          <option value="dot">Dot</option><option value="comma">Comma</option>
        </select></label>
        <label>Header<select value={slot.dialect.header ? 'yes' : 'no'} onChange={(event) => updateDialect({ header: event.target.value === 'yes' })}>
          <option value="yes">First record</option><option value="no">No header</option>
        </select></label>
        <label>Skip records<input type="number" min="0" max="128" value={slot.dialect.preamble_records}
          onChange={(event) => updateDialect({ preamble_records: Number(event.target.value) })} /></label>
      </div>

      {slot.preview && <div className="research-preview">
        <div><strong>{slot.preview.data_records.toLocaleString()}</strong> data records · {headers.length} columns · {slot.preview.blank_records} blank</div>
        <div className="research-preview-table"><table><thead><tr><th>Record</th>{headers.map((header, index) => <th key={index}>{header || `Column ${index + 1}`}</th>)}</tr></thead>
          <tbody>{slot.preview.sample.map((row) => <tr key={`${row.record}-${row.line}`}><td>{row.record}</td>{row.fields.map((field, index) => <td key={index} title={field}>{field}</td>)}</tr>)}</tbody>
        </table></div>
      </div>}

      {slot.preview && headers.length > 0 && <div className="research-import-form">
        <div className="research-form-grid">
          <label>Dataset name<input value={slot.draft.name} onChange={(event) => updateDraft({ name: event.target.value })} /></label>
          <label>Origin<select value={slot.draft.origin} onChange={(event) => updateDraft({ origin: event.target.value as DatasetSlot['draft']['origin'] })}>
            <option value="measured">Measured</option><option value="published_simulation">Published simulation</option>
            <option value="synthetic">Synthetic</option><option value="unspecified">Unspecified</option>
          </select></label>
          <label>Axis order<select value={slot.draft.axisOrder} onChange={(event) => updateDraft({ axisOrder: event.target.value as DatasetSlot['draft']['axisOrder'] })}>
            <option value="increasing">Increasing</option><option value="decreasing">Decreasing</option>
          </select></label>
          <label>Missing values<select value={slot.draft.missing} onChange={(event) => updateDraft({ missing: event.target.value as DatasetSlot['draft']['missing'] })}>
            <option value="error">Stop with error</option><option value="skip_row">Skip whole row and record it</option>
          </select></label>
          <label>Missing tokens<input value={slot.draft.missingTokens} onChange={(event) => updateDraft({ missingTokens: event.target.value })} /></label>
        </div>
        <h3>Comparison axis</h3>
        <MappingRow mapping={slot.draft.axis} headers={headers} axis onChange={(axis) => updateDraft({ axis })} />
        <div className="research-section-heading"><h3>Signals</h3><button disabled={slot.draft.signals.length >= 32} onClick={() => {
          const next = slot.draft.signals.length + 1;
          const used = new Set([slot.draft.axis.column, ...slot.draft.signals.map((signal) => signal.column)]);
          const available = headers.findIndex((_, index) => !used.has(index));
          updateDraft({ signals: [...slot.draft.signals, column(available >= 0 ? available : 0, `signal_${next}`, 'Volt')] });
        }}><Plus size={14} /> Add signal</button></div>
        {slot.draft.signals.map((signal, index) => <MappingRow key={index} mapping={signal} headers={headers}
          onChange={(next) => updateDraft({ signals: slot.draft.signals.map((item, i) => i === index ? next : item) })}
          onRemove={slot.draft.signals.length > 1 ? () => updateDraft({ signals: slot.draft.signals.filter((_, i) => i !== index) }) : undefined} />)}
        <details><summary>Identity and citation</summary><div className="research-form-grid">
          <label>Device<input value={slot.draft.device} onChange={(event) => updateDraft({ device: event.target.value })} /></label>
          <label>Sample / run<input value={slot.draft.sample} onChange={(event) => updateDraft({ sample: event.target.value })} /></label>
          <label className="research-wide">Citation or source<textarea rows={2} value={slot.draft.citation} onChange={(event) => updateDraft({ citation: event.target.value })} /></label>
        </div></details>
        <p className="research-note">The starting column choices are only a convenience. Import uses exactly the columns, units, gain and offset shown above; no values are guessed or silently sorted.</p>
        <button className="research-primary" onClick={onImport}>Import mapped dataset</button>
      </div>}
    </>}
    {slot.error && <p className="research-error" role="alert">{slot.error}</p>}
    {slot.dataset && <div className="research-import-summary">
      <div><strong>{slot.dataset.axis.values.length.toLocaleString()}</strong> retained rows · {slot.dataset.signals.length} signals · {slot.dataset.skipped.length} skipped</div>
      <code>{slot.dataset.identity}</code>
      <button onClick={() => downloadTextFile(JSON.stringify(slot.dataset, null, 2), `${sanitizeFileStem(slot.dataset!.spec.name)}.kessdata.json`)}><Download size={14} /> Download dataset JSON</button>
    </div>}
  </section>;
}

function polyline(points: Array<{ axis: number; value: number }>, x: (value: number) => number, y: (value: number) => number) {
  if (points.length > 2400) {
    const stride = Math.ceil(points.length / 2400);
    points = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
  }
  return points.map((point) => `${x(point.axis).toFixed(2)},${y(point.value).toFixed(2)}`).join(' ');
}

function ComparisonPlot({ comparison, signalIndex }: { comparison: DataComparison; signalIndex: number }) {
  const signal = comparison.signals[signalIndex];
  const matched = signal.points.filter((point) => point.predicted !== null);
  if (matched.length === 0) return <p className="research-note">No matched points are available to plot.</p>;
  const axes = matched.map((point) => point.axis);
  const values = matched.flatMap((point) => [point.observed, point.predicted!]);
  let minX = Math.min(...axes); let maxX = Math.max(...axes);
  let minY = Math.min(...values); let maxY = Math.max(...values);
  if (minX === maxX) { minX -= .5; maxX += .5; }
  if (minY === maxY) { const pad = Math.abs(minY) * .05 || .5; minY -= pad; maxY += pad; }
  const x = (value: number) => 62 + (value - minX) / (maxX - minX) * 786;
  const y = (value: number) => 220 - (value - minY) / (maxY - minY) * 184;
  const observed = matched.map((point) => ({ axis: point.axis, value: point.observed }));
  const predicted = matched.map((point) => ({ axis: point.axis, value: point.predicted! }));
  return <figure className="research-plot">
    <svg viewBox="0 0 900 260" role="img" aria-label={`${signal.mapping.data_signal} observed and reference overlay`}>
      <rect x="62" y="36" width="786" height="184" />
      {[0, .25, .5, .75, 1].map((part) => <g key={part}><line x1="62" x2="848" y1={36 + part * 184} y2={36 + part * 184} /><text x="54" y={40 + part * 184} textAnchor="end">{engineering(maxY - part * (maxY - minY))}</text></g>)}
      <polyline className="research-observed" points={polyline(observed, x, y)} />
      <polyline className="research-reference" points={polyline(predicted, x, y)} />
      <text x="62" y="242">{engineering(minX)}</text><text x="848" y="242" textAnchor="end">{engineering(maxX)} {comparison.axis_unit}</text>
    </svg>
    <figcaption><span className="observed-key" /> Observed: {signal.mapping.data_signal}<span className="reference-key" /> Reference: {signal.mapping.reference_signal} · {signal.unit}</figcaption>
  </figure>;
}

export function ResearchDataDialog({ open, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [step, setStep] = useState<Step>('data');
  const [data, setData] = useState(() => emptySlot('Observed dataset'));
  const [reference, setReference] = useState(() => emptySlot('Reference dataset'));
  const [pairs, setPairs] = useState<SignalPair[]>([{ data_signal: '', reference_signal: '' }]);
  const [comparisonName, setComparisonName] = useState('Observed data versus reference');
  const [coverage, setCoverage] = useState<'require_full' | 'overlap_only'>('require_full');
  const [interpolation, setInterpolation] = useState<'linear' | 'log_axis'>('linear');
  const [shift, setShift] = useState('0');
  const [windowStart, setWindowStart] = useState('');
  const [windowStop, setWindowStop] = useState('');
  const [maxGap, setMaxGap] = useState('');
  const [comparison, setComparison] = useState<DataComparison | null>(null);
  const [comparisonError, setComparisonError] = useState('');
  const [plotSignal, setPlotSignal] = useState(0);

  useEffect(() => {
    const current = dialog.current;
    if (!current) return;
    if (open && !current.open) current.showModal();
    if (!open && current.open) current.close();
  }, [open]);

  const slot = (id: SlotId) => id === 'data' ? data : reference;
  const setSlot = (id: SlotId, next: DatasetSlot | ((current: DatasetSlot) => DatasetSlot)) => {
    const setter = id === 'data' ? setData : setReference;
    setter(next);
    setComparison(null); setComparisonError('');
  };
  const preview = (id: SlotId, dialect: CsvDialect, csv = slot(id).csv) => {
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
      setSlot(id, (current) => ({ ...current, file, csv: '', preview: null, dataset: null, error: 'CSV must contain 1 byte-8 MiB' })); return;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const csv = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes);
      const current = slot(id);
      const result = preview_research_csv(csv, current.dialect) as CsvPreview;
      const headers = result.headers.length ? result.headers : (result.sample[0]?.fields ?? []).map((_, index) => `column_${index + 1}`);
      const axisName = logicalName(headers[0] ?? '', 'axis');
      const signalName = logicalName(headers[1] ?? '', 'signal');
      setSlot(id, {
        ...current, file, csv, preview: result, dataset: null, error: '',
        draft: { ...current.draft, name: file.name.replace(/\.csv$/i, '') || current.draft.name,
          axis: { ...current.draft.axis, column: 0, name: axisName },
          signals: [{ ...current.draft.signals[0], column: headers.length > 1 ? 1 : 0, name: signalName }],
        },
      });
    } catch (cause) {
      setSlot(id, (current) => ({ ...current, file, csv: '', preview: null, dataset: null, error: `Could not read CSV: ${message(cause)}` }));
    }
  };
  const importSlot = (id: SlotId) => {
    const current = slot(id);
    try {
      const dataset = import_research_csv(current.csv, mappingSpec(current)) as ResearchData;
      setSlot(id, { ...current, dataset, error: '' });
      if (id === 'data') setStep('reference'); else setStep('compare');
    } catch (cause) {
      setSlot(id, { ...current, dataset: null, error: message(cause) });
    }
  };

  useEffect(() => {
    if (!data.dataset || !reference.dataset) return;
    const first = pairs[0];
    if (!first.data_signal || !data.dataset.signals.some((signal) => signal.name === first.data_signal)
      || !reference.dataset.signals.some((signal) => signal.name === first.reference_signal)) {
      setPairs([{ data_signal: data.dataset.signals[0]?.name ?? '', reference_signal: reference.dataset.signals[0]?.name ?? '' }]);
    }
  }, [data.dataset, reference.dataset, pairs]);

  const canCompare = Boolean(data.dataset && reference.dataset);
  const compare = () => {
    if (!data.dataset || !reference.dataset) return;
    try {
      const start = parseOptional(windowStart, 'Window start');
      const stop = parseOptional(windowStop, 'Window stop');
      if ((start === null) !== (stop === null)) throw new Error('Set both window start and stop, or leave both empty');
      const spec = {
        schema_version: 'kessetsu.data-comparison.v1', name: comparisonName, signals: pairs,
        coverage, interpolation, reference_axis_shift: parseOptional(shift, 'Reference axis shift') ?? 0,
        window: start === null ? null : [start, stop], max_reference_gap: parseOptional(maxGap, 'Maximum reference gap'),
      };
      const result = compare_research_data(data.dataset, reference.dataset, spec) as DataComparison;
      setComparison(result); setComparisonError(''); setPlotSignal(0);
    } catch (cause) { setComparison(null); setComparisonError(message(cause)); }
  };
  const selectedSignal = comparison?.signals[plotSignal];
  const totalRetained = useMemo(() => (data.dataset?.axis.values.length ?? 0) + (reference.dataset?.axis.values.length ?? 0), [data.dataset, reference.dataset]);

  return <dialog ref={dialog} className="app-dialog research-dialog" aria-labelledby="research-data-title" onClose={onClose}>
    <header><div><h2 id="research-data-title">Research data</h2><p>Map and compare local CSV evidence. Files stay in this browser tab.</p></div><button aria-label="Close research data" onClick={() => dialog.current?.close()}><X size={17} /></button></header>
    <nav className="research-steps" aria-label="Research data steps">
      <button aria-pressed={step === 'data'} onClick={() => setStep('data')}>1 · Observed {data.dataset && <CheckCircle2 size={13} />}</button>
      <button aria-pressed={step === 'reference'} onClick={() => setStep('reference')}>2 · Reference {reference.dataset && <CheckCircle2 size={13} />}</button>
      <button aria-pressed={step === 'compare'} disabled={!canCompare} onClick={() => setStep('compare')}>3 · Compare</button>
    </nav>
    {step !== 'compare' ? <DatasetEditor slot={step === 'data' ? data : reference} role={step}
      onRead={(file) => void read(step, file)}
      onUpdate={(change) => setSlot(step, change)}
      onPreview={(dialect) => preview(step, dialect)} onImport={() => importSlot(step)} /> : <section className="research-comparison" aria-label="Compare research datasets">
      <div className="research-comparison-summary"><article><span>Observed</span><strong>{data.dataset?.spec.name}</strong><small>{data.dataset?.spec.metadata.origin} · {data.dataset?.axis.values.length} rows</small></article><article><span>Reference</span><strong>{reference.dataset?.spec.name}</strong><small>{reference.dataset?.spec.metadata.origin} · {reference.dataset?.axis.values.length} rows</small></article></div>
      <div className="research-form-grid">
        <label className="research-wide">Comparison name<input value={comparisonName} onChange={(event) => { setComparisonName(event.target.value); setComparison(null); }} /></label>
        <label>Coverage<select value={coverage} onChange={(event) => { setCoverage(event.target.value as typeof coverage); setComparison(null); }}><option value="require_full">Require every observed point</option><option value="overlap_only">Keep unmatched points</option></select></label>
        <label>Interpolation<select value={interpolation} onChange={(event) => { setInterpolation(event.target.value as typeof interpolation); setComparison(null); }}><option value="linear">Linear axis</option><option value="log_axis">Log-frequency axis</option></select></label>
        <label>Reference axis shift<input value={shift} onChange={(event) => { setShift(event.target.value); setComparison(null); }} /></label>
        <label>Maximum reference gap<input placeholder="No limit" value={maxGap} onChange={(event) => { setMaxGap(event.target.value); setComparison(null); }} /></label>
        <label>Window start<input placeholder="All data" value={windowStart} onChange={(event) => { setWindowStart(event.target.value); setComparison(null); }} /></label>
        <label>Window stop<input placeholder="All data" value={windowStop} onChange={(event) => { setWindowStop(event.target.value); setComparison(null); }} /></label>
      </div>
      <div className="research-section-heading"><h3>Signal pairs</h3><button disabled={pairs.length >= 16} onClick={() => setPairs((current) => [...current, { data_signal: data.dataset?.signals[0]?.name ?? '', reference_signal: reference.dataset?.signals[0]?.name ?? '' }])}><Plus size={14} /> Add pair</button></div>
      {pairs.map((pair, index) => <div className="research-pair" key={index}><label>Observed signal<select value={pair.data_signal} onChange={(event) => { setPairs((current) => current.map((item, i) => i === index ? { ...item, data_signal: event.target.value } : item)); setComparison(null); }}>{data.dataset?.signals.map((signal) => <option key={signal.name}>{signal.name}</option>)}</select></label><label>Reference signal<select value={pair.reference_signal} onChange={(event) => { setPairs((current) => current.map((item, i) => i === index ? { ...item, reference_signal: event.target.value } : item)); setComparison(null); }}>{reference.dataset?.signals.map((signal) => <option key={signal.name}>{signal.name}</option>)}</select></label>{pairs.length > 1 && <button className="research-icon-button" aria-label={`Remove comparison pair ${index + 1}`} onClick={() => { setPairs((current) => current.filter((_, i) => i !== index)); setComparison(null); }}><Trash2 size={14} /></button>}</div>)}
      <p className="research-note">Residual means reference minus observed. Kessetsu never extrapolates or auto-aligns axes. Log interpolation is available only for positive frequency axes.</p>
      <button className="research-primary" onClick={compare}><BarChart3 size={15} /> Compare datasets</button>
      {comparisonError && <p className="research-error" role="alert">{comparisonError}</p>}
      {comparison && <div className="research-results">
        <div className="research-result-header"><div><strong>Comparison complete</strong><small>{totalRetained.toLocaleString()} source rows retained across both datasets · {comparison.identity}</small></div><button onClick={() => downloadTextFile(JSON.stringify(comparison, null, 2), `${sanitizeFileStem(comparison.spec.name)}.kesscompare.json`)}><Download size={14} /> Download evidence JSON</button></div>
        <div className="research-metrics"><table><thead><tr><th>Signal</th><th>Coverage</th><th>Bias</th><th>MAE</th><th>RMSE</th><th>Max |error|</th></tr></thead><tbody>{comparison.signals.map((signal, index) => <tr key={signal.mapping.data_signal} className={plotSignal === index ? 'selected' : ''} onClick={() => setPlotSignal(index)}><td>{signal.mapping.data_signal} vs {signal.mapping.reference_signal}<small>{signal.unit}</small></td><td>{signal.metrics.matched}/{signal.metrics.total}<small>{signal.metrics.unmatched} unmatched · {signal.metrics.excluded_by_window} excluded</small></td><td>{engineering(signal.metrics.bias)}</td><td>{engineering(signal.metrics.mae)}</td><td>{engineering(signal.metrics.rmse)}</td><td>{engineering(signal.metrics.max_absolute)}</td></tr>)}</tbody></table></div>
        {selectedSignal && <ComparisonPlot comparison={comparison} signalIndex={plotSignal} />}
      </div>}
    </section>}
  </dialog>;
}
