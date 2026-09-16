import { describe, expect, it } from 'vitest';
import {
  analysisCompletions,
  completionsForLine,
  hoverDocumentation,
  metricCompletions,
  topLevelCompletions,
  waveformCompletions,
} from './kessetsuLanguage';

describe('Kessetsu editor language support', () => {
  it('covers every public declaration and model boundary', () => {
    const labels = topLevelCompletions.map((item) => item.label);
    expect(labels).toEqual(expect.arrayContaining([
      'param', 'net', 'source', 'current_source', 'resistor', 'capacitor', 'inductor',
      'diode', 'transistor', 'mosfet', 'opamp', 'connect', 'simulate', 'assert',
      'model_include', 'model', 'subcircuit', 'external_subcircuit', 'module', 'use',
    ]));
    expect(topLevelCompletions.find((item) => item.label === 'mosfet')?.insertText)
      .toBe('mosfet ${1:M1} ${2:IRF540}');
  });

  it('offers analyses, metrics, and waveforms only in their useful line contexts', () => {
    expect(completionsForLine('simulate ')).toBe(analysisCompletions);
    expect(completionsForLine('  assert cu')).toBe(metricCompletions);
    expect(completionsForLine('source VIN si')).toBe(waveformCompletions);
    expect(completionsForLine('res')).toBe(topLevelCompletions);
  });

  it('documents all supported analyses and assertion metrics', () => {
    for (const item of [...analysisCompletions, ...metricCompletions]) {
      expect(hoverDocumentation.get(item.label)).toContain(item.detail);
    }
    expect(metricCompletions.map((item) => item.label)).toEqual(expect.arrayContaining([
      'value', 'min', 'max', 'peak', 'average', 'avg', 'rms', 'gain', 'bandwidth',
      'cutoff', 'frequency', 'phase', 'output_power', 'dissipation', 'efficiency',
      'thd', 'clipping',
    ]));
  });
});
