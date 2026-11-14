import type { CircuitIrSummary, SchematicSummary } from '../domain';
import type { Analysis, AssertionResult, Dataset } from './types';

export type SimulationDataset = Dataset;

/** Nearest recorded sample in displayed coordinates, including descending DC axes. */
export function nearestSampleIndex(coordinates: number[], target: number): number | null {
  if (!coordinates.length) return null;
  const direction = coordinates.at(-1)! >= coordinates[0] ? 1 : -1;
  let low = 0;
  let high = coordinates.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (direction * coordinates[middle] < direction * target) low = middle + 1;
    else high = middle;
  }
  if (low === 0) return 0;
  if (low === coordinates.length) return low - 1;
  return Math.abs(coordinates[low - 1] - target) <= Math.abs(coordinates[low] - target) ? low - 1 : low;
}

export function zoomAroundPointer(zoom: [number, number], fraction: number, zoomOut: boolean): [number, number] {
  const oldWidth = zoom[1] - zoom[0];
  const width = Math.max(0.05, Math.min(1, oldWidth * (zoomOut ? 1.25 : 0.8)));
  const anchor = zoom[0] + fraction * oldWidth;
  const start = Math.max(0, Math.min(1 - width, anchor - fraction * width));
  return [start, start + width];
}

export function signalUnit(name: string): 'W' | 'A' | 'V' {
  const trimmed = name.trim();
  if (/^p\(/i.test(trimmed)) return 'W';
  if (/^i\(/i.test(trimmed) || trimmed.includes('#branch') || /^@.+\[i[bcdesg]?\]$/i.test(trimmed)) return 'A';
  return 'V';
}

export function signalLabel(name: string): string {
  const trimmed = name.trim();
  const deviceCurrent = /^@([dqmj])_(.+)\[(ic|id)\]$/i.exec(trimmed);
  if (deviceCurrent) {
    const kind = deviceCurrent[1].toLowerCase();
    const terminal = deviceCurrent[3].toLowerCase() === 'ic'
      ? 'collector'
      : (kind === 'm' || kind === 'j')
        ? 'drain'
        : 'diode';
    return `${deviceCurrent[2]} ${terminal} current`;
  }
  const power = /^p\((.+)\)$/i.exec(trimmed);
  if (power) return `P(${power[1].trim()})`;
  const current = /^i\((.+)\)$/i.exec(trimmed);
  if (current) return `I(${current[1].trim()})`;
  const diffVoltage = /^v\(([^(),]+),\s*([^(),]+)\)$/i.exec(trimmed);
  if (diffVoltage) return `V(${diffVoltage[1].trim()}, ${diffVoltage[2].trim()})`;
  const singleVoltage = /^v\(([^(),]+)\)$/i.exec(trimmed);
  if (singleVoltage) return `V(${singleVoltage[1].trim()})`;
  return signalUnit(trimmed) === 'A'
    ? `I(${trimmed.replace('#branch', '').replace(/^(?:v_sense_|[vlieh]_)/i, '')})`
    : `V(${trimmed})`;
}

function normalizeSignalToken(signal: string): string {
  return signal.trim().replace(/\s+/g, '').toLowerCase();
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function isNamedVoltage(name: string): boolean {
  if (signalUnit(name) !== 'V') return false;
  const diff = /^v\(([^(),]+),\s*([^(),]+)\)$/i.exec(name.trim());
  if (diff) {
    return !diff[1].includes('.') && !diff[2].includes('.');
  }
  return !name.includes('.')
    && !name.startsWith('n_')
    && !/^(0|gnd|vcc|vee|vdd|vss)$/i.test(name);
}

function matchAssertionSignal(names: string[], assertion: AssertionResult): string | undefined {
  const rawSignal = assertion.signal.trim();
  const normalized = normalizeSignalToken(rawSignal);

  const exact = names.find((name) => normalizeSignalToken(name) === normalized);
  if (exact && (signalUnit(exact) !== 'V' || isNamedVoltage(exact))) return exact;

  const singleV = /^v\(([^(),]+)\)$/i.exec(rawSignal);
  if (singleV) {
    const node = singleV[1].trim().toLowerCase();
    return names.find((name) => name.toLowerCase() === node && isNamedVoltage(name));
  }

  const diffV = /^v\(([^(),]+),\s*([^(),]+)\)$/i.exec(rawSignal);
  if (diffV) {
    const target = `v(${diffV[1].trim().toLowerCase()},${diffV[2].trim().toLowerCase()})`;
    const matched = names.find((name) => normalizeSignalToken(name) === target);
    if (matched) return matched;
    if (/^(0|gnd)$/i.test(diffV[2].trim())) {
      const node = diffV[1].trim().toLowerCase();
      return names.find((name) => name.toLowerCase() === node && isNamedVoltage(name));
    }
  }

  const currentMatch = /^i\(([^()]+)\)$/i.exec(rawSignal);
  if (currentMatch) {
    const comp = currentMatch[1].trim().toLowerCase();
    const expectedDerived = `i(${comp})`;
    return names.find((name) => (
      normalizeSignalToken(name) === expectedDerived
      || normalizeSignalToken(signalLabel(name)) === expectedDerived
      || new RegExp(`^@[dqmj]_${escapeRegExp(comp)}\\[i[cd]\\]$`, 'i').test(name)
    ));
  }

  const powerMatch = /^p\(([^()]+)\)$/i.exec(rawSignal);
  if (powerMatch) {
    const expectedDerived = `p(${powerMatch[1].trim().toLowerCase()})`;
    return names.find((name) => normalizeSignalToken(name) === expectedDerived);
  }

  if (assertion.metric.toLowerCase() === 'dissipation') {
    const expectedDerived = `p(${rawSignal.toLowerCase()})`;
    return names.find((name) => normalizeSignalToken(name) === expectedDerived);
  }

  return undefined;
}

export function preferredSignal(names: string[], assertions: AssertionResult[]): string {
  const asserted = assertions
    .map((assertion) => matchAssertionSignal(names, assertion))
    .find((match): match is string => Boolean(match));
  return names.find((name) => /^(out|output)$/i.test(name))
    ?? asserted
    ?? names.find(isNamedVoltage)
    ?? names[0]
    ?? '';
}

export function sweepAxis(analysis: Analysis): { label: string; unit: string } {
  if (analysis.kind !== 'dc_sweep') return { label: 'Time', unit: 's' };
  const unit = analysis.start.unit === 'Ampere' ? 'A' : 'V';
  return { label: `Sweep of ${analysis.source}`, unit };
}

export interface AcQuickSummary {
  peakMagnitudeDb: number;
  peakFrequencyHz: number;
  peakIndex: number;
  cutoffFrequencyHz: number | null;
  cutoffIndex: number | null;
  phaseAtCutoffOrPeakDeg: number;
  minMagnitudeDb: number;
  minFrequencyHz: number;
  minIndex: number;
}

export function summarizeAcSeries(
  frequenciesHz: number[],
  magnitudeDb: number[],
  phaseDeg: number[],
): AcQuickSummary | null {
  if (frequenciesHz.length === 0 || magnitudeDb.length === 0) return null;
  let peakIndex = 0;
  let minIndex = 0;
  for (let i = 1; i < magnitudeDb.length; i += 1) {
    if (magnitudeDb[i] > magnitudeDb[peakIndex]) peakIndex = i;
    if (magnitudeDb[i] < magnitudeDb[minIndex]) minIndex = i;
  }
  const peakMagnitudeDb = magnitudeDb[peakIndex];
  const targetDb = peakMagnitudeDb - 3.010299956639812; // -3 dB

  let cutoffIndex: number | null = null;
  let cutoffFrequencyHz: number | null = null;

  // Search forward from peak first (low-pass / band-pass upper edge), then backward (high-pass lower edge)
  for (let i = peakIndex; i < magnitudeDb.length - 1; i += 1) {
    if (magnitudeDb[i] >= targetDb && magnitudeDb[i + 1] <= targetDb) {
      const span = magnitudeDb[i] - magnitudeDb[i + 1];
      const frac = span > 1e-12 ? (magnitudeDb[i] - targetDb) / span : 0;
      const logF0 = Math.log10(Math.max(frequenciesHz[i], Number.MIN_VALUE));
      const logF1 = Math.log10(Math.max(frequenciesHz[i + 1], Number.MIN_VALUE));
      cutoffFrequencyHz = 10 ** (logF0 + frac * (logF1 - logF0));
      cutoffIndex = frac < 0.5 ? i : i + 1;
      break;
    }
  }
  if (cutoffFrequencyHz === null) {
    for (let i = peakIndex; i > 0; i -= 1) {
      if (magnitudeDb[i] >= targetDb && magnitudeDb[i - 1] <= targetDb) {
        const span = magnitudeDb[i] - magnitudeDb[i - 1];
        const frac = span > 1e-12 ? (magnitudeDb[i] - targetDb) / span : 0;
        const logF0 = Math.log10(Math.max(frequenciesHz[i], Number.MIN_VALUE));
        const logF1 = Math.log10(Math.max(frequenciesHz[i - 1], Number.MIN_VALUE));
        cutoffFrequencyHz = 10 ** (logF0 + frac * (logF1 - logF0));
        cutoffIndex = frac < 0.5 ? i : i - 1;
        break;
      }
    }
  }

  const refIndex = cutoffIndex ?? peakIndex;
  return {
    peakMagnitudeDb,
    peakFrequencyHz: frequenciesHz[peakIndex],
    peakIndex,
    cutoffFrequencyHz,
    cutoffIndex,
    phaseAtCutoffOrPeakDeg: phaseDeg[refIndex] ?? 0,
    minMagnitudeDb: magnitudeDb[minIndex],
    minFrequencyHz: frequenciesHz[minIndex],
    minIndex,
  };
}

export interface WaveformQuickSummary {
  minValue: number;
  minAxis: number;
  minIndex: number;
  maxValue: number;
  maxAxis: number;
  maxIndex: number;
  peakToPeak: number;
  average: number;
  rms: number;
}

export function summarizeWaveformSeries(
  axis: number[],
  values: number[],
): WaveformQuickSummary | null {
  if (axis.length === 0 || values.length === 0) return null;
  let minIndex = 0;
  let maxIndex = 0;
  let sum = 0;
  let sumSq = 0;
  for (let i = 0; i < values.length; i += 1) {
    const v = values[i];
    if (v < values[minIndex]) minIndex = i;
    if (v > values[maxIndex]) maxIndex = i;
    sum += v;
    sumSq += v * v;
  }
  const minValue = values[minIndex];
  const maxValue = values[maxIndex];
  return {
    minValue,
    minAxis: axis[minIndex],
    minIndex,
    maxValue,
    maxAxis: axis[maxIndex],
    maxIndex,
    peakToPeak: maxValue - minValue,
    average: sum / values.length,
    rms: Math.sqrt(sumSq / values.length),
  };
}

const SI_PREFIX_FACTORS: ReadonlyArray<[RegExp, number]> = [
  [/^meg/i, 1e6],
  [/^T/, 1e12],
  [/^G/, 1e9],
  [/^M/, 1e6],
  [/^[kK]/, 1e3],
  [/^m/, 1e-3],
  [/^[uµ]/, 1e-6],
  [/^n/, 1e-9],
  [/^p/, 1e-12],
];

const SUPPORTED_UNITS = /^(?:Ohm|ohm|OHM|Ω|F|H|[Vv]|[Aa]|Hz|hz|HZ|s|S|siemens|Siemens|mho|[Ww]|[Jj]|%|deg|degrees?)$/;

export function parseKessQuantity(raw: string): { value: number; unit: string | null } | null {
  const trimmed = raw.trim();
  const numMatch = /^([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)(.*)$/.exec(trimmed);
  if (!numMatch) return null;
  const numeric = Number(numMatch[1]);
  if (!Number.isFinite(numeric)) return null;
  let suffix = numMatch[2].trim();
  let factor = 1;
  for (const [prefixPattern, prefixFactor] of SI_PREFIX_FACTORS) {
    if (prefixPattern.test(suffix)) {
      const prefixLen = suffix.match(prefixPattern)![0].length;
      const rest = suffix.slice(prefixLen);
      if (rest === '' || SUPPORTED_UNITS.test(rest)) {
        factor = prefixFactor;
        suffix = rest;
        break;
      }
    }
  }
  if (suffix !== '' && !SUPPORTED_UNITS.test(suffix)) return null;
  return { value: numeric * factor, unit: suffix || null };
}

const FORMAT_PREFIXES: ReadonlyArray<[number, string]> = [
  [1e12, 'T'],
  [1e9, 'G'],
  [1e6, 'M'],
  [1e3, 'k'],
  [1, ''],
  [1e-3, 'm'],
  [1e-6, 'u'],
  [1e-9, 'n'],
  [1e-12, 'p'],
];

export function formatKessQuantity(value: number, suffix = ''): string {
  if (!Number.isFinite(value)) return `0${suffix}`;
  if (value === 0) return `0${suffix}`;
  const sign = value < 0 ? '-' : '';
  const abs = Math.abs(value);
  let chosenFactor = 1;
  let chosenPrefix = '';
  if (abs >= 1e15 || abs < 1e-14) {
    return `${sign}${Number(abs.toPrecision(4)).toExponential()}${suffix}`;
  }
  for (const [factor, prefix] of FORMAT_PREFIXES) {
    if (abs >= factor * (1 - 1e-9) || prefix === 'p') {
      chosenFactor = factor;
      chosenPrefix = prefix;
      break;
    }
  }
  let scaled = Number((abs / chosenFactor).toPrecision(4));
  if (scaled >= 1000 && chosenFactor < 1e12) {
    const idx = FORMAT_PREFIXES.findIndex(([f]) => f === chosenFactor);
    if (idx > 0) {
      chosenFactor = FORMAT_PREFIXES[idx - 1][0];
      chosenPrefix = FORMAT_PREFIXES[idx - 1][1];
      scaled = Number((abs / chosenFactor).toPrecision(4));
    }
  }
  const numStr = String(scaled);
  return `${sign}${numStr}${chosenPrefix}${suffix}`;
}

export type QuickTweakKind = 'resistor' | 'capacitor' | 'inductor' | 'source' | 'current_source';

export interface ComponentValueSpan {
  componentId: string;
  kind: QuickTweakKind;
  start: number;
  end: number;
  rawValue: string;
  value: number;
  suffix: string;
}

const DEFAULT_KIND_SUFFIX: Record<QuickTweakKind, string> = {
  resistor: '',
  capacitor: 'F',
  inductor: 'H',
  source: 'V',
  current_source: 'A',
};

export function findLiteralComponentValueSpan(source: string, componentId: string): ComponentValueSpan | null {
  const lines = source.split('\n');
  let offset = 0;
  let braceDepth = 0;
  const targetLower = componentId.toLowerCase();

  for (const rawLine of lines) {
    const lineWithoutCr = rawLine.endsWith('\r') ? rawLine.slice(0, -1) : rawLine;
    const commentIdx = lineWithoutCr.indexOf('//');
    const codePart = commentIdx >= 0 ? lineWithoutCr.slice(0, commentIdx) : lineWithoutCr;

    if (braceDepth === 0) {
      const match = /^(\s*)(resistor|capacitor|inductor|source|current_source)\s+([A-Za-z][A-Za-z0-9_]*)\s+([^\s{("]+)\s*$/.exec(codePart);
      if (match && match[3].toLowerCase() === targetLower) {
        const rawValue = match[4];
        const parsed = parseKessQuantity(rawValue);
        if (parsed) {
          const kind = match[2] as QuickTweakKind;
          const tokenStartInLine = codePart.lastIndexOf(rawValue);
          const start = offset + tokenStartInLine;
          const end = start + rawValue.length;
          return {
            componentId: match[3],
            kind,
            start,
            end,
            rawValue,
            value: parsed.value,
            suffix: parsed.unit ?? DEFAULT_KIND_SUFFIX[kind],
          };
        }
      }
    }

    for (let i = 0; i < codePart.length; i += 1) {
      if (codePart[i] === '{') braceDepth += 1;
      else if (codePart[i] === '}' && braceDepth > 0) braceDepth -= 1;
    }
    offset += rawLine.length + 1;
  }

  return null;
}

export function componentKindName(kind: string | Record<string, unknown>): string {
  if (typeof kind === 'string') return kind;
  return Object.keys(kind)[0] ?? '';
}

function readIrQuantityValue(raw: unknown): number | null {
  if (typeof raw === 'number') return Number.isFinite(raw) ? raw : null;
  if (typeof raw === 'string') return parseKessQuantity(raw)?.value ?? null;
  if (raw && typeof raw === 'object' && 'value' in raw) {
    const candidate = (raw as { value: unknown }).value;
    if (typeof candidate === 'number' && Number.isFinite(candidate)) return candidate;
  }
  return null;
}

export function extractComponentNumericValue(
  component: CircuitIrSummary['components'][number],
): number | null {
  const params = component.parameters;
  if (!params || typeof params !== 'object') return null;
  const record = params as Record<string, unknown>;

  const twoPin = record.TwoPinPassive as { value?: unknown } | undefined;
  if (twoPin?.value !== undefined) return readIrQuantityValue(twoPin.value);

  const dependent = record.DependentSource as { value?: unknown } | undefined;
  if (dependent?.value !== undefined) return readIrQuantityValue(dependent.value);

  const vSource = record.VoltageSource as { value?: { Dc?: unknown } } | undefined;
  if (vSource?.value?.Dc !== undefined) return readIrQuantityValue(vSource.value.Dc);

  const iSource = record.CurrentSource as { value?: { Dc?: unknown } } | undefined;
  if (iSource?.value?.Dc !== undefined) return readIrQuantityValue(iSource.value.Dc);

  for (const key of ['r', 'c', 'l', 'dc', 'value']) {
    if (key in record) {
      const val = readIrQuantityValue(record[key]);
      if (val !== null) return val;
    }
  }
  return null;
}

function isGroundNetName(net: string): boolean {
  const lower = net.trim().toLowerCase();
  return lower === '0' || lower === 'gnd';
}

function solveComponentPinsFromIr(
  circuitIr: CircuitIrSummary,
): Map<string, Array<{ pin: string; net: string }>> {
  const result = new Map<string, Array<{ pin: string; net: string }>>();
  const connections = circuitIr.connections;
  if (!connections?.length) return result;

  const parent = new Map<string, string>();
  const find = (key: string): string => {
    let cur = parent.get(key);
    if (!cur) {
      parent.set(key, key);
      return key;
    }
    while (parent.get(cur) !== cur) {
      cur = parent.get(cur)!;
    }
    let node = key;
    while (parent.get(node) !== cur) {
      const next = parent.get(node)!;
      parent.set(node, cur);
      node = next;
    }
    return cur;
  };
  const unite = (a: string, b: string) => {
    const ra = find(a);
    const rb = find(b);
    if (ra !== rb) parent.set(rb, ra);
  };
  const refKey = (component: string, pin: string) => `${component}\u0000${pin}`;

  for (const conn of connections) {
    if (!conn.pins.length) continue;
    const first = refKey(conn.pins[0].component, conn.pins[0].pin);
    for (let i = 1; i < conn.pins.length; i += 1) {
      unite(first, refKey(conn.pins[i].component, conn.pins[i].pin));
    }
  }

  const declaredNets = new Set((circuitIr.nets ?? []).map((n) => n.toLowerCase()));
  const rootNet = new Map<string, string>();
  for (const conn of connections) {
    for (const ref of conn.pins) {
      const key = refKey(ref.component, ref.pin);
      const root = find(key);
      if (ref.pin === '' || isGroundNetName(ref.component) || declaredNets.has(ref.component.toLowerCase())) {
        const existing = rootNet.get(root);
        if (!existing || (!isGroundNetName(existing) && isGroundNetName(ref.component))) {
          rootNet.set(root, isGroundNetName(ref.component) ? '0' : ref.component);
        }
      }
    }
  }

  let autoNetIndex = 1;
  for (const conn of connections) {
    for (const ref of conn.pins) {
      if (ref.pin === '') continue;
      const root = find(refKey(ref.component, ref.pin));
      let netName = rootNet.get(root);
      if (!netName) {
        netName = `n_${autoNetIndex}`;
        autoNetIndex += 1;
        rootNet.set(root, netName);
      }
      const list = result.get(ref.component.toLowerCase()) ?? [];
      if (!list.some((item) => item.pin.toLowerCase() === ref.pin.toLowerCase())) {
        list.push({ pin: ref.pin, net: netName });
      }
      result.set(ref.component.toLowerCase(), list);
    }
  }

  return result;
}

function canonicalTerminalPair(
  kindName: string,
  pins: Array<{ pin: string; net: string }>,
): { posNet: string; negNet: string } | null {
  if (!pins.length) return null;
  const byPin = (candidate: string) =>
    pins.find((item) => item.pin.toLowerCase() === candidate.toLowerCase())?.net;
  const lowerKind = kindName.toLowerCase();
  if (lowerKind === 'bjt') {
    const posNet = byPin('c') ?? pins[0]?.net;
    const negNet = byPin('e') ?? pins[2]?.net ?? pins[1]?.net;
    return posNet && negNet ? { posNet, negNet } : null;
  }
  if (lowerKind === 'mosfet' || lowerKind === 'jfet') {
    const posNet = byPin('d') ?? pins[0]?.net;
    const negNet = byPin('s') ?? pins[2]?.net ?? pins[1]?.net;
    return posNet && negNet ? { posNet, negNet } : null;
  }
  const posNet = byPin('p1') ?? byPin('anode') ?? byPin('a') ?? byPin('pos') ?? pins[0]?.net;
  const negNet = byPin('p2') ?? byPin('cathode') ?? byPin('k') ?? byPin('neg') ?? pins[1]?.net;
  return posNet && negNet ? { posNet, negNet } : null;
}

function netNameMapFromSchematic(schematic?: SchematicSummary | null): Map<number, string> {
  const map = new Map<number, string>();
  if (!schematic) return map;
  for (const net of schematic.nets) {
    map.set(net.id, net.kind === 'ground' ? '0' : net.name);
  }
  return map;
}

function pinsFromSchematicComponent(
  componentId: string,
  schematic?: SchematicSummary | null,
  netMap?: Map<number, string>,
): Array<{ pin: string; net: string }> | null {
  if (!schematic) return null;
  const lowerId = componentId.toLowerCase();
  const comp = schematic.components.find(
    (c) => c.id.toLowerCase() === lowerId || c.reference.toLowerCase() === lowerId,
  );
  if (!comp) return null;
  const resolvedNetMap = netMap ?? netNameMapFromSchematic(schematic);
  return comp.pins.flatMap((p) => {
    if (p.net == null) return [];
    const netName = resolvedNetMap.get(p.net);
    return netName ? [{ pin: p.name, net: netName }] : [];
  });
}

export function findComponentPinNets(
  componentId: string,
  kindName: string,
  schematic?: SchematicSummary | null,
  circuitIr?: CircuitIrSummary | null,
): { posNet: string; negNet: string } | null {
  const schemPins = pinsFromSchematicComponent(componentId, schematic);
  if (schemPins) return canonicalTerminalPair(kindName, schemPins);
  if (circuitIr) {
    const irPins = solveComponentPinsFromIr(circuitIr).get(componentId.toLowerCase());
    if (irPins) return canonicalTerminalPair(kindName, irPins);
  }
  return null;
}

export function findComponentCurrentSignalKey(
  componentId: string,
  kindName: string,
  signalNames: string[],
): string | null {
  const lowerId = componentId.toLowerCase();
  const k = kindName.toLowerCase();
  const candidates: string[] = [`i(${lowerId})`];
  if (k === 'voltagesource') candidates.push(`v_${lowerId}#branch`);
  else if (k === 'inductor') candidates.push(`l_${lowerId}#branch`);
  else if (k === 'vcvs') candidates.push(`e_${lowerId}#branch`);
  else if (k === 'ccvs') candidates.push(`h_${lowerId}#branch`, `v_sense_${lowerId}#branch`);
  else if (k === 'diode') candidates.push(`@d_${lowerId}[id]`);
  else if (k === 'bjt') candidates.push(`@q_${lowerId}[ic]`);
  else if (k === 'mosfet') candidates.push(`@m_${lowerId}[id]`);
  else if (k === 'jfet') candidates.push(`@j_${lowerId}[id]`);

  for (const candidate of candidates) {
    const hit = signalNames.find((name) => normalizeSignalToken(name) === normalizeSignalToken(candidate));
    if (hit) return hit;
  }
  return null;
}

export function componentVoltageSignalKey(posNet: string, negNet: string): string {
  if (isGroundNetName(negNet)) return posNet;
  return `v(${posNet}, ${negNet})`;
}

const BRANCH_POWER_KINDS = new Set([
  'voltagesource',
  'inductor',
  'diode',
  'bjt',
  'mosfet',
  'jfet',
  'vcvs',
  'ccvs',
]);

function resolveNetName(
  endpoint: string,
  schematic?: SchematicSummary | null,
  schemNetMap?: Map<number, string>,
  irPinsMap?: Map<string, Array<{ pin: string; net: string }>>,
): string {
  const trimmed = endpoint.trim();
  const dot = trimmed.indexOf('.');
  if (dot <= 0) return trimmed;
  const compId = trimmed.slice(0, dot).toLowerCase();
  const pinId = trimmed.slice(dot + 1).toLowerCase();
  const canonicalPin = pinId === 'anode' || pinId === 'a'
    ? 'p1'
    : pinId === 'cathode' || pinId === 'k'
      ? 'p2'
      : pinId;
  const schemPins = pinsFromSchematicComponent(compId, schematic, schemNetMap);
  const hit = (schemPins ?? irPinsMap?.get(compId))?.find(
    (p) => p.pin.toLowerCase() === pinId || p.pin.toLowerCase() === canonicalPin,
  );
  return hit ? hit.net : trimmed;
}

export function augmentDatasetWithDerivedSignals(
  dataset: Dataset,
  circuitIr: CircuitIrSummary | null,
  schematic?: SchematicSummary | null,
  extraSignals: string[] = [],
): Dataset {
  const components = circuitIr?.components ?? [];
  const assertions = circuitIr?.assertions ?? [];
  if (!components.length && !assertions.length && !extraSignals.length) {
    return dataset;
  }

  const schemNetMap = schematic ? netNameMapFromSchematic(schematic) : undefined;
  const irPinsMap = circuitIr && !schematic ? solveComponentPinsFromIr(circuitIr) : undefined;
  const getComponentPins = (compId: string, kindName: string): { posNet: string; negNet: string } | null => {
    const schemPins = pinsFromSchematicComponent(compId, schematic, schemNetMap);
    if (schemPins) return canonicalTerminalPair(kindName, schemPins);
    const irPins = irPinsMap?.get(compId.toLowerCase());
    if (irPins) return canonicalTerminalPair(kindName, irPins);
    return null;
  };

  // Gather differential voltage requests from assertions and UI selections.
  const diffRequests: Array<{ key: string; a: string; b: string }> = [];
  const collectDiff = (expr: string) => {
    const match = /^v\(([^(),]+),\s*([^(),]+)\)$/i.exec(expr.trim());
    if (!match) return;
    const rawA = match[1].trim();
    const rawB = match[2].trim();
    const a = resolveNetName(rawA, schematic, schemNetMap, irPinsMap);
    const b = resolveNetName(rawB, schematic, schemNetMap, irPinsMap);
    const key = `v(${rawA}, ${rawB})`;
    if (!diffRequests.some((d) => normalizeSignalToken(d.key) === normalizeSignalToken(key))) {
      diffRequests.push({ key, a, b });
    }
    if ((a !== rawA || b !== rawB)) {
      const resolvedKey = `v(${a}, ${b})`;
      if (!diffRequests.some((d) => normalizeSignalToken(d.key) === normalizeSignalToken(resolvedKey))) {
        diffRequests.push({ key: resolvedKey, a, b });
      }
    }
  };
  for (const assertion of assertions) collectDiff(assertion.signal);
  for (const extra of extraSignals) collectDiff(extra);

  if (dataset.kind === 'ac') {
    const nextSignals: Record<string, { real: number[]; imaginary: number[] }> = { ...dataset.signals };
    const len = dataset.frequency_hz.length;

    const findAcKey = (net: string): string | null =>
      Object.keys(nextSignals).find((k) => k.toLowerCase() === net.toLowerCase()) ?? null;

    const hasAcKey = (target: string): boolean =>
      Object.keys(nextSignals).some((k) => normalizeSignalToken(k) === normalizeSignalToken(target));

    const getPhasors = (net: string): { real: number[]; imaginary: number[] } | null => {
      if (isGroundNetName(net)) {
        return { real: new Array<number>(len).fill(0), imaginary: new Array<number>(len).fill(0) };
      }
      const key = findAcKey(net);
      if (!key) return null;
      const complex = nextSignals[key];
      if (!complex || complex.real.length !== len || complex.imaginary.length !== len) return null;
      return complex;
    };

    for (const comp of components) {
      const kind = componentKindName(comp.kind).toLowerCase();
      if (kind !== 'resistor') continue;
      const r = extractComponentNumericValue(comp);
      if (r === null || !Number.isFinite(r) || r <= 0) continue;
      const pins = getComponentPins(comp.id, kind);
      if (!pins) continue;
      const a = getPhasors(pins.posNet);
      const b = getPhasors(pins.negNet);
      if (!a || !b) continue;
      const iKey = `i(${comp.id})`;
      if (hasAcKey(iKey)) continue;
      const real = new Array<number>(len);
      const imaginary = new Array<number>(len);
      for (let i = 0; i < len; i += 1) {
        real[i] = (a.real[i] - b.real[i]) / r;
        imaginary[i] = (a.imaginary[i] - b.imaginary[i]) / r;
      }
      nextSignals[iKey] = { real, imaginary };
    }

    for (const diff of diffRequests) {
      if (hasAcKey(diff.key)) continue;
      const a = getPhasors(diff.a);
      const b = getPhasors(diff.b);
      if (!a || !b) continue;
      const real = new Array<number>(len);
      const imaginary = new Array<number>(len);
      for (let i = 0; i < len; i += 1) {
        real[i] = a.real[i] - b.real[i];
        imaginary[i] = a.imaginary[i] - b.imaginary[i];
      }
      nextSignals[diff.key] = { real, imaginary };
    }

    return { ...dataset, signals: nextSignals };
  }

  if (dataset.kind === 'operating_point') {
    const nextValues: Record<string, number> = { ...dataset.values };
    const existingKeys = () => Object.keys(nextValues);
    const hasKey = (target: string) =>
      existingKeys().some((k) => normalizeSignalToken(k) === normalizeSignalToken(target));
    const readScalarVoltage = (net: string): number | null => {
      if (isGroundNetName(net)) return 0;
      const hit = existingKeys().find((k) => k.toLowerCase() === net.toLowerCase());
      return hit !== undefined ? nextValues[hit] : null;
    };

    for (const comp of components) {
      const kind = componentKindName(comp.kind).toLowerCase();
      const pins = getComponentPins(comp.id, kind);
      if (!pins) continue;
      const va = readScalarVoltage(pins.posNet);
      const vb = readScalarVoltage(pins.negNet);
      if (va === null || vb === null) continue;
      const dv = va - vb;

      if (kind === 'resistor') {
        const r = extractComponentNumericValue(comp);
        if (r !== null && Number.isFinite(r) && r > 0) {
          const current = dv / r;
          const iKey = `i(${comp.id})`;
          const pKey = `p(${comp.id})`;
          if (!hasKey(iKey)) nextValues[iKey] = current;
          if (!hasKey(pKey)) nextValues[pKey] = dv * current;
        }
      } else if (BRANCH_POWER_KINDS.has(kind)) {
        const currentKey = findComponentCurrentSignalKey(comp.id, kind, existingKeys());
        if (currentKey) {
          const pKey = `p(${comp.id})`;
          if (!hasKey(pKey)) nextValues[pKey] = dv * nextValues[currentKey];
        }
      }
    }

    for (const diff of diffRequests) {
      if (hasKey(diff.key)) continue;
      const va = readScalarVoltage(diff.a);
      const vb = readScalarVoltage(diff.b);
      if (va !== null && vb !== null) nextValues[diff.key] = va - vb;
    }

    return { ...dataset, values: nextValues };
  }

  // Transient or DC sweep waveform vectors
  const len = dataset.axis.values.length;
  const nextSignals: Record<string, number[]> = { ...dataset.signals };
  const existingKeys = () => Object.keys(nextSignals);
  const hasKey = (target: string) =>
    existingKeys().some((k) => normalizeSignalToken(k) === normalizeSignalToken(target));
  const readVectorVoltage = (net: string): number[] | null => {
    if (isGroundNetName(net)) return new Array<number>(len).fill(0);
    const hit = existingKeys().find((k) => k.toLowerCase() === net.toLowerCase());
    if (!hit) return null;
    const series = nextSignals[hit];
    return series.length === len ? series : null;
  };

  for (const comp of components) {
    const kind = componentKindName(comp.kind).toLowerCase();
    const pins = getComponentPins(comp.id, kind);
    if (!pins) continue;
    const va = readVectorVoltage(pins.posNet);
    const vb = readVectorVoltage(pins.negNet);
    if (!va || !vb) continue;

    if (kind === 'resistor') {
      const r = extractComponentNumericValue(comp);
      if (r !== null && Number.isFinite(r) && r > 0) {
        const iVec = new Array<number>(len);
        const pVec = new Array<number>(len);
        for (let idx = 0; idx < len; idx += 1) {
          const dv = va[idx] - vb[idx];
          const current = dv / r;
          iVec[idx] = current;
          pVec[idx] = dv * current;
        }
        const iKey = `i(${comp.id})`;
        const pKey = `p(${comp.id})`;
        if (!hasKey(iKey)) nextSignals[iKey] = iVec;
        if (!hasKey(pKey)) nextSignals[pKey] = pVec;
      }
    } else if (BRANCH_POWER_KINDS.has(kind)) {
      const currentKey = findComponentCurrentSignalKey(comp.id, kind, existingKeys());
      const iVec = currentKey ? nextSignals[currentKey] : null;
      if (iVec && iVec.length === len) {
        const pKey = `p(${comp.id})`;
        if (!hasKey(pKey)) {
          const pVec = new Array<number>(len);
          for (let idx = 0; idx < len; idx += 1) {
            pVec[idx] = (va[idx] - vb[idx]) * iVec[idx];
          }
          nextSignals[pKey] = pVec;
        }
      }
    }
  }

  for (const diff of diffRequests) {
    if (hasKey(diff.key)) continue;
    const va = readVectorVoltage(diff.a);
    const vb = readVectorVoltage(diff.b);
    if (!va || !vb) continue;
    const diffVec = new Array<number>(len);
    for (let idx = 0; idx < len; idx += 1) {
      diffVec[idx] = va[idx] - vb[idx];
    }
    nextSignals[diff.key] = diffVec;
  }

  return { ...dataset, signals: nextSignals };
}


