import { Activity, RotateCcw } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type { SimulationState } from '../domain';
import type { AssertionResult, BrowserEvaluation, Dataset } from '../simulation/types';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

interface Props {
  state: SimulationState;
  message: string;
  evaluation: BrowserEvaluation | null;
  panelControls: PanelWindowControls;
}

function engineering(value: number, unit = ''): string {
  if (!Number.isFinite(value)) return '—';
  if (value === 0) return `0 ${unit}`.trim();
  const absolute = Math.abs(value);
  const scales = [
    [1e9, 'G'], [1e6, 'M'], [1e3, 'k'], [1, ''], [1e-3, 'm'], [1e-6, 'µ'], [1e-9, 'n'], [1e-12, 'p'],
  ] as const;
  const [factor, prefix] = scales.find(([factor]) => absolute >= factor) ?? scales.at(-1)!;
  return `${(value / factor).toPrecision(4)} ${prefix}${unit}`.trim();
}

function signalLabel(name: string) {
  return name.includes('#branch') ? `${name.replace('#branch', '')} current` : `V(${name})`;
}

function measuredQuantity(value: number, unit: string) {
  const units: Record<string, string> = { volt: 'V', ampere: 'A', watt: 'W', hertz: 'Hz', second: 's', degree: '°', ohm: 'Ω', dimensionless: '', ratio: '' };
  const canonical = unit.toLowerCase();
  if (canonical === 'dimensionless' || canonical === 'ratio' || canonical === '') return Number(value.toPrecision(4)).toString();
  if (canonical === 'percent') return `${value.toPrecision(4)} %`;
  return engineering(value, units[canonical] ?? unit);
}

function axisUnit(name: string) {
  if (name.includes('time')) return 's';
  if (name.includes('frequency')) return 'Hz';
  if (name.includes('sweep')) return '';
  return '';
}

interface PlotProps {
  axis: number[];
  values: number[];
  axisName: string;
  valueUnit: string;
  thresholds?: number[];
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

function LinePlot({ axis, values, axisName, valueUnit, thresholds = [], logX = false }: PlotProps) {
  const [zoom, setZoom] = useState<[number, number]>([0, 1]);
  const [cursor, setCursor] = useState<number | null>(null);
  const start = Math.floor(zoom[0] * Math.max(0, axis.length - 1));
  const stop = Math.max(start + 1, Math.ceil(zoom[1] * axis.length));
  const visibleAxis = axis.slice(start, stop);
  const visibleValues = values.slice(start, stop);
  const xValues = logX ? visibleAxis.map((value) => Math.log10(Math.max(value, Number.MIN_VALUE))) : visibleAxis;
  const minX = Math.min(...xValues);
  const maxX = Math.max(...xValues);
  const allY = [...visibleValues, ...thresholds];
  const minY = Math.min(...allY);
  const maxY = Math.max(...allY);
  const x = (value: number) => PLOT_LEFT + ((value - minX) / (maxX - minX || 1)) * PLOT_WIDTH;
  const y = (value: number) => 174 - ((value - minY) / (maxY - minY || 1)) * 146;
  const points = xValues.map((value, index) => `${x(value)},${y(visibleValues[index])}`).join(' ');
  const cursorIndex = cursor === null ? null : Math.min(axis.length - 1, Math.max(0, Math.round(start + cursor * (stop - start - 1))));

  return (
    <div className="plot-wrap">
      <svg
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
        onWheel={(event) => {
          event.preventDefault();
          const center = (zoom[0] + zoom[1]) / 2;
          const width = Math.max(0.05, Math.min(1, (zoom[1] - zoom[0]) * (event.deltaY > 0 ? 1.25 : 0.8)));
          setZoom([Math.max(0, center - width / 2), Math.min(1, center + width / 2)]);
        }}
      >
        <rect x={PLOT_LEFT} y="18" width={PLOT_WIDTH} height="156" className="plot-bg" />
        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => <line key={ratio} x1={PLOT_LEFT} x2={PLOT_RIGHT} y1={28 + ratio * 146} y2={28 + ratio * 146} className="grid-line" />)}
        {thresholds.map((threshold) => <line key={threshold} x1={PLOT_LEFT} x2={PLOT_RIGHT} y1={y(threshold)} y2={y(threshold)} className="threshold-line" />)}
        <polyline points={points} className="signal-line" />
        {cursorIndex !== null && (
          <>
            <line x1={PLOT_LEFT + (cursor ?? 0) * PLOT_WIDTH} x2={PLOT_LEFT + (cursor ?? 0) * PLOT_WIDTH} y1="18" y2="174" className="cursor-line" />
            <circle cx={x(logX ? Math.log10(Math.max(axis[cursorIndex], Number.MIN_VALUE)) : axis[cursorIndex])} cy={y(values[cursorIndex])} r="4" className="cursor-dot" />
          </>
        )}
        <text x={PLOT_LEFT} y="195" className="axis-label">{engineering(visibleAxis[0], axisUnit(axisName))}</text>
        <text x={PLOT_RIGHT} y="195" textAnchor="end" className="axis-label">{engineering(visibleAxis.at(-1) ?? 0, axisUnit(axisName))}</text>
        <text x="48" y="14" className="axis-label">{engineering(maxY, valueUnit)}</text>
      </svg>
      {cursorIndex !== null && (
        <output className="cursor-readout">
          {engineering(axis[cursorIndex], axisUnit(axisName))} · {engineering(values[cursorIndex], valueUnit)}
        </output>
      )}
      {zoom[1] - zoom[0] < 0.999 && <button className="plot-reset" onClick={() => setZoom([0, 1])}><RotateCcw size={12} /> Reset zoom</button>}
    </div>
  );
}

function signalThresholds(assertions: AssertionResult[], signal: string) {
  const canonical = signal.replace('#branch', '').toLowerCase();
  return assertions
    .filter((assertion) => ['value', 'min', 'max', 'peak', 'average', 'avg', 'rms'].includes(assertion.metric))
    .filter((assertion) => assertion.signal.toLowerCase() === `v(${canonical})` || assertion.signal.toLowerCase() === `i(${canonical})`)
    .map((assertion) => assertion.threshold);
}

function DatasetView({ dataset, assertions }: { dataset: Dataset; assertions: AssertionResult[] }) {
  const signalNames = dataset.kind === 'operating_point' ? Object.keys(dataset.values) : Object.keys(dataset.signals);
  const preferredSignal = signalNames.find((name) => name === 'out') ?? signalNames[0] ?? '';
  const [signal, setSignal] = useState(preferredSignal);
  useEffect(() => {
    const names = dataset.kind === 'operating_point' ? Object.keys(dataset.values) : Object.keys(dataset.signals);
    setSignal(names.find((name) => name === 'out') ?? names[0] ?? '');
  }, [dataset]);

  if (dataset.kind === 'operating_point') {
    return <div className="op-grid">{Object.entries(dataset.values).map(([name, value]) => (
      <div key={name}><span>{signalLabel(name)}</span><strong>{engineering(value, name.includes('#branch') ? 'A' : 'V')}</strong></div>
    ))}</div>;
  }

  if (signalNames.length === 0) {
    return <div className="result-empty">No plottable signals in this analysis.</div>;
  }
  const selectedSignal = signalNames.includes(signal) ? signal : signalNames[0];

  const signalControl = (
    <label className="signal-picker">Signal
      <select aria-label="Signal" value={selectedSignal} onChange={(event) => setSignal(event.target.value)}>
        {signalNames.map((name) => <option key={name} value={name}>{signalLabel(name)}</option>)}
      </select>
    </label>
  );

  if (dataset.kind === 'ac') {
    const series = dataset.signals[selectedSignal];
    if (!series) return null;
    const magnitude = series.real.map((real, index) => 20 * Math.log10(Math.max(Math.hypot(real, series.imaginary[index]), Number.MIN_VALUE)));
    const phase = series.real.map((real, index) => Math.atan2(series.imaginary[index], real) * 180 / Math.PI);
    return <>{signalControl}<div className="bode-grid">
      <div><span className="plot-title">Magnitude</span><LinePlot axis={dataset.frequency_hz} values={magnitude} axisName="frequency" valueUnit="dB" logX /></div>
      <div><span className="plot-title">Phase</span><LinePlot axis={dataset.frequency_hz} values={phase} axisName="frequency" valueUnit="deg" logX /></div>
    </div></>;
  }

  const values = dataset.signals[selectedSignal];
  if (!values) return null;
  return <>{signalControl}<LinePlot
    axis={dataset.axis.values}
    values={values}
    axisName={dataset.axis.name}
    valueUnit={selectedSignal.includes('#branch') ? 'A' : 'V'}
    thresholds={signalThresholds(assertions, selectedSignal)}
  /></>;
}

export function ResultsPanel({ state, message, evaluation, panelControls }: Props) {
  const [datasetIndex, setDatasetIndex] = useState(0);
  const datasets = evaluation?.simulation.datasets ?? [];
  const selected = datasets[datasetIndex] ?? datasets[0];
  useEffect(() => setDatasetIndex(0), [evaluation]);
  const summary = evaluation?.assertions.summary;
  const statusText = useMemo(() => {
    if (!summary) return message;
    return `${summary.passed}/${summary.total} requirements passed · ${evaluation?.simulation.simulator.version}`;
  }, [evaluation?.simulation.simulator.version, message, summary]);

  return (
    <section className="workspace-panel results-panel" aria-label="Simulation results" data-testid="simulation-summary" data-state={state}>
      <PanelHeader controls={panelControls} icon={<Activity size={15} />} title="Results">
        <span className={`run-status status-${state}`}>{statusText}</span>
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
            {selected && <DatasetView dataset={selected.data} assertions={evaluation?.assertions.assertions ?? []} />}
          </>
        ) : <div className={`result-empty result-${state}`}><Activity size={28} /><p>{message}</p></div>}
        {!!evaluation?.assertions.assertions.length && <div className="requirements-wrap">
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
      </div>
    </section>
  );
}
