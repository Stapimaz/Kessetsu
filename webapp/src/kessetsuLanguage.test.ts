import { describe, expect, it } from 'vitest';
import {
  COMPONENT_TYPES,
  PIN_MAP,
  analysisCompletions,
  componentTypes,
  completionsForLine,
  extractDeclarations,
  getConnectPinCompletions,
  getPinCompletions,
  hoverDocumentation,
  kessetsuLanguageDefinition,
  metricCompletions,
  topLevelCompletions,
  waveformCompletions,
} from './kessetsuLanguage';

describe('Kessetsu editor language support', () => {
  it('covers every public declaration and model boundary', () => {
    const labels = topLevelCompletions.map((item) => item.label);
    expect(labels).toEqual(expect.arrayContaining([
      'param', 'net', 'source', 'current_source', 'resistor', 'capacitor', 'inductor',
      'vcvs', 'vccs', 'ccvs', 'cccs',
      'diode', 'transistor', 'mosfet', 'opamp', 'connect', 'simulate', 'assert',
      'model_include', 'model', 'subcircuit', 'external_subcircuit', 'module', 'use',
    ]));
    expect(topLevelCompletions.find((item) => item.label === 'mosfet')?.insertText)
      .toBe('mosfet ${1:M1} ${2:IRF540}');
    expect(topLevelCompletions.find((item) => item.label === 'vcvs')?.insertText)
      .toBe('vcvs ${1:E1} ${2:1}');
    expect(topLevelCompletions.find((item) => item.label === 'vccs')?.insertText)
      .toBe('vccs ${1:G1} ${2:1mS}');
    expect(topLevelCompletions.find((item) => item.label === 'ccvs')?.insertText)
      .toBe('ccvs ${1:H1} ${2:1kOhm}');
    expect(topLevelCompletions.find((item) => item.label === 'cccs')?.insertText)
      .toBe('cccs ${1:F1} ${2:1}');
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
      'thd', 'clipping', 'gain_at', 'lower_cutoff', 'upper_cutoff',
      'rise_time', 'fall_time', 'settling_time', 'overshoot', 'energy',
    ]));
  });

  describe('Dependent sources (vcvs, vccs, ccvs, cccs)', () => {
    it('defines COMPONENT_TYPES entries with expected details and documentation', () => {
      expect(COMPONENT_TYPES.vcvs).toEqual(expect.objectContaining({
        label: 'vcvs',
        detail: 'vcvs <name> <gain>',
        documentation: 'Voltage-controlled voltage source (SPICE E). Pins: p_out, n_out, ctrl_p, ctrl_n.',
      }));
      expect(COMPONENT_TYPES.vccs).toEqual(expect.objectContaining({
        label: 'vccs',
        detail: 'vccs <name> <gm>',
        documentation: 'Voltage-controlled current source (SPICE G, in S/mS). Pins: p_out, n_out, ctrl_p, ctrl_n.',
      }));
      expect(COMPONENT_TYPES.ccvs).toEqual(expect.objectContaining({
        label: 'ccvs',
        detail: 'ccvs <name> <rm>',
        documentation: 'Current-controlled voltage source (SPICE H, in Ohm/kOhm). Pins: p_out, n_out, ctrl_p, ctrl_n.',
      }));
      expect(COMPONENT_TYPES.cccs).toEqual(expect.objectContaining({
        label: 'cccs',
        detail: 'cccs <name> <gain>',
        documentation: 'Current-controlled current source (SPICE F). Pins: p_out, n_out, ctrl_p, ctrl_n.',
      }));
    });

    it('defines 4-pin mappings in PIN_MAP for all linear dependent sources', () => {
      const expectedPins = ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'];
      expect(PIN_MAP.vcvs).toEqual(expectedPins);
      expect(PIN_MAP.vccs).toEqual(expectedPins);
      expect(PIN_MAP.ccvs).toEqual(expectedPins);
      expect(PIN_MAP.cccs).toEqual(expectedPins);
    });

    it('extracts declarations for dependent sources', () => {
      const sourceCode = `
        vcvs E1 10
        vccs G1 2mS
        ccvs H1 500
        cccs F1 0.5
        resistor R1 1k
      `;
      const decls = extractDeclarations(sourceCode);
      expect(decls.get('E1')).toBe('vcvs');
      expect(decls.get('G1')).toBe('vccs');
      expect(decls.get('H1')).toBe('ccvs');
      expect(decls.get('F1')).toBe('cccs');
      expect(decls.get('R1')).toBe('resistor');
    });

    it('provides pin completions for connect statements with dependent sources', () => {
      const code = `
        vcvs E_AMP 100
        vccs G_TRANS 5mS
        resistor R1 1k
      `;
      const decls = extractDeclarations(code);
      const pins = getPinCompletions('E_AMP', decls).map((c) => c.label);
      expect(pins).toEqual(['p_out', 'n_out', 'ctrl_p', 'ctrl_n']);

      const pinCompletions = completionsForLine('connect E_AMP.', code);
      expect(pinCompletions.map((c) => c.label)).toEqual(['p_out', 'n_out', 'ctrl_p', 'ctrl_n']);

      const directPinCompletions = getConnectPinCompletions('connect E_AMP.', code);
      expect(directPinCompletions?.map((c) => c.label)).toEqual(['p_out', 'n_out', 'ctrl_p', 'ctrl_n']);

      const targetPinCompletions = completionsForLine('connect R1.p1 to G_TRANS.', code);
      expect(targetPinCompletions.map((c) => c.label)).toEqual(['p_out', 'n_out', 'ctrl_p', 'ctrl_n']);
    });

    it('includes dependent sources in componentTypes and monarch token provider', () => {
      expect(componentTypes).toEqual(expect.arrayContaining(['vcvs', 'vccs', 'ccvs', 'cccs']));

      const rootRules = kessetsuLanguageDefinition.tokenizer.root as Array<[RegExp, string]>;
      const typeRule = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'type');
      expect(typeRule).toBeDefined();
      const typeRegex = typeRule![0];
      expect(typeRegex.test('vcvs')).toBe(true);
      expect(typeRegex.test('vccs')).toBe(true);
      expect(typeRegex.test('ccvs')).toBe(true);
      expect(typeRegex.test('cccs')).toBe(true);
    });

    it('supports Siemens unit highlighting (S, mS, uS, µS)', () => {
      const rootRules = kessetsuLanguageDefinition.tokenizer.root as Array<[RegExp, string]>;
      const numberRule = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'number');
      expect(numberRule).toBeDefined();
      const numRegex = numberRule![0];

      expect(numRegex.test('10S')).toBe(true);
      expect(numRegex.test('1mS')).toBe(true);
      expect(numRegex.test('50uS')).toBe(true);
      expect(numRegex.test('2µS')).toBe(true);
      expect(numRegex.test('0.5S')).toBe(true);
    });
  });
});
