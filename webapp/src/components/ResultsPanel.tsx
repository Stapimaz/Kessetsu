import { Activity, RotateCcw, Square, Zap } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { SimulationState, SourceFeedback } from '../domain';
import type { Analysis, AssertionResult, BrowserEvaluation, Dataset } from '../simulation/types';
import { nearestSampleIndex, preferredSignal, signalLabel, signalUnit, sweepAxis, zoomAroundPointer } from '../simulation/plot';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

interface Props {
  state: SimulationState;
  hasSimulationAttempt: boolean;
  message: string;
  evaluation: BrowserEvaluation | null;
  compileSucceeded: boolean;
  sourceFeedback: SourceFeedback | null;
  simulationConfigured: boolean;
  onResolveSource(): void;
  onRun(): void;
  onCancel(): void;
  panelControls: PanelWindowControls;
}

function engineering(value: number, unit = ''): string {
  if (!Number.isFinite(value)) return '—';
  if (unit.startsWith('dB') || unit === '°') return `${Number(value.toPrecision(4))} ${unit}`;
  if (value === 0) return `0 ${unit}`.trim();
  const absolute = Math.abs(value);
  const scales = [
    [1e9, 'G'], [1e6, 'M'], [1e3, 'k'], [1, ''], [1e-3, 'm'], [1e-6, 'µ'], [1e-9, 'n'], [1e-12, 'p'],
  ] as const;
  const [factor, prefix] = scales.find(([factor]) => absolute >= factor) ?? scales.at(-1)!;
  return `${(value / factor).toPrecision(4)} ${prefix}${unit}`.trim();
}

function measuredQuantity(value: number, unit: string) {
  const units: Record<string, string> = { volt: 'V', ampere: 'A', watt: 'W', hertz: 'Hz', second: 's', degree: '°', ohm: 'Ω', dimensionless: '', ratio: '' };
  const canonical = unit.toLowerCase();
  if (canonical === 'dimensionless' || canonical === 'ratio' || canonical === '') return Number(value.toPrecision(4)).toString();
  if (canonical === 'percent') return `${value.toPrecision(4)} %`;
  return engineering(value, units[canonical] ?? unit);
}

function partStressStatus(status: string) {
  if (status === 'within_provided_limit') return 'Within';
  if (status === 'exceeds_provided_limit') return 'Exceeds';
  return 'Unavailable';
}

function partRatingLabel(rating: string) {
  return rating.split('_').map((word) => word[0].toUpperCase() + word.slice(1)).join(' ');
}

interface PlotProps {
  axis: number[];
  values: number[];
  axisName: string;
  axisUnit: string;
  valueUnit: string;
  logX?: boolean;
}

const PLOT_LEFT = 42;
const PLOT_RIGHT = 578;
const PLOT_WIDTH = PLOT_RIGHT - PLOT_LEFT;

function pointerPositionInPlot(svg: SVGSVGElement, clientX: number, clientY: number) {
  const matrix = svg.getScreenCTM();
  if (!matrix) return null;
  const point = svg.createSVGPoint();
  point.x = clientX;
  point.y = clientY;
  const local = point.matrixTransform(matrix.inverse());
  return Math.max(0, Math.min(1, (local.x - PLOT_LEFT) / PLOT_WIDTH));
}

function LinePlot({ axis, values, axisName, axisUnit, valueUnit, logX = false }: PlotProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState<[number, number]>([0, 1]);
  const [cursor, setCursor] = useState<number | null>(null);
  useEffect(() => { setZoom([0, 1]); setCursor(null); }, [axis, values]);
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const fraction = pointerPositionInPlot(svg, event.clientX, event.clientY) ?? 0.5;
      setZoom((current) => zoomAroundPointer(current, fraction, event.deltaY > 0));
    };
    svg.addEventListener('wheel', wheel, { passive: false });
    return () => svg.removeEventListener('wheel', wheel);
  }, []);
  const coordinates = useMemo(() => logX ? axis.map((value) => Math.log10(value)) : axis, [axis, logX]);
  const first = coordinates[0] ?? 0;
  const last = coordinates.at(-1) ?? 1;
  const domainMin = Math.min(first, last);
  const domainMax = Math.max(first, last);
  const minX = domainMin + zoom[0] * (domainMax - domainMin);
  const maxX = domainMin + zoom[1] * (domainMax - domainMin);
  const visible = useMemo(() => coordinates.flatMap((coordinate, index) =>
    coordinate >= minX && coordinate <= maxX ? [index] : []), [coordinates, minX, maxX]);
  const xValues = useMemo(() => visible.map((index) => coordinates[index]), [coordinates, visible]);
  const [minY, maxY] = useMemo(() => {
    let low = Infinity;
    let high = -Infinity;
    for (const index of visible) { low = Math.min(low, values[index]); high = Math.max(high, values[index]); }
    return visible.length ? [low, high] : [0, 0];
  }, [values, visible]);
  const x = useCallback((value: number) => PLOT_LEFT + ((value - minX) / (maxX - minX || 1)) * PLOT_WIDTH, [minX, maxX]);
  const y = useCallback((value: number) => 174 - ((value - minY) / (maxY - minY || 1)) * 146, [minY, maxY]);
  const points = useMemo(() => xValues.map((value, index) => `${x(value)},${y(values[visible[index]])}`).join(' '), [xValues, values, visible, x, y]);
  const localIndex = cursor === null ? null : nearestSampleIndex(xValues, minX + cursor * (maxX - minX));
  const cursorIndex = localIndex === null ? null : visible[localIndex];
  const cursorX = cursorIndex === null ? null : x(coordinates[cursorIndex]);
  const physicalX = (value: number) => logX ? 10 ** value : value;

  return (
    <div className="plot-wrap">
      <svg
        ref={svgRef}
        className="result-plot"
        viewBox="0 0 620 205"
        role="img"
        aria-label={`${axisName} plot`}
        onPointerMove={(event) => {
          setCursor(pointerPositionInPlot(
            event.currentTarget,
            event.clientX,
            event.clientY,
          ));
        }}
        onPointerLeave={() => setCursor(null)}
      >
        <rect x={PLOT_LEFT} y="18" width={PLOT_WIDTH} height="156" className="plot-bg" />
        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => <line key={ratio} x1={PLOT_LEFT} x2={PLOT_RIGHT} y1={28 + ratio * 146} y2={28 + ratio * 146} className="grid-line" />)}
        <polyline points={points} className="signal-line" />
        {!visible.length && <text x="310" y="96" textAnchor="middle" className="axis-label">No recorded samples in this zoom range</text>}
        {cursorIndex !== null && (
          <>
            <line x1={cursorX!} x2={cursorX!} y1="18" y2="174" className="cursor-line" />
            <circle cx={cursorX!} cy={y(values[cursorIndex])} r="4" className="cursor-dot" />
          </>
        )}
        <text x={PLOT_LEFT} y="195" className="axis-label">{engineering(physicalX(minX), axisUnit)}</text>
        <text x={PLOT_RIGHT} y="195" textAnchor="end" className="axis-label">{engineering(physicalX(maxX), axisUnit)}</text>
        <text x="48" y="14" className="axis-label">{engineering(maxY, valueUnit)}</text>
      </svg>
      {cursorIndex !== null && (
        <output className="cursor-readout" data-axis-value={axis[cursorIndex]} data-signal-value={values[cursorIndex]}>
          {engineering(axis[cursorIndex], axisUnit)} · {engineering(values[cursorIndex], valueUnit)}
        </output>
      )}
      {zoom[1] - zoom[0] < 0.999 && <button className="plot-reset" onClick={() => setZoom([0, 1])}><RotateCcw size={12} /> Reset zoom</button>}
    </div>
  );
}

function DatasetView({ dataset, analysis, assertions }: { dataset: Dataset; analysis: Analysis; assertions: AssertionResult[] }) {
  const signalNames = dataset.kind === 'operating_point' ? Object.keys(dataset.values) : Object.keys(dataset.signals);
  const preferred = preferredSignal(signalNames, assertions);
  const [signal, setSignal] = useState(preferred);
  useEffect(() => {
    setSignal(preferred);
  }, [dataset, preferred]);
  const selectedSignal = signalNames.includes(signal) ? signal : preferred;
  const ac = useMemo(() => {
    if (dataset.kind !== 'ac') return null;
    const series = dataset.signals[selectedSignal];
    if (!series) return null;
    return {
      magnitude: series.real.map((real, index) => 20 * Math.log10(Math.max(Math.hypot(real, series.imaginary[index]), Number.MIN_VALUE))),
      phase: series.real.map((real, index) => Math.atan2(series.imaginary[index], real) * 180 / Math.PI),
    };
  }, [dataset, selectedSignal]);

  if (dataset.kind === 'operating_point') {
    return <div className="op-grid">{Object.entries(dataset.values).map(([name, value]) => (
      <div key={name}><span>{signalLabel(name)}</span><strong>{engineering(value, signalUnit(name))}</strong></div>
    ))}</div>;
  }

  if (signalNames.length === 0) {
    return <div className="result-empty">No plottable signals in this analysis.</div>;
  }
  const signalControl = (
    <label className="signal-picker">Signal
      <select aria-label="Signal" value={selectedSignal} onChange={(event) => setSignal(event.target.value)}>
        {signalNames.map((name) => <option key={name} value={name}>{signalLabel(name)}</option>)}
      </select>
    </label>
  );

  if (dataset.kind === 'ac') {
    if (!ac) return null;
    const reference = signalUnit(selectedSignal) === 'A' ? '1 A' : '1 V';
    const magnitudeUnit = signalUnit(selectedSignal) === 'A' ? 'dB re 1 A' : 'dBV';
    return <>{signalControl}<p className="plot-explanation">Magnitude is 20 log₁₀(|{signalLabel(selectedSignal)}| / {reference}), not output/input gain. Phase is the angle of this signal.</p><div className="bode-grid">
      <div><span className="plot-title">Magnitude (re {reference})</span><LinePlot axis={dataset.frequency_hz} values={ac.magnitude} axisName="Frequency" axisUnit="Hz" valueUnit={magnitudeUnit} logX /></div>
      <div><span className="plot-title">Phase</span><LinePlot axis={dataset.frequency_hz} values={ac.phase} axisName="Frequency" axisUnit="Hz" valueUnit="°" logX /></div>
    </div></>;
  }

  const values = dataset.signals[selectedSignal];
  if (!values) return null;
  const horizontal = sweepAxis(analysis);
  return <>{signalControl}<span className="plot-title">{horizontal.label} ({horizontal.unit})</span><LinePlot
    axis={dataset.axis.values}
    values={values}
    axisName={horizontal.label}
    axisUnit={horizontal.unit}
    valueUnit={signalUnit(selectedSignal)}
  /></>;
}

export function ResultsPanel({ state, hasSimulationAttempt, message, evaluation, compileSucceeded, sourceFeedback, simulationConfigured, onResolveSource, onRun, onCancel, panelControls }: Props) {
  const [datasetIndex, setDatasetIndex] = useState(0);
  const datasets = evaluation?.simulation.datasets ?? [];
  const selected = datasets[datasetIndex] ?? datasets[0];
  useEffect(() => setDatasetIndex(0), [evaluation]);
  const summary = evaluation?.assertions.summary;
  const showFirstRunGuide = !sourceFeedback && simulationConfigured && state === 'idle'
    && (message.startsWith('Run the simulation') || message.startsWith('This starter circuit'));
  const needsAnalysis = !sourceFeedback && !simulationConfigured;
  const rerun = hasSimulationAttempt && (state === 'failed' || state === 'cancelled' || message.startsWith('Simulation results are out of date') || message.startsWith('Model bindings changed'));
  const displayedMessage = sourceFeedback?.message ?? (needsAnalysis
    ? 'Your circuit is valid and can be exported. Add an analysis in Source to calculate its behavior.'
    : message);
  const statusText = useMemo(() => {
    if (sourceFeedback) return sourceFeedback.title;
    if (!simulationConfigured) return 'No analysis configured';
    if (summary && summary.total > 0) return `${summary.passed}/${summary.total} requirements passed · ${evaluation?.simulation.simulator.version}`;
    if (state === 'succeeded') return `Simulation complete · ${evaluation?.simulation.simulator.version}`;
    if (state === 'running') return message;
    if (state === 'failed') return 'Failed';
    if (state === 'cancelled') return 'Cancelled';
    if (message.startsWith('Simulation results are out of date')) return 'Results out of date';
    if (message.startsWith('Model bindings changed')) return 'Results out of date';
    if (message.startsWith('Add a simulation command')) return 'Not configured';
    return 'Ready';
  }, [evaluation?.simulation.simulator.version, message, state, summary, sourceFeedback, simulationConfigured]);

  return (
    <section className="workspace-panel results-panel" aria-label="Circuit simulation" data-testid="simulation-summary" data-state={state}>
      <PanelHeader controls={panelControls} icon={<Activity size={15} />} title="Simulation">
        <div className="simulation-toolbar">
          <span className={`run-status status-${state}`} role="status">{statusText}</span>
          {state === 'running'
            ? <button className="simulation-run-button cancel-button" aria-label="Cancel simulation" onClick={onCancel}><Square size={12} /><span>Cancel</span></button>
            : <button className="simulation-run-button run-button" aria-label="Run simulation" title={sourceFeedback?.message ?? (needsAnalysis ? 'Add a simulation analysis in Source first.' : 'Simulate the current source and evaluate its assertions.')} onClick={onRun} disabled={!compileSucceeded || !simulationConfigured}><Zap size={13} /><span>{rerun ? 'Run again' : 'Run'}</span></button>}
        </div>
      </PanelHeader>
      <div className="results-body">
        {datasets.length > 0 ? (
          <>
            <div className="analysis-tabs" role="tablist" aria-label="Analysis results">
              {datasets.map((dataset, index) => (
                <button
                  key={dataset.index}
                  role="tab"
                  aria-selected={datasetIndex === index}
                  onClick={() => setDatasetIndex(index)}
                  data-testid="dataset-kind"
                >{dataset.data.kind.replace('_', ' ')}</button>
              ))}
            </div>
            {selected && <DatasetView key={selected.index} dataset={selected.data} analysis={selected.analysis} assertions={evaluation?.assertions.assertions ?? []} />}
          </>
        ) : <div className={`result-empty result-${state}`}>
          <Activity size={28} className={state === 'running' ? 'simulation-running-icon' : undefined} />
          {sourceFeedback && <strong>{sourceFeedback.title}</strong>}
          {needsAnalysis && <strong>Add an analysis to simulate</strong>}
          {showFirstRunGuide && <strong>Ready for your first run</strong>}
          <p>{displayedMessage}</p>
          {sourceFeedback?.actionLabel && <button className="secondary-button" onClick={onResolveSource}>{sourceFeedback.actionLabel}</button>}
          {needsAnalysis && <div className="simulation-help">
            <p>Start with <code>simulate op</code> on a new line for DC voltages and currents.</p>
            <a href={`${import.meta.env.BASE_URL}docs/reference/simulation-and-assertions/`} target="_blank" rel="noreferrer">Choose an analysis: operating point, AC, transient or DC sweep</a>
          </div>}
          {!sourceFeedback && !needsAnalysis && state === 'failed' && <div className="simulation-help">
            <p>No results were accepted. Check the error above and your source, then Run again.</p>
            <a href={`${import.meta.env.BASE_URL}docs/guides/troubleshooting/`} target="_blank" rel="noreferrer">Simulation troubleshooting</a>
          </div>}
          {!sourceFeedback && !needsAnalysis && state === 'cancelled' && <p>Your source is unchanged. Run again when you are ready.</p>}
          {showFirstRunGuide && <div className="result-start-guide" aria-label="First simulation steps">
            <span><b>1</b> Review Source</span><span><b>2</b> Press Run above</span><span><b>3</b> Inspect plots and assertions</span><span><b>4</b> Export from the top bar</span>
            <a href={`${import.meta.env.BASE_URL}docs/guides/tutorial/`} target="_blank" rel="noreferrer">Open the first-circuit tutorial</a>
          </div>}
        </div>}
        {state === 'succeeded' && summary?.total === 0 && <p className="simulation-help">Simulation completed without requirement checks. Add <code>assert</code> statements in Source to test measurable limits. <a href={`${import.meta.env.BASE_URL}docs/reference/simulation-and-assertions/`} target="_blank" rel="noreferrer">Learn about assertions</a></p>}
        {!!evaluation?.assertions.assertions.length && <div className="requirements-wrap">
          <p className="plot-explanation">Requirement measurements and limits are listed below, not drawn as instantaneous waveform limits.</p>
          <table className="requirements-table" aria-label="Engineering requirements">
            <thead><tr><th>Status</th><th>Requirement</th><th>Measured</th><th>Limit</th></tr></thead>
            <tbody>{evaluation.assertions.assertions.map((assertion) => <tr key={assertion.code}>
              <td><span className={`assertion-${assertion.status.toLowerCase()}`}>{assertion.status}</span></td>
              <td><span hidden data-assertion-code={assertion.code} data-actual={assertion.actual ?? ''} />
                <span className="requirement-name">{assertion.metric}({assertion.signal})</span>
                {assertion.message && assertion.status !== 'PASS' && <small>{assertion.message}</small>}
              </td>
              <td>{assertion.actual == null ? '—' : measuredQuantity(assertion.actual, assertion.unit)}</td>
              <td>{assertion.comparator} {measuredQuantity(assertion.threshold, assertion.unit)}</td>
            </tr>)}</tbody>
          </table>
        </div>}
        {!!evaluation?.part_stress.results.length && <div className="requirements-wrap part-stress-wrap">
          <div className="part-stress-heading">
            <div><strong>Provided part limits</strong><span>Simulated model stress, kept separate from design requirements.</span></div>
            <small>{evaluation.part_stress.disclaimer}</small>
          </div>
          <table className="requirements-table part-stress-table" aria-label="Provided part limit comparison">
            <thead><tr><th>State</th><th>Part / measurement</th><th>Simulated</th><th>Provided limit</th><th>Recorded conditions</th></tr></thead>
            <tbody>{evaluation.part_stress.results.map((result) => <tr key={`${result.component}-${result.rating}`}>
              <td><span className={`part-stress-${result.status}`}>{partStressStatus(result.status)}</span></td>
              <td><span className="requirement-name">{result.component} · {partRatingLabel(result.rating)}</span>
                <small>{result.metric}({result.signal}){result.analysis ? ` · ${result.analysis}` : ''}</small>
              </td>
              <td>{result.actual == null ? '—' : measuredQuantity(result.actual, result.limit.unit)}
                {result.utilization_percent != null && <small>{result.utilization_percent.toPrecision(4)}% of limit</small>}
                {result.message && <small>{result.message}</small>}
              </td>
              <td>{measuredQuantity(result.limit.value, result.limit.unit)}</td>
              <td>{result.conditions}{result.source && <small>Source: {result.source}</small>}</td>
            </tr>)}</tbody>
          </table>
        </div>}
      </div>
    </section>
  );
}
