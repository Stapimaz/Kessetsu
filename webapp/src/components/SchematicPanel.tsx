import { Activity, Boxes, CircuitBoard, Code2, Focus, Grid3X3, Minus, Plus, Search, SlidersHorizontal, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { CircuitIrSummary, SchematicComponent, SchematicSummary, SourceFeedback } from '../domain';
import {
  augmentDatasetWithDerivedSignals,
  componentKindName,
  componentVoltageSignalKey,
  findComponentCurrentSignalKey,
  findComponentPinNets,
  findLiteralComponentValueSpan,
  formatKessQuantity,
  parseKessQuantity,
  summarizeAcSeries,
  summarizeWaveformSeries,
} from '../simulation/plot';
import type { BrowserEvaluation } from '../simulation/types';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

interface Props {
  schematic: SchematicSummary | null;
  circuitIr: CircuitIrSummary | null;
  svg: string;
  code?: string;
  documentRevision?: number;
  onTweakCode?(nextCode: string): void;
  onPlotSignal?(signal: string): void;
  onCompareSignal?(signal: string): void;
  panelControls: PanelWindowControls;
  sourceFeedback: SourceFeedback | null;
  onResolveSource(): void;
  evaluation?: BrowserEvaluation | null;
  onRevealSymbol?(symbol: string): void;
}

function formatEngineering(value: number, unit = ''): string {
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

type Selection =
  | { type: 'component'; id: string }
  | { type: 'net'; id: number }
  | { type: 'group'; path: string[] };

function componentAtTarget(target: EventTarget | null) {
  return target instanceof Element
    ? target.closest('[data-component]')?.getAttribute('data-component') ?? null
    : null;
}

function netAtTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) return null;
  const value = target.closest('[data-net]')?.getAttribute('data-net');
  if (value === null || value === undefined || !/^\d+$/.test(value)) return null;
  return Number(value);
}

function pointToSegmentDistance(
  px: number,
  py: number,
  ax: number,
  ay: number,
  bx: number,
  by: number,
): number {
  const dx = bx - ax;
  const dy = by - ay;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) return Math.hypot(px - ax, py - ay);
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lenSq));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

function parseSvgPoints(raw: string | null): Array<[number, number]> {
  if (!raw) return [];
  const nums = raw.trim().split(/[\s,]+/).map(Number).filter((v) => Number.isFinite(v));
  const points: Array<[number, number]> = [];
  for (let i = 0; i + 1 < nums.length; i += 2) {
    points.push([nums[i], nums[i + 1]]);
  }
  return points;
}

function netAtPointer(
  target: EventTarget | null,
  surface: HTMLDivElement,
  clientX: number,
  clientY: number,
): number | null {
  const direct = netAtTarget(target);
  if (direct !== null) return direct;

  let bestNet: number | null = null;
  let bestDistance = 8;

  for (const wire of surface.querySelectorAll<SVGPolylineElement>('polyline[data-net]')) {
    const attr = wire.getAttribute('data-net');
    if (!attr || !/^\d+$/.test(attr)) continue;
    const netId = Number(attr);
    const ctm = typeof wire.getScreenCTM === 'function' ? wire.getScreenCTM() : null;
    const pts = parseSvgPoints(wire.getAttribute('points'));
    if (ctm && pts.length >= 2) {
      const screenPts = pts.map(([x, y]) => [
        ctm.a * x + ctm.c * y + ctm.e,
        ctm.b * x + ctm.d * y + ctm.f,
      ] as const);
      for (let i = 0; i + 1 < screenPts.length; i += 1) {
        const dist = pointToSegmentDistance(
          clientX,
          clientY,
          screenPts[i][0],
          screenPts[i][1],
          screenPts[i + 1][0],
          screenPts[i + 1][1],
        );
        if (dist <= bestDistance) {
          bestDistance = dist;
          bestNet = netId;
        }
      }
      continue;
    }
    const bounds = wire.getBoundingClientRect();
    const dx = Math.max(bounds.left - clientX, 0, clientX - bounds.right);
    const dy = Math.max(bounds.top - clientY, 0, clientY - bounds.bottom);
    const dist = Math.hypot(dx, dy);
    if (dist <= bestDistance) {
      bestDistance = dist;
      bestNet = netId;
    }
  }

  for (const elem of surface.querySelectorAll<Element>('[data-net]:not(polyline)')) {
    const attr = elem.getAttribute('data-net');
    if (!attr || !/^\d+$/.test(attr)) continue;
    const bounds = elem.getBoundingClientRect();
    const dx = Math.max(bounds.left - clientX, 0, clientX - bounds.right);
    const dy = Math.max(bounds.top - clientY, 0, clientY - bounds.bottom);
    const dist = Math.hypot(dx, dy);
    if (dist <= 6 && dist <= bestDistance) {
      bestDistance = dist;
      bestNet = Number(attr);
    }
  }

  return bestNet;
}

function componentAtPointer(
  target: EventTarget | null,
  surface: HTMLDivElement,
  clientX: number,
  clientY: number,
) {
  const direct = componentAtTarget(target);
  if (direct) return direct;

  const padding = 6;
  const matches = Array.from(
    surface.querySelectorAll<SVGGElement>('g.component[data-component]'),
  ).flatMap((group) => {
    const bounds = group.getBoundingClientRect();
    if (
      clientX < bounds.left - padding
      || clientX > bounds.right + padding
      || clientY < bounds.top - padding
      || clientY > bounds.bottom + padding
    ) return [];
    return [{
      component: group.getAttribute('data-component') ?? '',
      area: bounds.width * bounds.height,
    }];
  });
  matches.sort((left, right) => left.area - right.area || left.component.localeCompare(right.component));
  return matches[0]?.component || null;
}

function pathKey(path: string[]) {
  return path.join('\u001f');
}

function pathStartsWith(path: string[], prefix: string[]) {
  return prefix.every((part, index) => path[index] === part);
}

function componentKind(kind: string | Record<string, unknown> | undefined) {
  if (!kind) return 'Component';
  if (typeof kind === 'string') return kind.replaceAll('_', ' ');
  return Object.keys(kind)[0]?.replaceAll('_', ' ') ?? 'Component';
}

export function SchematicPanel({
  schematic,
  circuitIr,
  svg,
  code = '',
  documentRevision = 0,
  onTweakCode,
  onPlotSignal,
  onCompareSignal,
  panelControls,
  sourceFeedback,
  onResolveSource,
  evaluation = null,
  onRevealSymbol,
}: Props) {
  const [view, setView] = useState({ zoom: 1, pan: { x: 0, y: 0 } });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [gridVisible, setGridVisible] = useState(true);
  const [hoveredComponent, setHoveredComponent] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [navigatorOpen, setNavigatorOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [tweakDraft, setTweakDraft] = useState('');
  const documentRef = useRef<HTMLDivElement>(null);
  const { zoom, pan } = view;
  const activeComponent = hoveredComponent ?? (selection?.type === 'component' ? selection.id : null);
  const reset = () => setView({ zoom: 1, pan: { x: 0, y: 0 } });

  const irComponents = useMemo(() => new Map(
    (circuitIr?.components ?? []).map((component) => [component.id, component]),
  ), [circuitIr]);
  const components = useMemo(() => schematic?.components ?? [], [schematic]);
  const nets = useMemo(() => schematic?.nets ?? [], [schematic]);
  const groups = useMemo(() => {
    const unique = new Map<string, string[]>();
    for (const component of circuitIr?.components ?? []) {
      const path = component.instance_path ?? [];
      for (let depth = 1; depth <= path.length; depth += 1) {
        const group = path.slice(0, depth);
        unique.set(pathKey(group), group);
      }
    }
    return [...unique.values()].sort((left, right) => pathKey(left).localeCompare(pathKey(right)));
  }, [circuitIr]);

  const selectedComponent = selection?.type === 'component'
    ? components.find((component) => component.id === selection.id) ?? null
    : null;
  const selectedNet = selection?.type === 'net'
    ? nets.find((net) => net.id === selection.id) ?? null
    : null;

  const netSimulationSummary = useMemo(() => {
    if (!selectedNet || !evaluation) return null;
    const key = selectedNet.name.toLocaleLowerCase('en-US');
    const items: Array<{ label: string; value: string }> = [];
    const opDataset = evaluation.simulation.datasets.find((d) => d.data.kind === 'operating_point')?.data;
    if (selectedNet.kind === 'ground' || key === 'gnd' || key === '0') {
      items.push({ label: 'DC Operating Point', value: '0 V (ground)' });
    } else if (opDataset?.kind === 'operating_point' && key in opDataset.values) {
      items.push({ label: 'DC Operating Point', value: formatEngineering(opDataset.values[key], 'V') });
    }
    const tranDataset = evaluation.simulation.datasets.find((d) => d.data.kind === 'transient')?.data;
    if (tranDataset?.kind === 'transient' && key in tranDataset.signals) {
      const summary = summarizeWaveformSeries(tranDataset.axis.values, tranDataset.signals[key]);
      if (summary) {
        items.push({ label: 'Transient Vpp', value: formatEngineering(summary.peakToPeak, 'V') });
        items.push({ label: 'Transient RMS', value: formatEngineering(summary.rms, 'V') });
      }
    }
    const acDataset = evaluation.simulation.datasets.find((d) => d.data.kind === 'ac')?.data;
    if (acDataset?.kind === 'ac' && key in acDataset.signals) {
      const series = acDataset.signals[key];
      const mag = series.real.map((r, i) => 20 * Math.log10(Math.max(Math.hypot(r, series.imaginary[i]), Number.MIN_VALUE)));
      const phase = series.real.map((r, i) => Math.atan2(series.imaginary[i], r) * 180 / Math.PI);
      const summary = summarizeAcSeries(acDataset.frequency_hz, mag, phase);
      if (summary) {
        items.push({ label: 'AC Peak', value: `${formatEngineering(summary.peakMagnitudeDb, 'dBV')} @ ${formatEngineering(summary.peakFrequencyHz, 'Hz')}` });
        if (summary.cutoffFrequencyHz !== null) {
          items.push({ label: 'AC -3 dB Cutoff', value: formatEngineering(summary.cutoffFrequencyHz, 'Hz') });
        }
      }
    }
    return items.length > 0 ? items : null;
  }, [evaluation, selectedNet]);

  const componentSimulationSummary = useMemo(() => {
    if (!selectedComponent || !evaluation) return null;
    const metrics: Array<{ label: string; value: string }> = [];
    const irComp = irComponents.get(selectedComponent.id);
    const kindName = componentKindName(irComp?.kind ?? '');
    const pins = findComponentPinNets(selectedComponent.id, kindName, schematic, circuitIr);

    const opRaw = evaluation.simulation.datasets.find((d) => d.data.kind === 'operating_point')?.data;
    const tranRaw = evaluation.simulation.datasets.find((d) => d.data.kind === 'transient')?.data;

    if (opRaw?.kind === 'operating_point') {
      const augOp = augmentDatasetWithDerivedSignals(opRaw, circuitIr, schematic);
      if (augOp.kind === 'operating_point') {
        const opKeys = Object.keys(augOp.values);
        const readNetV = (netName: string): number | null => {
          const lower = netName.toLowerCase();
          if (lower === '0' || lower === 'gnd') return 0;
          const hit = opKeys.find((k) => k.toLowerCase() === lower);
          return hit !== undefined ? augOp.values[hit] : null;
        };
        if (pins) {
          const va = readNetV(pins.posNet);
          const vb = readNetV(pins.negNet);
          if (va !== null && vb !== null) {
            metrics.push({ label: 'ΔV', value: formatEngineering(va - vb, 'V') });
          }
        }
        const currKey = findComponentCurrentSignalKey(selectedComponent.id, kindName, opKeys);
        if (currKey) {
          metrics.push({ label: 'I', value: formatEngineering(augOp.values[currKey], 'A') });
        }
        const powerKey = opKeys.find((k) => k.toLowerCase() === `p(${selectedComponent.id.toLowerCase()})`);
        if (powerKey) {
          metrics.push({ label: 'P', value: formatEngineering(augOp.values[powerKey], 'W') });
        }
        for (const pin of selectedComponent.pins) {
          const net = nets.find((candidate) => candidate.id === pin.net);
          if (!net) continue;
          const key = net.name.toLocaleLowerCase('en-US');
          if (net.kind === 'ground' || key === 'gnd' || key === '0') {
            metrics.push({ label: `V(${pin.name})`, value: '0 V' });
          } else {
            const v = readNetV(net.name);
            if (v !== null) metrics.push({ label: `V(${pin.name})`, value: formatEngineering(v, 'V') });
          }
        }
      }
    } else if (tranRaw?.kind === 'transient') {
      const augTran = augmentDatasetWithDerivedSignals(tranRaw, circuitIr, schematic);
      if (augTran.kind === 'transient') {
        const tranKeys = Object.keys(augTran.signals);
        const readNetSeries = (netName: string): number[] | null => {
          const lower = netName.toLowerCase();
          if (lower === '0' || lower === 'gnd') return new Array<number>(augTran.axis.values.length).fill(0);
          const hit = tranKeys.find((k) => k.toLowerCase() === lower);
          return hit ? augTran.signals[hit] : null;
        };
        if (pins) {
          const sa = readNetSeries(pins.posNet);
          const sb = readNetSeries(pins.negNet);
          if (sa && sb && sa.length === sb.length) {
            const diff = sa.map((v, idx) => v - sb[idx]);
            const summary = summarizeWaveformSeries(augTran.axis.values, diff);
            if (summary) metrics.push({ label: 'ΔV (RMS)', value: formatEngineering(summary.rms, 'V') });
          }
        }
        const currKey = findComponentCurrentSignalKey(selectedComponent.id, kindName, tranKeys);
        if (currKey) {
          const summary = summarizeWaveformSeries(augTran.axis.values, augTran.signals[currKey]);
          if (summary) metrics.push({ label: 'I (RMS)', value: formatEngineering(summary.rms, 'A') });
        }
        const powerKey = tranKeys.find((k) => k.toLowerCase() === `p(${selectedComponent.id.toLowerCase()})`);
        if (powerKey) {
          const summary = summarizeWaveformSeries(augTran.axis.values, augTran.signals[powerKey]);
          if (summary) metrics.push({ label: 'P (avg)', value: formatEngineering(summary.average, 'W') });
        }
      }
    }

    const stresses = evaluation.part_stress.results.filter(
      (r) => r.component === selectedComponent.id || r.component === selectedComponent.reference,
    );
    for (const stress of stresses) {
      if (stress.actual != null) {
        const unitMap: Record<string, string> = { volt: 'V', ampere: 'A', watt: 'W' };
        const unit = unitMap[stress.limit.unit.toLowerCase()] ?? stress.limit.unit;
        const util = stress.utilization_percent != null ? ` (${stress.utilization_percent.toPrecision(3)}%)` : '';
        metrics.push({
          label: stress.rating.replaceAll('_', ' '),
          value: `${formatEngineering(stress.actual, unit)}${util}`,
        });
      }
    }
    return metrics.length > 0 ? metrics : null;
  }, [circuitIr, evaluation, irComponents, nets, schematic, selectedComponent]);

  const plottableComponentSignals = useMemo(() => {
    if (!selectedComponent || !evaluation) return null;
    const plottable = evaluation.simulation.datasets.filter((d) => d.data.kind !== 'operating_point');
    if (plottable.length === 0) return null;

    const irComp = irComponents.get(selectedComponent.id);
    const kindName = componentKindName(irComp?.kind ?? '');
    const pins = findComponentPinNets(selectedComponent.id, kindName, schematic, circuitIr);

    let voltageSignal: string | null = null;
    let currentSignal: string | null = null;
    let powerSignal: string | null = null;

    if (pins) {
      const posGround = /^(0|gnd)$/i.test(pins.posNet);
      const negGround = /^(0|gnd)$/i.test(pins.negNet);
      if (!(posGround && negGround)) {
        const vCandidate = componentVoltageSignalKey(pins.posNet, pins.negNet);
        for (const entry of plottable) {
          const aug = augmentDatasetWithDerivedSignals(entry.data, circuitIr, schematic, [vCandidate]);
          if (aug.kind === 'operating_point') continue;
          const hit = Object.keys(aug.signals).find((k) => k.toLowerCase() === vCandidate.toLowerCase());
          if (hit) {
            voltageSignal = hit;
            break;
          }
        }
      }
    }

    for (const entry of plottable) {
      const aug = augmentDatasetWithDerivedSignals(entry.data, circuitIr, schematic);
      if (aug.kind === 'operating_point') continue;
      const keys = Object.keys(aug.signals);
      if (!currentSignal) {
        currentSignal = findComponentCurrentSignalKey(selectedComponent.id, kindName, keys);
      }
      if (!powerSignal && entry.data.kind !== 'ac') {
        powerSignal = keys.find((k) => k.toLowerCase() === `p(${selectedComponent.id.toLowerCase()})`) ?? null;
      }
    }

    if (!voltageSignal && !currentSignal && !powerSignal) return null;
    return { voltageSignal, currentSignal, powerSignal };
  }, [circuitIr, evaluation, irComponents, schematic, selectedComponent]);

  const literalSpan = useMemo(() => {
    if (!selectedComponent || !code) return null;
    if ((irComponents.get(selectedComponent.id)?.instance_path?.length ?? 0) > 0) return null;
    return findLiteralComponentValueSpan(code, selectedComponent.id);
  }, [code, irComponents, selectedComponent]);

  useEffect(() => {
    setTweakDraft(literalSpan?.rawValue ?? '');
  }, [literalSpan?.componentId, literalSpan?.rawValue]);

  const isSelectedNetPlottable = useMemo(() => {
    if (!selectedNet || !evaluation) return false;
    const key = selectedNet.name.toLowerCase();
    if (selectedNet.kind === 'ground' || key === '0' || key === 'gnd') return false;
    return evaluation.simulation.datasets.some(
      (entry) => entry.data.kind !== 'operating_point'
        && Object.keys(entry.data.signals).some((sig) => sig.toLowerCase() === key),
    );
  }, [evaluation, selectedNet]);

  const parsedDraft = useMemo(() => parseKessQuantity(tweakDraft), [tweakDraft]);
  const isDraftValid = Boolean(
    literalSpan
    && parsedDraft
    && (
      literalSpan.kind === 'source'
      || literalSpan.kind === 'current_source'
      || parsedDraft.value > 0
    ),
  );

  const applyScaledTweak = (multiplier: number) => {
    if (!literalSpan || !onTweakCode || !code) return;
    const nextValue = literalSpan.value * multiplier;
    if (!Number.isFinite(nextValue)) return;
    if (
      (literalSpan.kind === 'resistor' || literalSpan.kind === 'capacitor' || literalSpan.kind === 'inductor')
      && nextValue <= 0
    ) return;
    const nextToken = formatKessQuantity(nextValue, literalSpan.suffix);
    onTweakCode(`${code.slice(0, literalSpan.start)}${nextToken}${code.slice(literalSpan.end)}`);
  };

  const applyDraftTweak = () => {
    if (!literalSpan || !onTweakCode || !code || !parsedDraft || !isDraftValid) return;
    const nextToken = tweakDraft.trim();
    onTweakCode(`${code.slice(0, literalSpan.start)}${nextToken}${code.slice(literalSpan.end)}`);
  };

  const focusSelector = (selector: string) => {
    globalThis.requestAnimationFrame(() => {
      const surface = documentRef.current?.parentElement;
      const matches = [...(documentRef.current?.querySelectorAll<SVGGraphicsElement>(selector) ?? [])];
      if (!surface || matches.length === 0) return;
      const boxes = matches.map((element) => element.getBoundingClientRect());
      const left = Math.min(...boxes.map((box) => box.left));
      const right = Math.max(...boxes.map((box) => box.right));
      const top = Math.min(...boxes.map((box) => box.top));
      const bottom = Math.max(...boxes.map((box) => box.bottom));
      const viewport = surface.getBoundingClientRect();
      setView((current) => ({
        ...current,
        pan: {
          x: current.pan.x + viewport.left + viewport.width / 2 - (left + right) / 2,
          y: current.pan.y + viewport.top + viewport.height / 2 - (top + bottom) / 2,
        },
      }));
    });
  };

  const chooseComponent = (id: string, focus = false) => {
    setSelection({ type: 'component', id });
    setNavigatorOpen(true);
    if (focus) focusSelector(`[data-component="${CSS.escape(id)}"]`);
  };
  const chooseNet = (id: number, focus = false) => {
    setSelection({ type: 'net', id });
    setNavigatorOpen(true);
    if (focus) focusSelector(`[data-net="${id}"]`);
  };
  const chooseGroup = (path: string[]) => {
    setSelection({ type: 'group', path });
    setNavigatorOpen(true);
    globalThis.requestAnimationFrame(() => focusSelector('.is-group-active'));
  };

  useEffect(() => {
    setHoveredComponent(null);
    setSelection(null);
    setNavigatorOpen(false);
    setQuery('');
  }, [documentRevision]);

  useEffect(() => {
    setHoveredComponent(null);
    if (!svg || !schematic) {
      setSelection(null);
      return;
    }
    setSelection((current) => {
      if (!current) return null;
      if (current.type === 'component') {
        return schematic.components.some((c) => c.id === current.id) ? current : null;
      }
      if (current.type === 'net') {
        return schematic.nets.some((n) => n.id === current.id) ? current : null;
      }
      const hasGroup = (circuitIr?.components ?? []).some(
        (c) => (c.instance_path?.length ?? 0) > 0 && pathStartsWith(c.instance_path!, current.path),
      );
      return hasGroup ? current : null;
    });
  }, [circuitIr, schematic, svg]);

  useEffect(() => {
    const root = documentRef.current;
    if (!root) return;
    const selectedGroup = selection?.type === 'group' ? selection.path : null;
    const connected = new Set(
      selection?.type === 'net'
        ? nets.find((net) => net.id === selection.id)?.pins.map((pin) => pin.component) ?? []
        : [],
    );
    root.classList.toggle('has-active-selection', selection !== null);
    for (const node of root.querySelectorAll('[data-component]')) {
      const id = node.getAttribute('data-component') ?? '';
      const instancePath = irComponents.get(id)?.instance_path ?? [];
      node.classList.toggle('is-component-active', activeComponent === id);
      node.classList.toggle('is-net-connected', connected.has(id));
      node.classList.toggle('is-group-active', selectedGroup !== null && pathStartsWith(instancePath, selectedGroup));
    }
    for (const node of root.querySelectorAll('[data-net]')) {
      node.classList.toggle(
        'is-net-active',
        selection?.type === 'net' && node.getAttribute('data-net') === String(selection.id),
      );
    }
  }, [activeComponent, irComponents, nets, selection, svg, view]);

  useEffect(() => {
    const clearSelection = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setSelection(null);
        setNavigatorOpen(false);
      }
    };
    window.addEventListener('keydown', clearSelection);
    return () => window.removeEventListener('keydown', clearSelection);
  }, []);

  const normalizedQuery = query.trim().toLocaleLowerCase('en-US');
  const matchingComponents = components.filter((component) => {
    const ir = irComponents.get(component.id);
    return [component.reference, component.id, component.value, component.model,
      ...(ir?.instance_path ?? [])].filter(Boolean).join(' ').toLocaleLowerCase('en-US').includes(normalizedQuery);
  }).slice(0, 40);
  const matchingNets = nets.filter((net) => net.name.toLocaleLowerCase('en-US').includes(normalizedQuery)).slice(0, 40);

  const componentRow = (component: SchematicComponent) => {
    const ir = irComponents.get(component.id);
    return <button key={component.id} className="navigator-result" onClick={() => chooseComponent(component.id, true)}>
      <span><strong>{component.reference}</strong><small>{component.value ?? component.model ?? componentKind(ir?.kind)}</small></span>
      {(ir?.instance_path?.length ?? 0) > 0 && <small>{ir!.instance_path!.join(' / ')}</small>}
    </button>;
  };

  return (
    <section className="workspace-panel schematic-panel" aria-label="Canonical schematic">
      <PanelHeader controls={panelControls} icon={<CircuitBoard size={15} />} title="Schematic">
        <span className={`quality-badge ${schematic?.quality.passed ? 'quality-pass' : ''}`}>
          {schematic?.connectivity.verified ? 'Connectivity verified' : sourceFeedback?.title ?? 'Waiting'}
        </span>
        <div className="icon-actions" aria-label="Schematic view">
          <button
            aria-label="Find and inspect schematic"
            aria-pressed={navigatorOpen}
            title="Find components and nets"
            onClick={() => setNavigatorOpen((current) => !current)}
          ><Search size={14} /></button>
          <button
            aria-label="Toggle schematic grid"
            aria-pressed={gridVisible}
            onClick={() => setGridVisible((current) => !current)}
          ><Grid3X3 size={14} /></button>
          <button aria-label="Zoom out" onClick={() => setView((current) => ({ ...current, zoom: Math.max(0.2, current.zoom - 0.2) }))}><Minus size={14} /></button>
          <button aria-label="Fit view" onClick={reset}><Focus size={14} /></button>
          <button aria-label="Zoom in" onClick={() => setView((current) => ({ ...current, zoom: Math.min(4, current.zoom + 0.2) }))}><Plus size={14} /></button>
        </div>
      </PanelHeader>
      <div
        className={`schematic-surface${gridVisible ? ' has-grid' : ''}${dragStart ? ' is-dragging' : ''}`}
        tabIndex={0}
        onWheel={(event) => {
          event.preventDefault();
          const bounds = event.currentTarget.getBoundingClientRect();
          const anchor = {
            x: event.clientX - bounds.left - bounds.width / 2,
            y: event.clientY - bounds.top - bounds.height / 2,
          };
          setView((current) => {
            const nextZoom = Math.max(0.2, Math.min(4, current.zoom * Math.exp(-event.deltaY * 0.0015)));
            const ratio = nextZoom / current.zoom;
            return {
              zoom: nextZoom,
              pan: {
                x: anchor.x - (anchor.x - current.pan.x) * ratio,
                y: anchor.y - (anchor.y - current.pan.y) * ratio,
              },
            };
          });
        }}
        onDoubleClick={(event) => {
          if (!onRevealSymbol) return;
          const directNet = netAtTarget(event.target);
          const directComp = componentAtTarget(event.target);
          const component = directComp
            ?? (directNet === null
              ? componentAtPointer(event.target, event.currentTarget, event.clientX, event.clientY)
              : null);
          if (component) {
            onRevealSymbol(component);
            return;
          }
          const netId = directNet ?? netAtPointer(event.target, event.currentTarget, event.clientX, event.clientY);
          if (netId !== null) {
            const net = nets.find((item) => item.id === netId);
            if (net) onRevealSymbol(net.name);
          }
        }}
        onPointerDown={(event) => {
          event.currentTarget.focus();
          const directNet = netAtTarget(event.target);
          const directComp = componentAtTarget(event.target);
          const component = directComp
            ?? (directNet === null
              ? componentAtPointer(event.target, event.currentTarget, event.clientX, event.clientY)
              : null);
          if (component) {
            setSelection((current) => current?.type === 'component' && current.id === component ? null : { type: 'component', id: component });
            setNavigatorOpen(true);
            return;
          }
          const netId = directNet ?? netAtPointer(event.target, event.currentTarget, event.clientX, event.clientY);
          if (netId !== null) {
            setSelection((current) => current?.type === 'net' && current.id === netId ? null : { type: 'net', id: netId });
            setNavigatorOpen(true);
            return;
          }
          setSelection(null);
          event.currentTarget.setPointerCapture(event.pointerId);
          setDragStart({ x: event.clientX - pan.x, y: event.clientY - pan.y });
        }}
        onPointerMove={(event) => {
          setHoveredComponent(componentAtPointer(event.target, event.currentTarget, event.clientX, event.clientY));
          if (dragStart) {
            setView((current) => ({
              ...current,
              pan: { x: event.clientX - dragStart.x, y: event.clientY - dragStart.y },
            }));
          }
        }}
        onPointerUp={() => setDragStart(null)}
        onPointerLeave={() => { setHoveredComponent(null); setDragStart(null); }}
        onKeyDown={(event) => {
          const delta = event.shiftKey ? 40 : 12;
          if (event.key === 'Escape') setSelection(null);
          if (event.key === 'ArrowLeft') setView((current) => ({ ...current, pan: { ...current.pan, x: current.pan.x - delta } }));
          if (event.key === 'ArrowRight') setView((current) => ({ ...current, pan: { ...current.pan, x: current.pan.x + delta } }));
          if (event.key === 'ArrowUp') setView((current) => ({ ...current, pan: { ...current.pan, y: current.pan.y - delta } }));
          if (event.key === 'ArrowDown') setView((current) => ({ ...current, pan: { ...current.pan, y: current.pan.y + delta } }));
        }}
      >
        {svg ? (
          <div
            ref={documentRef}
            className="schematic-document"
            data-testid="canonical-schematic"
            data-quality={schematic?.quality.passed ? 'pass' : 'warning'}
            data-selected-component={selection?.type === 'component' ? selection.id : undefined}
            data-selected-net={selection?.type === 'net' ? selection.id : undefined}
            style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        ) : <div className="empty-state source-recovery" onPointerDown={(event) => event.stopPropagation()}>
          <strong>{sourceFeedback?.title ?? 'Waiting for a valid circuit'}</strong>
          <p>{sourceFeedback?.message ?? 'A valid source will generate the schematic automatically.'}</p>
          {sourceFeedback?.actionLabel && <button className="secondary-button" onClick={onResolveSource}>{sourceFeedback.actionLabel}</button>}
        </div>}

        {svg && navigatorOpen && <aside className="schematic-navigator" aria-label="Schematic inspector" onPointerDown={(event) => event.stopPropagation()} onWheel={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key.startsWith('Arrow')) event.stopPropagation(); }}>
          <header><div><strong>{selection ? 'Inspect' : 'Navigate'}</strong><small>{components.length} components · {nets.length} nets</small></div><button aria-label="Close schematic inspector" onClick={() => setNavigatorOpen(false)}><X size={14} /></button></header>
          <label className="navigator-search"><Search size={13} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find component or net" /></label>

          {selectedComponent && <div className="inspector-content">
            <div className="inspector-heading">
              <strong>{selectedComponent.reference}</strong>
              <span className="inspector-badge">{componentKind(irComponents.get(selectedComponent.id)?.kind)}</span>
              {onRevealSymbol && (
                <button
                  type="button"
                  className="inspector-source-link"
                  title="Reveal declaration in Source editor"
                  onClick={() => onRevealSymbol(selectedComponent.id)}
                >
                  <Code2 size={11} /> Source
                </button>
              )}
            </div>
            {irComponents.get(selectedComponent.id)?.instance_path?.length
              ? <button className="group-breadcrumb" onClick={() => chooseGroup(irComponents.get(selectedComponent.id)!.instance_path!)}><Boxes size={12} />{irComponents.get(selectedComponent.id)!.instance_path!.join(' / ')}</button>
              : <small className="inspector-scope-label">Top-level circuit</small>}
            <dl className="inspector-meta-list">
              {selectedComponent.value && <><dt>Value</dt><dd>{selectedComponent.value}</dd></>}
              {selectedComponent.model && <><dt>Model</dt><dd>{selectedComponent.model}</dd></>}
              {Object.entries(selectedComponent.instance_parameters ?? {}).map(([name, value]) => <span className="inspector-pair" key={name}><dt>{name}</dt><dd>{value}</dd></span>)}
            </dl>
            {literalSpan && onTweakCode && (
              <div className="inspector-tweak-section">
                <h4><SlidersHorizontal size={10} /> Quick tweak</h4>
                <div className="inspector-tweak-bar" role="group" aria-label="Quick value tweak">
                  <button type="button" className="inspector-tweak-step" onClick={() => applyScaledTweak(0.9)}>-10%</button>
                  <input
                    type="text"
                    className={`inspector-tweak-input${isDraftValid ? '' : ' is-invalid'}`}
                    aria-label="Component value"
                    value={tweakDraft}
                    onChange={(event) => setTweakDraft(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') {
                        event.preventDefault();
                        applyDraftTweak();
                      }
                    }}
                  />
                  <button type="button" className="inspector-tweak-step" onClick={() => applyScaledTweak(1.1)}>+10%</button>
                  <button type="button" className="inspector-tweak-apply" disabled={!isDraftValid} onClick={applyDraftTweak}>Apply</button>
                </div>
              </div>
            )}
            {componentSimulationSummary && (
              <div className="inspector-sim-block">
                <h4><Activity size={10} /> Simulated</h4>
                <dl className="inspector-sim-metrics">
                  {componentSimulationSummary.map((item) => (
                    <span className="inspector-pair" key={item.label}>
                      <dt>{item.label}</dt>
                      <dd>{item.value}</dd>
                    </span>
                  ))}
                </dl>
              </div>
            )}
            {plottableComponentSignals && onPlotSignal && (
              <div className="inspector-action-row" role="group" aria-label="Plot component waveforms">
                {plottableComponentSignals.voltageSignal && (
                  <button type="button" className="inspector-plot-btn" onClick={() => onPlotSignal(plottableComponentSignals.voltageSignal!)}>Plot V</button>
                )}
                {plottableComponentSignals.currentSignal && (
                  <button type="button" className="inspector-plot-btn" onClick={() => onPlotSignal(plottableComponentSignals.currentSignal!)}>Plot I</button>
                )}
                {plottableComponentSignals.powerSignal && (
                  <button type="button" className="inspector-plot-btn" onClick={() => onPlotSignal(plottableComponentSignals.powerSignal!)}>Plot P</button>
                )}
              </div>
            )}
            <h4>Pins</h4>
            <div className="inspector-pin-list">{selectedComponent.pins.map((pin) => {
              const net = nets.find((candidate) => candidate.id === pin.net);
              return <button key={pin.name} disabled={!net} onClick={() => net && chooseNet(net.id, true)}><span>{pin.name}</span><strong>{net?.name ?? 'unconnected'}</strong></button>;
            })}</div>
            {selectedComponent.model_metadata && <details><summary>Model provenance</summary><dl><dt>Source</dt><dd>{selectedComponent.model_metadata.source}</dd><dt>Version</dt><dd>{selectedComponent.model_metadata.version}</dd><dt>Simulator</dt><dd>{selectedComponent.model_metadata.simulator}</dd><dt>Hash</dt><dd className="inspector-hash">{selectedComponent.model_metadata.content_hash}</dd></dl></details>}
          </div>}

          {selectedNet && <div className="inspector-content">
            <div className="inspector-heading">
              <strong>{selectedNet.name}</strong>
              <span className="inspector-badge">{selectedNet.kind} net</span>
              {onRevealSymbol && (
                <button
                  type="button"
                  className="inspector-source-link"
                  title="Reveal net in Source editor"
                  onClick={() => onRevealSymbol(selectedNet.name)}
                >
                  <Code2 size={11} /> Source
                </button>
              )}
            </div>
            {netSimulationSummary && (
              <div className="inspector-sim-block">
                <h4><Activity size={10} /> Simulated</h4>
                <dl className="inspector-sim-metrics">
                  {netSimulationSummary.map((item) => (
                    <span className="inspector-pair" key={item.label}>
                      <dt>{item.label}</dt>
                      <dd>{item.value}</dd>
                    </span>
                  ))}
                </dl>
              </div>
            )}
            {isSelectedNetPlottable && onPlotSignal && (
              <div className="inspector-action-row" role="group" aria-label="Plot net waveform">
                <button type="button" className="inspector-plot-btn" onClick={() => onPlotSignal(selectedNet.name)}>Plot</button>
                {onCompareSignal && (
                  <button type="button" className="inspector-plot-btn is-secondary" onClick={() => onCompareSignal(selectedNet.name)}>+ Compare</button>
                )}
              </div>
            )}
            <h4>Connected pins</h4>
            <div className="inspector-pin-list">{selectedNet.pins.map((pin) => <button key={`${pin.component}.${pin.pin}`} onClick={() => chooseComponent(pin.component, true)}><span>{pin.pin}</span><strong>{pin.component}</strong></button>)}</div>
          </div>}

          {selection?.type === 'group' && <div className="inspector-content">
            <div className="inspector-heading">
              <strong>{selection.path.at(-1)}</strong>
              <span>Reusable block</span>
              {onRevealSymbol && selection.path.at(-1) && (
                <button
                  type="button"
                  className="inspector-source-link"
                  title="Reveal block in Source editor"
                  onClick={() => onRevealSymbol(selection.path.at(-1)!)}
                >
                  <Code2 size={11} /> Source
                </button>
              )}
            </div>
            <small>{selection.path.join(' / ')}</small>
            <h4>Components</h4>
            <div className="navigator-results">{components.filter((component) => pathStartsWith(irComponents.get(component.id)?.instance_path ?? [], selection.path)).map(componentRow)}</div>
          </div>}

          {!selection && !normalizedQuery && <div className="navigator-section">
            {groups.length > 0 && <><h4>Reusable blocks</h4>{groups.map((path) => <button className="navigator-result" key={pathKey(path)} onClick={() => chooseGroup(path)}><span><strong>{path.at(-1)}</strong><small>{components.filter((component) => pathStartsWith(irComponents.get(component.id)?.instance_path ?? [], path)).length} components</small></span><small>{path.slice(0, -1).join(' / ')}</small></button>)}</>}
            <h4>Components</h4>
            <div className="navigator-results">{components.slice(0, 12).map(componentRow)}{components.length > 12 && <small>Search to inspect {components.length - 12} more components.</small>}</div>
            <h4>Nets</h4>
            <div className="navigator-results">{nets.slice(0, 12).map((net) => <button className="navigator-result" key={`net-${net.id}`} onClick={() => chooseNet(net.id, true)}><span><strong>{net.name}</strong><small>{net.kind} · {net.pins.length} pins</small></span></button>)}{nets.length > 12 && <small>Search to inspect {nets.length - 12} more nets.</small>}</div>
          </div>}
          {normalizedQuery && <div className="navigator-section"><h4>Search results</h4><div className="navigator-results">{matchingComponents.map(componentRow)}{matchingNets.map((net) => <button className="navigator-result" key={`net-${net.id}`} onClick={() => chooseNet(net.id, true)}><span><strong>{net.name}</strong><small>{net.kind} · {net.pins.length} pins</small></span></button>)}{matchingComponents.length + matchingNets.length === 0 && <small>No matching component or net.</small>}</div></div>}
        </aside>}
      </div>
    </section>
  );
}
