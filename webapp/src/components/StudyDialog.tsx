import { X, Play, Square, Plus, Download, RotateCcw } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import coreWasmUrl from 'kessetsu-core/kessetsu_core_bg.wasm?url';
import {
  study_parameters, plan_study, prepare_study_case, start_study_results,
  evaluate_study_case, inspect_study_results, resume_study_results,
  export_study_results, study_effective_source, compare_study_plot, prepare_browser_simulation,
} from 'kessetsu-core';
import { downloadTextFile, sanitizeFileStem } from '../document';
import { newStudy, parameterAxis, reusable, suffix, type StudySpec, type StudyPlan, type StudyResults, type StudyParameter, type StudyCaseResult, type Unit, type AxisValues } from '../experiment';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserSimulationPlan } from '../simulation/types';
import './StudyDialog.css';

interface Props { open: boolean; source: string; name: string; resources: Record<string, number[]>; onClose: () => void; onApplySource: (source: string) => void }
const message = (error: unknown) => error instanceof Error ? error.message : String(error);
const probeSource = 'net GND\nnet OUT\nsource V1 1V\nresistor R1 1k\nconnect V1.plus, R1.p1 to OUT\nconnect V1.minus, R1.p2 to GND\nsimulate op\n';
const signalLabel = (key: string) => key.endsWith('#branch') ? `I(${key.slice(0, -7)})` : key.startsWith('@') ? key : `V(${key})`;
type Tolerance = NonNullable<StudySpec['tolerances']>;

export function StudyDialog({ open, source, name, resources, onClose, onApplySource }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const runner = useRef<BrowserSimulationRunner | null>(null);
  if (!runner.current) runner.current = new BrowserSimulationRunner();
  const cancelled = useRef(false); const active = useRef(false);
  const initialized = useRef(false);
  const [spec, setSpec] = useState<StudySpec>(() => newStudy(source, name));
  const [parameters, setParameters] = useState<StudyParameter[]>([]);
  const [parameterError, setParameterError] = useState('');
  const [results, setResults] = useState<StudyResults | null>(null);
  const [comparison, setComparison] = useState<StudyResults | null>(null);
  const [running, setRunning] = useState(false); const [progress, setProgress] = useState('');
  const [error, setError] = useState(''); const [tab, setTab] = useState<'configure' | 'results'>('configure');
  const [selected, setSelected] = useState<string[]>([]); const [signal, setSignal] = useState('V(OUT)');
  const [analysis, setAnalysis] = useState(0); const [advancedJson, setAdvancedJson] = useState('');
  const [temperatureText, setTemperatureText] = useState('27');
  const [plot, setPlot] = useState(''); const [plotError, setPlotError] = useState('');
  const [exportTarget, setExportTarget] = useState('json');
  const importInput = useRef<HTMLInputElement>(null); const compareInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open && !initialized.current) {
      initialized.current = true;
      const next = newStudy(source, name);
      try { const roots = study_parameters(source, resources) as StudyParameter[]; if (roots[0]) next.axes = [parameterAxis(roots[0])]; } catch { /* Diagnostic below. */ }
      setSpec(next);
    }
    if (open && !dialog.current?.open) dialog.current?.showModal();
    if (!open && dialog.current?.open) dialog.current?.close();
  }, [open, source, name, resources]);
  useEffect(() => () => { cancelled.current = true; runner.current?.dispose(); }, []);
  useEffect(() => {
    if (!running) return;
    const warn = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = ''; };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [running]);
  useEffect(() => { if (!open && active.current) { cancelled.current = true; runner.current?.cancel(); } }, [open]);
  useEffect(() => {
    if (active.current) { cancelled.current = true; runner.current?.cancel(); }
    if (!open) return;
    try { setParameters(study_parameters(spec.source, resources) as StudyParameter[]); setParameterError(''); }
    catch (cause) { setParameters([]); setParameterError(message(cause)); }
  }, [spec.source, resources, open]);
  const update = (change: Partial<StudySpec>) => { setSpec(current => ({ ...current, ...change })); setError(''); };
  const axisUpdate = (index: number, change: Partial<StudySpec['axes'][number]>) => update({ axes: spec.axes.map((axis, i) => i === index ? { ...axis, ...change } : axis) });
  const toleranceUpdate = (index: number, change: Partial<Tolerance['parameters'][number]>) => {
    if (spec.tolerances) update({ tolerances: { ...spec.tolerances, parameters: spec.tolerances.parameters.map((p, i) => i === index ? { ...p, ...change } : p) } });
  };
  const measureUpdate = (index: number, change: Partial<StudySpec['measurements'][number]>) => update({ measurements: spec.measurements.map((m, i) => i === index ? { ...m, ...change } : m) });
  const acceptSpec = (next: StudySpec) => { setSpec(next); setTemperatureText(next.temperatures_c.join(', ')); setAdvancedJson(''); setError(''); };
  const reset = () => {
    const next = newStudy(source, name);
    try { const roots = study_parameters(source, resources) as StudyParameter[]; if (roots[0]) next.axes = [parameterAxis(roots[0])]; }
    catch { /* Canonical planning owns diagnostics. */ }
    acceptSpec(next); setTab('configure');
  };
  const mode = spec.tolerances?.mode ?? 'sweep';
  const changeMode = (next: string) => {
    if (next === 'sweep') update({ tolerances: null, axes: parameters[0] ? [parameterAxis(parameters[0])] : [] });
    else update({ axes: [], tolerances: { mode: next as Tolerance['mode'], seed: 42, samples: 12, parameters: parameters[0] ? [{ parameter: parameters[0].name, relative: 0.05, distribution: 'uniform', group: null }] : [] } });
  };
  const runStudy = async (resume: boolean) => {
    if (active.current) return;
    active.current = true; cancelled.current = false; setRunning(true); setError(''); setProgress('Preparing study…');
    let current: StudyResults | null = null;
    const bound = resources; const snapshot = spec; const solver = runner.current!;
    try {
      const plan = plan_study(snapshot, bound) as StudyPlan;
      if (plan.cases.length > 32) throw new Error(`This study has ${plan.cases.length} cases. Browser limit: 32. Download the specification and run it with the CLI (up to 256 cases).`);
      const response = await fetch(coreWasmUrl); if (!response.ok) throw new Error('Could not identify the Core runtime');
      const hash = await crypto.subtle.digest('SHA-256', await response.arrayBuffer());
      const fingerprint = `eecircuit-engine@1.7.0:sha256:${Array.from(new Uint8Array(hash), b => b.toString(16).padStart(2, '0')).join('')}`;
      if (cancelled.current) throw new SimulationCancelledError();
      setProgress('Identifying local simulator…');
      const probe = await solver.run(prepare_browser_simulation(probeSource) as BrowserSimulationPlan, { timeoutMs: 30_000 });
      if (!resume && results && !comparison) setComparison(results);
      current = resume && results ? resume_study_results(results, snapshot, bound, probe.simulator, fingerprint) as StudyResults : start_study_results(plan, probe.simulator, fingerprint) as StudyResults;
      setResults(current); setTab('results'); setSelected(plan.cases.slice(0, 12).map(c => c.id));
      for (let index = 0; index < plan.cases.length; index++) {
        if (cancelled.current) break;
        if (reusable(current.cases[index].status)) continue;
        setProgress(`Running case ${index + 1} of ${plan.cases.length}…`);
        let row: StudyCaseResult;
        try {
          const simulationPlan = prepare_study_case(plan, index, bound) as BrowserSimulationPlan;
          const simulation = await solver.run(simulationPlan, { timeoutMs: snapshot.timeout_ms });
          row = evaluate_study_case(plan, index, simulation, bound, '', false) as StudyCaseResult;
        } catch (cause) { row = evaluate_study_case(plan, index, null, bound, message(cause), cancelled.current || cause instanceof SimulationCancelledError) as StudyCaseResult; }
        current = { ...current, cases: current.cases.map((previous, i) => i === index ? row : previous) };
        if (JSON.stringify(current).length > 32 * 1024 * 1024) {
          current.cases[index] = evaluate_study_case(plan, index, null, bound, 'Browser retained-data limit reached (32 MiB). Narrow the study or use the CLI.', false) as StudyCaseResult;
          cancelled.current = true;
        }
        setResults(current);
      }
      current = inspect_study_results(current) as StudyResults; setResults(current);
      setProgress(cancelled.current ? 'Study stopped. Partial results retained; use Resume or download Results JSON.' : 'Study complete. Every case remains in the report.');
    } catch (cause) {
      setError(message(cause));
      setProgress(cancelled.current ? 'Study stopped. Existing partial results are retained.' : 'Study not completed. See the diagnostic above.');
      if (current) { setResults(inspect_study_results(current) as StudyResults); setTab('results'); }
    } finally { active.current = false; setRunning(false); }
  };
  const cancel = () => { cancelled.current = true; runner.current?.cancel(); setProgress('Stopping study…'); };
  const importFile = async (file: File, compare: boolean) => {
    try {
      if (file.size > (compare ? 32 : 64) * 1024 * 1024) throw new Error('Study file is too large');
      const parsed = JSON.parse(await file.text()) as StudySpec | StudyResults;
      if (compare) {
        if (parsed.schema_version !== 'kessetsu.experiment-results.v1') throw new Error('Choose Results JSON to compare');
        setComparison(inspect_study_results(parsed) as StudyResults); setTab('results');
      } else if (parsed.schema_version === 'kessetsu.experiment-results.v1') {
        const report = inspect_study_results(parsed) as StudyResults;
        setResults(report); acceptSpec(report.plan.spec); setSelected(report.plan.cases.slice(0, 12).map(c => c.id)); setTab('results');
      } else { acceptSpec((plan_study(parsed, resources) as StudyPlan).spec); setTab('configure'); }
      setError('');
    } catch (cause) { setError(message(cause)); }
  };
  const exportFile = () => {
    if (!results) return;
    try {
      const content = exportTarget === 'svg' && comparison ? compare_study_plot(results, comparison, analysis, signal) : export_study_results(results, exportTarget, analysis, signal, selected);
      const extension = exportTarget === 'data_csv' ? 'csv' : exportTarget;
      downloadTextFile(content, `${sanitizeFileStem(results.plan.spec.name)}${exportTarget === 'data_csv' ? '-samples' : ''}.${extension}`); setError('');
    } catch (cause) { setError(message(cause)); }
  };
  useEffect(() => {
    if (!results || running || selected.length === 0) { setPlot(''); return; }
    try { setPlot(comparison ? compare_study_plot(results, comparison, analysis, signal) : export_study_results(results, 'svg', analysis, signal, selected)); setPlotError(''); }
    catch (cause) { setPlot(''); setPlotError(message(cause)); }
  }, [results, comparison, analysis, signal, selected, running]);
  const firstSimulation = results?.cases.find(row => row.simulation)?.simulation;
  const dataset = firstSimulation?.datasets.find(data => data.index === analysis)?.data;
  const signals = dataset && dataset.kind !== 'operating_point' ? Object.keys(dataset.signals).map(signalLabel) : [];
  const counts = results?.cases.reduce((acc, row) => { acc[row.status] = (acc[row.status] ?? 0) + 1; return acc; }, {} as Record<string, number>);
  const parameterOptions = parameters.map(p => <option key={p.name} value={p.name}>{p.name} ({suffix[p.declared_unit] || 'ratio'})</option>);

  return <dialog ref={dialog} className="app-dialog study-dialog" aria-labelledby="study-title" onClose={() => { if (active.current) cancel(); onClose(); }}>
    <header><div><h2 id="study-title">Parameter study</h2><p>Try conditions, keep every outcome and compare the results.</p></div><button aria-label="Close parameter study" onClick={() => dialog.current?.close()}><X size={18} /></button></header>
    <nav className="study-tabs" aria-label="Study sections"><button aria-pressed={tab === 'configure'} onClick={() => setTab('configure')}>Configure</button><button aria-pressed={tab === 'results'} disabled={!results} onClick={() => setTab('results')}>Results</button><button onClick={() => importInput.current?.click()} disabled={running}>Open study / results…</button></nav>
    <input ref={importInput} type="file" className="sr-only" accept=".json" aria-label="Open study file" onChange={event => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (file) void importFile(file, false); }} />
    <input ref={compareInput} type="file" className="sr-only" accept=".json" aria-label="Compare study results file" onChange={event => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (file) void importFile(file, true); }} />
    {error && <p className="study-error" role="alert">{error}</p>}
    {tab === 'configure' ? <fieldset className="study-config" disabled={running}>
      <div className="study-row"><label>Study name<input value={spec.name} onChange={event => update({ name: event.target.value })} /></label><button onClick={reset}><RotateCcw size={14} /> Use current circuit</button></div>
      <p className="study-note">A source snapshot, not changes to the open circuit. Local models stay separate and are never uploaded.</p>
      {spec.source !== source && <p className="study-note">This study uses a different source snapshot from the editor. Choose “Use current circuit” to start from the current source.</p>}
      {parameterError && <p role="alert">{parameterError} Open the study source in the editor and choose its matching model files.</p>}
      {parameters.length === 0 && !parameterError && <p>Add named root <code>param</code> declarations, or open a Loaded Filter / Transistor Driver example. With no axes, only the nominal design runs.</p>}
      <label>Study type<select value={mode} onChange={event => changeMode(event.target.value)}><option value="sweep">Parameter sweep</option><option value="corners">Nominal + tolerance corners</option><option value="monte_carlo">Nominal + Monte Carlo samples</option></select></label>
      {!spec.tolerances ? <section><h3>Sweep parameters</h3>{spec.axes.map((axis, index) => <div className="study-axis" key={index}>
        <label>Parameter<select aria-label={`Sweep parameter ${index + 1}`} value={axis.parameter} onChange={event => { const parameter = parameters.find(p => p.name === event.target.value); if (parameter) axisUpdate(index, parameterAxis(parameter)); }}>{parameterOptions}</select></label>
        <label>Values<select aria-label={`Sweep method ${index + 1}`} value={axis.values.kind} onChange={event => {
          const p = parameters.find(p => p.name === axis.parameter); const value = p?.resolved.value ?? 1; const unit = p ? suffix[p.declared_unit] : '';
          const values: AxisValues = event.target.value === 'list' ? { kind: 'list', values: [`${value}${unit}`] } : { kind: event.target.value as 'linear' | 'log', start: `${value * 0.5}${unit}`, stop: `${value * 2}${unit}`, points: 5 }; axisUpdate(index, { values });
        }}><option value="list">Explicit list</option><option value="linear">Linear range</option><option value="log">Log range</option></select></label>
        {axis.values.kind === 'list' ? <label className="study-values">Comma-separated values<input aria-label={`Sweep values ${index + 1}`} value={axis.values.values.join(', ')} onChange={event => axisUpdate(index, { values: { kind: 'list', values: event.target.value.split(',').map(v => v.trim()) } })} /></label>
          : <><label>Start<input value={axis.values.start} onChange={event => { if (axis.values.kind !== 'list') axisUpdate(index, { values: { ...axis.values, start: event.target.value } }); }} /></label><label>Stop<input value={axis.values.stop} onChange={event => { if (axis.values.kind !== 'list') axisUpdate(index, { values: { ...axis.values, stop: event.target.value } }); }} /></label><label>Points<input type="number" min="2" max="256" value={axis.values.points} onChange={event => { if (axis.values.kind !== 'list') axisUpdate(index, { values: { ...axis.values, points: Number(event.target.value) } }); }} /></label></>}
        <button aria-label={`Remove sweep ${index + 1}`} onClick={() => update({ axes: spec.axes.filter((_, i) => i !== index) })}><X size={14} /></button>
      </div>)}<button disabled={spec.axes.length >= 8 || parameters.length === 0} onClick={() => { const p = parameters.find(p => !spec.axes.some(a => a.parameter === p.name)); if (p) update({ axes: [...spec.axes, parameterAxis(p)] }); }}><Plus size={14} /> Add parameter</button></section>
        : <section><h3>Tolerances</h3>{spec.tolerances.parameters.map((p, index) => <div className="study-axis" key={index}>
          <label>Parameter<select value={p.parameter} onChange={event => toleranceUpdate(index, { parameter: event.target.value })}>{parameterOptions}</select></label>
          <label>{p.distribution === 'normal' ? 'Standard deviation (%)' : 'Half-width (±%)'}<input type="number" min="0.001" max="99" value={p.relative * 100} onChange={event => toleranceUpdate(index, { relative: Number(event.target.value) / 100 })} /></label>
          {mode === 'monte_carlo' && <><label>Distribution<select value={p.distribution} onChange={event => toleranceUpdate(index, { distribution: event.target.value as 'uniform' | 'normal' })}><option value="uniform">Uniform</option><option value="normal">Gaussian</option></select></label><label>Correlation group (optional)<input value={p.group ?? ''} onChange={event => toleranceUpdate(index, { group: event.target.value || null })} /></label></>}
          <button aria-label={`Remove tolerance ${index + 1}`} onClick={() => update({ tolerances: { ...spec.tolerances!, parameters: spec.tolerances!.parameters.filter((_, i) => i !== index) } })}><X size={14} /></button>
        </div>)}<button disabled={spec.tolerances.parameters.length >= 8 || parameters.length === 0} onClick={() => { const p = parameters.find(p => !spec.tolerances!.parameters.some(t => t.parameter === p.name)); if (p) update({ tolerances: { ...spec.tolerances!, parameters: [...spec.tolerances!.parameters, { parameter: p.name, relative: 0.05, distribution: 'uniform', group: null }] } }); }}><Plus size={14} /> Add tolerance</button>
          {mode === 'monte_carlo' && <div className="study-row"><label>Samples (plus nominal)<input type="number" min="1" max="255" value={spec.tolerances.samples} onChange={event => update({ tolerances: { ...spec.tolerances!, samples: Number(event.target.value) } })} /></label><label>Random seed<input type="number" min="0" max="4294967295" value={spec.tolerances.seed} onChange={event => update({ tolerances: { ...spec.tolerances!, seed: Number(event.target.value) } })} /></label></div>}
          <p className="study-note">Same-group parameters share a signed deviation. Gaussian samples are unbounded; invalid values stay failed cases. Results are not guaranteed production yield.</p></section>}
      <details><summary>Measurements, constraints and search</summary><p>Use ordinary metric calls without a comparison. Circuit assertions remain constraints. Search selects only the best feasible evaluated case, not a global optimum.</p>
        {spec.measurements.map((m, index) => <div className="study-axis" key={index}><label>Name<input value={m.name} onChange={event => measureUpdate(index, { name: event.target.value })} /></label><label className="study-values">Metric expression<input placeholder="cutoff(V(OUT),V(IN))" value={m.expression} onChange={event => measureUpdate(index, { expression: event.target.value })} /></label><label>Unit<select value={m.unit} onChange={event => measureUpdate(index, { unit: event.target.value as Unit })}>{Object.keys(suffix).map(unit => <option key={unit}>{unit}</option>)}</select></label><button aria-label={`Remove measurement ${index + 1}`} onClick={() => update({ measurements: spec.measurements.filter((_, i) => i !== index), objective: null })}><X size={14} /></button></div>)}
        <button onClick={() => update({ measurements: [...spec.measurements, { name: `metric_${spec.measurements.length + 1}`, expression: 'cutoff(V(OUT),V(IN))', unit: 'Hertz' }] })}><Plus size={14} /> Add measurement</button>
        <div className="study-row"><label>Search objective<select value={spec.objective?.measurement ?? ''} onChange={event => update({ objective: event.target.value ? { measurement: event.target.value, direction: 'minimize' } : null })}><option value="">None — compare all cases</option>{spec.measurements.map(m => <option key={m.name}>{m.name}</option>)}</select></label>{spec.objective && <label>Direction<select value={spec.objective.direction} onChange={event => update({ objective: { ...spec.objective!, direction: event.target.value as 'minimize' | 'maximize' } })}><option value="minimize">Minimize</option><option value="maximize">Maximize</option></select></label>}</div>
        <label>Independent literal .kessreq constraints (optional)<textarea rows={3} value={spec.requirements ?? ''} placeholder="assert gain_at(V(OUT),V(IN),1kHz) > 0.6" onChange={event => update({ requirements: event.target.value || null })} /></label><p className="study-note">Independent requirements cannot mix with inline assertions or reference design parameters.</p>
      </details>
      <details><summary>Conditions and portable specification</summary><div className="study-row"><label>Temperatures (°C, comma-separated)<input value={temperatureText} onChange={event => { setTemperatureText(event.target.value); update({ temperatures_c: event.target.value.split(',').map(v => v.trim() ? Number(v.trim()) : NaN) }); }} /></label><label>Per-case timeout (seconds)<input type="number" min="0.001" max="120" value={spec.timeout_ms / 1000} onChange={event => update({ timeout_ms: Math.round(Number(event.target.value) * 1000) })} /></label></div>
        <p className="study-note">Temperature affects only models with temperature-dependent equations. Ideal passives and ideal generic op-amps do not acquire new temperature coefficients.</p>
        <button onClick={() => { try { const checked = (plan_study(spec, resources) as StudyPlan).spec; downloadTextFile(JSON.stringify(checked, null, 2), `${sanitizeFileStem(checked.name)}.kessstudy.json`); setError(''); } catch (cause) { setError(message(cause)); } }}><Download size={14} /> Download specification</button>
        <details><summary>Advanced JSON: revisions and combined studies</summary><textarea rows={10} aria-label="Advanced study specification" value={advancedJson || JSON.stringify(spec, null, 2)} onChange={event => setAdvancedJson(event.target.value)} /><button onClick={() => { try { acceptSpec((plan_study(JSON.parse(advancedJson || JSON.stringify(spec)), resources) as StudyPlan).spec); } catch (cause) { setError(message(cause)); } }}>Apply specification</button></details>
      </details>
    </fieldset> : <section className="study-results" aria-label="Study results">{results && <>
      <p className="study-summary" data-testid="study-summary">{results.plan.cases.length} total · {counts?.passed ?? 0} passed · {counts?.completed ?? 0} completed without constraints · {counts?.failed ?? 0} failed · {counts?.error ?? 0} errors · {counts?.cancelled ?? 0} cancelled · {counts?.pending ?? 0} pending</p>
      {!running && results.summary.best_case && <p>Best feasible evaluated case: <strong>{results.summary.best_case}</strong>. Search budget: {results.summary.total} cases.</p>}
      {!running && results.plan.spec.objective && !results.summary.best_case && <p>No feasible evaluated candidate met the study constraints.</p>}
      <div className="study-row"><label>Analysis<select value={analysis} onChange={event => setAnalysis(Number(event.target.value))}>{firstSimulation?.datasets.map(d => <option key={d.index} value={d.index}>{d.index + 1}: {d.analysis.kind}</option>)}</select></label><label>Overlay signal<select value={signal} onChange={event => setSignal(event.target.value)}><option value={signal}>{signal}</option>{signals.filter(s => s.toLowerCase() !== signal.toLowerCase()).map(s => <option key={s}>{s}</option>)}</select></label><button disabled={running} onClick={() => compareInput.current?.click()}>Compare another Results JSON…</button>{comparison && <button onClick={() => setComparison(null)}>Clear comparison</button>}</div>
      {comparison && <p>Comparing with {comparison.plan.spec.name}. Up to six cases per report are plotted; original files retain every case.</p>}
      {plot && <div className="study-plot" data-testid="study-plot" dangerouslySetInnerHTML={{ __html: plot }} />}{!running && plotError && <p className="study-note">{plotError}</p>}
      <div className="study-table"><table><thead><tr><th>Overlay</th><th>Case / conditions</th><th>Status</th><th>Measured outcomes</th><th>Continue</th></tr></thead><tbody>{results.plan.cases.map((point, index) => {
const row = results.cases[index]; return <tr key={point.id} className={`study-case-${row.status}`}><td><input type="checkbox" aria-label={`Overlay ${point.id}`} checked={selected.includes(point.id)} disabled={running || comparison !== null} onChange={event => setSelected(current => event.target.checked ? [...current, point.id].slice(0, 12) : current.filter(id => id !== point.id))} /></td><td><strong>{point.id}</strong><br />{point.name} · {point.temperature_c} °C<br /><code>{Object.entries(point.parameters).map(([k, v]) => `${k}=${v}`).join(', ') || 'Nominal parameters'}</code></td><td>{row.status}</td><td>{Object.entries(row.measurements).map(([name, m]) => <div key={name}>{name}: {m.value === null ? m.error : `${m.value.toPrecision(6)} ${suffix[m.unit]}`}</div>)}{row.assertions?.assertions.map(a => <div key={a.code}>{a.metric}({a.signal}): {a.actual?.toPrecision(6) ?? 'unavailable'} · {a.status}{a.message && ` — ${a.message}`}</div>)}{row.errors.map((e, i) => <div key={i}>{e}</div>)}</td><td><button disabled={running || row.status === 'pending' || row.status === 'error'} onClick={() => { try {
          if (!globalThis.confirm(`Apply this case's parameters to the open source? Unsaved source will be replaced. Study temperature (${point.temperature_c} °C), independent constraints and results stay in the study, not ordinary circuit simulation.`)) return;
          onApplySource(study_effective_source(results.plan, index, resources)); dialog.current?.close();
        } catch (cause) { setError(message(cause)); } }}>Apply parameters</button></td></tr>;
      })}</tbody></table></div>
      <div className="study-row"><label>Export current study<select value={exportTarget} onChange={event => setExportTarget(event.target.value)}><option value="json">Results JSON — full data + resume</option><option value="csv">Measurement summary CSV</option><option value="data_csv">Full numeric samples CSV</option><option value="svg">Visible overlay SVG</option><option value="html">Printable HTML report</option></select></label><button disabled={running} onClick={exportFile}><Download size={14} /> Download</button></div>
      <p className="study-note">Full samples are independent of plotted samples. Files contain source and model references, not local model bodies. Native/browser solver identities differ: compare reports, but resume with the original adapter.</p>
    </>}</section>}
    <footer><p role="status">{progress || 'Local execution: one solver at a time, up to 32 browser cases. Use the CLI for larger studies.'}</p>{running ? <button onClick={cancel}><Square size={14} /> Stop study</button> : <><button disabled={!results || results.cases.every(row => reusable(row.status))} onClick={() => void runStudy(true)}>Resume</button><button className="primary" onClick={() => void runStudy(false)}><Play size={14} /> {results ? 'Run new study' : 'Run study'}</button></>}</footer>
  </dialog>;
}
