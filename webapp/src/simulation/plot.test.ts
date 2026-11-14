import { describe, expect, it } from 'vitest';
import type { CircuitIrSummary, SchematicSummary } from '../domain';
import {
  augmentDatasetWithDerivedSignals,
  findLiteralComponentValueSpan,
  formatKessQuantity,
  nearestSampleIndex,
  parseKessQuantity,
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

describe('derived signals, units, labels, and quick-tweak value spans', () => {
  it('formats JFET, dependent source branch, and derived I/P/differential labels and units', () => {
    expect(signalUnit('@j_j1[id]')).toBe('A');
    expect(signalLabel('@j_j1[id]')).toBe('j1 drain current');
    expect(signalUnit('e_e1#branch')).toBe('A');
    expect(signalLabel('e_e1#branch')).toBe('I(e1)');
    expect(signalLabel('h_h1#branch')).toBe('I(h1)');
    expect(signalLabel('v_sense_h1#branch')).toBe('I(h1)');
    expect(signalUnit('i(R1)')).toBe('A');
    expect(signalLabel('i(R1)')).toBe('I(R1)');
    expect(signalUnit('p(R1)')).toBe('W');
    expect(signalLabel('p(R1)')).toBe('P(R1)');
    expect(signalUnit('v(in, out)')).toBe('V');
    expect(signalLabel('v(in, out)')).toBe('V(in, out)');
  });

  it('prefers asserted I/P/differential traces while keeping OUT priority intact', () => {
    const currAssert: AssertionResult[] = [{
      code: 'KES-T001', metric: 'max', signal: 'I(R1)', comparator: '<', threshold: 0.02, actual: 0.01, unit: 'ampere', status: 'PASS',
    }];
    expect(preferredSignal(['in', 'i(R1)', 'p(R1)'], currAssert)).toBe('i(R1)');
    expect(preferredSignal(['in', 'out', 'i(R1)'], currAssert)).toBe('out');

    const diffAssert: AssertionResult[] = [{
      code: 'KES-T002', metric: 'max', signal: 'V(a, b)', comparator: '<', threshold: 5, actual: 2, unit: 'volt', status: 'PASS',
    }];
    expect(preferredSignal(['a', 'b', 'v(a, b)'], diffAssert)).toBe('v(a, b)');

    const powerAssert: AssertionResult[] = [{
      code: 'KES-T003', metric: 'max', signal: 'P(R1)', comparator: '<', threshold: 0.5, actual: 0.1, unit: 'watt', status: 'PASS',
    }];
    expect(preferredSignal(['in', 'p(R1)'], powerAssert)).toBe('p(R1)');
  });

  it('augments operating point, transient, and AC datasets with derived R current/power, branch power, and V(A, B)', () => {
    const ir: CircuitIrSummary = {
      analyses: [{ kind: 'operating_point' }],
      assertions: [{ metric: 'max', signal: 'V(in, out)' }],
      components: [
        { id: 'R1', kind: 'Resistor', parameters: { TwoPinPassive: { value: { value: 1000, unit: 'Ohm' } } } },
        { id: 'V1', kind: 'VoltageSource', parameters: { VoltageSource: { value: { Dc: { value: 5, unit: 'Volt' } } } } },
      ],
      model_manifest: { schema_version: '1', packages: [], models: [] },
    };
    const schematic: SchematicSummary = {
      schema_version: '1',
      connectivity: { verified: true },
      quality: { passed: true, issues: [] },
      nets: [
        { id: 1, name: 'in', kind: 'signal', pins: [] },
        { id: 2, name: 'out', kind: 'signal', pins: [] },
        { id: 0, name: '0', kind: 'ground', pins: [] },
      ],
      components: [
        { id: 'R1', reference: 'R1', instance_parameters: {}, pins: [{ name: 'p1', net: 1 }, { name: 'p2', net: 2 }] },
        { id: 'V1', reference: 'V1', instance_parameters: {}, pins: [{ name: 'p1', net: 1 }, { name: 'p2', net: 0 }] },
      ],
    };

    const op = augmentDatasetWithDerivedSignals(
      { kind: 'operating_point', values: { in: 5, out: 2, 'v_v1#branch': -0.003 } },
      ir,
      schematic,
    );
    expect(op.kind).toBe('operating_point');
    if (op.kind === 'operating_point') {
      expect(op.values['i(R1)']).toBeCloseTo(0.003, 9);
      expect(op.values['p(R1)']).toBeCloseTo(0.009, 9);
      expect(op.values['p(V1)']).toBeCloseTo(-0.015, 9);
      expect(op.values['v(in, out)']).toBeCloseTo(3, 9);
    }

    const tran = augmentDatasetWithDerivedSignals(
      { kind: 'transient', axis: { name: 'time', values: [0, 1e-3] }, signals: { in: [5, 10], out: [2, 4], 'v_v1#branch': [-0.003, -0.006] } },
      ir,
      schematic,
    );
    expect(tran.kind).toBe('transient');
    if (tran.kind === 'transient') {
      expect(tran.signals['i(R1)'][0]).toBeCloseTo(0.003, 9);
      expect(tran.signals['i(R1)'][1]).toBeCloseTo(0.006, 9);
      expect(tran.signals['p(R1)'][0]).toBeCloseTo(0.009, 9);
      expect(tran.signals['p(R1)'][1]).toBeCloseTo(0.036, 9);
      expect(tran.signals['v(in, out)']).toEqual([3, 6]);
    }

    const ac = augmentDatasetWithDerivedSignals(
      {
        kind: 'ac',
        frequency_hz: [100],
        signals: {
          in: { real: [1], imaginary: [0] },
          out: { real: [0.5], imaginary: [0] },
        },
      },
      ir,
      schematic,
    );
    expect(ac.kind).toBe('ac');
    if (ac.kind === 'ac') {
      expect(ac.signals['p(R1)']).toBeUndefined();
      // V(in) - V(out) = 1 - 0.5 = 0.5 -> I(R1) = 0.5mA
      expect(ac.signals['v(in, out)'].real[0]).toBeCloseTo(0.5, 9);
      expect(ac.signals['i(R1)'].real[0]).toBeCloseTo(0.0005, 9);
    }
  });

  it('parses/formats Kessetsu quantities and locates top-level literal component spans safely', () => {
    expect(parseKessQuantity('4.7kOhm')?.value).toBeCloseTo(4700, 6);
    expect(parseKessQuantity('100nF')?.value).toBeCloseTo(1e-7, 15);
    expect(formatKessQuantity(1100, '')).toBe('1.1k');
    expect(formatKessQuantity(1.1e-7, 'F')).toBe('110nF');

    const source = [
      'module Stage(in, out) {',
      '  resistor R1 99k',
      '}',
      'source Vin sine(0, 5V, 1kHz)',
      'resistor R1 10k // load resistor',
      'capacitor C1 {cap_val}',
      'capacitor C2 100nF',
    ].join('\n');

    expect(findLiteralComponentValueSpan(source, 'Vin')).toBeNull();
    expect(findLiteralComponentValueSpan(source, 'C1')).toBeNull();

    const rSpan = findLiteralComponentValueSpan(source, 'R1')!;
    expect(rSpan).not.toBeNull();
    expect(rSpan.rawValue).toBe('10k');
    expect(source.slice(rSpan.start, rSpan.end)).toBe('10k');
    expect(source.slice(0, rSpan.start) + formatKessQuantity(rSpan.value * 1.1, rSpan.suffix) + source.slice(rSpan.end)).toContain('resistor R1 11k // load resistor');

    const cSpan = findLiteralComponentValueSpan(source, 'C2')!;
    expect(cSpan.suffix).toBe('F');
    expect(cSpan.value).toBeCloseTo(1e-7, 15);
  });
});


