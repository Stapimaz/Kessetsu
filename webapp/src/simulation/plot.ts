import type { Analysis, AssertionResult } from './types';

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

export function signalUnit(name: string): 'A' | 'V' {
  return name.includes('#branch') || /^@.+\[i[bcdesg]?\]$/i.test(name) ? 'A' : 'V';
}

export function signalLabel(name: string): string {
  const deviceCurrent = /^@([dqm])_(.+)\[(ic|id)\]$/i.exec(name);
  if (deviceCurrent) return `${deviceCurrent[2]} ${deviceCurrent[3].toLowerCase() === 'ic' ? 'collector' : deviceCurrent[1].toLowerCase() === 'm' ? 'drain' : 'diode'} current`;
  return signalUnit(name) === 'A' ? `I(${name.replace('#branch', '').replace(/^[vli]_/, '')})` : `V(${name})`;
}

export function preferredSignal(names: string[], assertions: AssertionResult[]): string {
  const namedVoltage = (name: string) => signalUnit(name) === 'V' && !name.includes('.')
    && !name.startsWith('n_') && !/^(0|gnd|vcc|vee|vdd|vss)$/i.test(name);
  const asserted = assertions.flatMap((assertion) => {
    const node = /^v\(([^(),]+)\)/i.exec(assertion.signal.trim())?.[1];
    return node ? names.filter((name) => name.toLowerCase() === node.toLowerCase()) : [];
  });
  return names.find((name) => /^(out|output)$/i.test(name))
    ?? asserted.find(namedVoltage) ?? names.find(namedVoltage) ?? names[0] ?? '';
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

