import { describe, expect, it } from 'vitest';
import {
  nearestSampleIndex,
  preferredSignal,
  signalLabel,
  signalUnit,
  summarizeAcSeries,
  summarizeWaveformSeries,
  sweepAxis,
  zoomAroundPointer,
} from './plot';
import { mergeConvertedKessIntoSource } from '../spiceImport';
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

describe('quick measurement summaries and SPICE merge preview', () => {
  it('summarizes AC peak, -3 dB cutoff, and phase', () => {
    const freqs = [10, 100, 1000, 10000];
    const mag = [0, -0.04, -3.0103, -20];
    const phase = [-0.5, -5.7, -45, -84.3];
    const summary = summarizeAcSeries(freqs, mag, phase)!;
    expect(summary.peakIndex).toBe(0);
    expect(summary.peakMagnitudeDb).toBe(0);
    expect(summary.cutoffFrequencyHz).toBeCloseTo(1000, 0);
    expect(summary.phaseAtCutoffOrPeakDeg).toBeCloseTo(-45, 1);
    expect(summary.minIndex).toBe(3);
  });

  it('summarizes transient min, max, peak-to-peak, average, and RMS', () => {
    const summary = summarizeWaveformSeries([0, 1, 2, 3], [-2, 0, 2, 0])!;
    expect(summary.minValue).toBe(-2);
    expect(summary.maxValue).toBe(2);
    expect(summary.peakToPeak).toBe(4);
    expect(summary.average).toBe(0);
    expect(summary.rms).toBeCloseTo(Math.SQRT2, 6);
  });

  it('merges converted SPICE into existing .kess without duplicating nets or analyses', () => {
    const existing = 'net GND\nnet in\nsource V1 ac(1V)\nsimulate ac dec 40 10Hz 100kHz\n';
    const converted = '// Imported from a supported SPICE subset by Kessetsu.\n// Original source: sha256:abc\nnet GND\nnet in\nnet out\nresistor Rload 1e3\nconnect Rload.p1 to in\nconnect Rload.p2 to out\nsimulate ac dec 20 10Hz 100kHz\n';
    const merged = mergeConvertedKessIntoSource(existing, converted);
    expect(merged.match(/^net GND$/gm)).toHaveLength(1);
    expect(merged.match(/^net in$/gm)).toHaveLength(1);
    expect(merged).toContain('net out');
    expect(merged).toContain('resistor Rload 1e3');
    expect(merged.match(/^simulate ac\b/gm)).toHaveLength(1);
  });
});

