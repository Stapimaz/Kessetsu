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
