import { describe, expect, it } from 'vitest';
import { nearestSampleIndex, preferredSignal, signalLabel, signalUnit, sweepAxis, zoomAroundPointer } from './plot';
import type { AssertionResult } from './types';

describe('physical plot coordinates', () => {
  it('selects a physical sample, not a normalized array index, including reversed and sparse axes', () => {
    expect(nearestSampleIndex([0, 0.000001, 0.9, 1], 0.5)).toBe(2);
    expect(nearestSampleIndex([5, 4, 1, 0], 2)).toBe(2);
    expect(nearestSampleIndex([5, 4, 1, 0], 6)).toBe(0);
    expect(nearestSampleIndex([0, 0, 1], 0)).toBe(0);
    expect(nearestSampleIndex([7], 1)).toBe(0);
    expect(nearestSampleIndex([], 1)).toBeNull();
  });
  it('selects logarithmic samples using displayed log distance', () => {
    const coordinates = [10, 100, 1000, 1e6].map(Math.log10);
    expect(nearestSampleIndex(coordinates, 2.2)).toBe(1);
    expect(nearestSampleIndex(coordinates, 4.6)).toBe(3);
  });
  it('keeps the pointer anchor on physical zoom and preserves width at edges', () => {
    const zoom: [number, number] = [0.2, 0.8];
    const fraction = 0.25;
    const next = zoomAroundPointer(zoom, fraction, false);
    expect(next[0] + fraction * (next[1] - next[0])).toBeCloseTo(0.35);
    const edge = zoomAroundPointer([0.8, 1], 1, true);
    expect(edge).toEqual([0.75, 1]);
    expect(zoomAroundPointer([0, 1], 0.5, true)).toEqual([0, 1]);
  });
});

it('uses typed sweep units and chooses external voltages before internal/current vectors', () => {
  expect(sweepAxis({ kind: 'dc_sweep', source: 'I1', start: { value: 0, unit: 'Ampere' }, stop: { value: 1, unit: 'Ampere' }, step: { value: 0.1, unit: 'Ampere' } })).toEqual({ label: 'Sweep of I1', unit: 'A' });
  const assertions: AssertionResult[] = [{ code: 'KES-T001', metric: 'max', signal: 'V(SENSE)', comparator: '<', threshold: 2, actual: 1, unit: 'volt', status: 'PASS' }];
  expect(preferredSignal(['x.u1.internal', 'v_vin#branch', 'vcc', 'sense', 'input'], assertions)).toBe('sense');
  expect(preferredSignal(['x.u1.internal', 'vcc', 'input'], [])).toBe('input');
  expect(preferredSignal(['v_vin#branch', 'OUT'], [])).toBe('OUT');
  expect(signalUnit('@q_q1[ic]')).toBe('A');
  expect(signalLabel('@q_q1[ic]')).toBe('q1 collector current');
  expect(signalUnit('sense')).toBe('V');
  expect(signalLabel('v_vin#branch')).toBe('I(vin)');
});
