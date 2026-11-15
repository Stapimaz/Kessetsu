import { describe, expect, it } from 'vitest';
import {
  BUILTIN_MODELS,
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
      'param', 'net', 'initial', 'source', 'current_source', 'resistor', 'capacitor', 'inductor',
      'vcvs', 'vccs', 'ccvs', 'cccs', 'switch',
      'diode', 'transistor', 'mosfet', 'jfet', 'opamp', 'connect', 'simulate', 'assert',
      'model_include', 'model', 'subcircuit', 'external_subcircuit', 'module', 'use',
    ]));
    expect(topLevelCompletions.find((item) => item.label === 'switch')?.insertText)
      .toBe('switch ${1:S1} ${2:KESSETSU_SW_V1}');
    expect(topLevelCompletions.find((item) => item.label === 'model')?.insertText)
      .toBe('model ${1|diode,bjt,mosfet,jfet,switch|} ${2:ModelName} ${3:version=1.0.0} ${4:license=MIT}');
    expect(topLevelCompletions.find((item) => item.label === 'model')?.detail)
      .toBe('Declare an allowlisted typed diode, BJT, MOSFET, JFET, or switch model.');
    expect(topLevelCompletions.find((item) => item.label === 'mosfet')?.insertText)
      .toBe('mosfet ${1:M1} ${2:IRF540}');
    expect(topLevelCompletions.find((item) => item.label === 'jfet')?.insertText)
      .toBe('jfet ${1:J1} ${2:2N5457}');
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
      'value', 'min', 'max', 'peak', 'pk2pk', 'p2p', 'vpp', 'average', 'avg', 'rms', 'gain', 'bandwidth',
      'cutoff', 'frequency', 'freq', 'period', 'duty_cycle', 'duty', 'phase', 'output_power', 'dissipation', 'efficiency',
      'thd', 'clipping', 'gain_at', 'lower_cutoff', 'upper_cutoff',
      'rise_time', 'fall_time', 'settling_time', 'overshoot', 'energy',
    ]));
    const rootRules = kessetsuLanguageDefinition.tokenizer.root as Array<[RegExp, string]>;
    const keywordRegex = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'keyword')![0];
    const functionRegex = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'function')![0];
    expect(keywordRegex.test('initial')).toBe(true);
    for (const metric of ['freq', 'period', 'duty_cycle', 'duty']) {
      expect(functionRegex.test(metric)).toBe(true);
    }
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

  describe('Voltage-controlled switch (switch)', () => {
    it('defines COMPONENT_TYPES entry with expected details and documentation', () => {
      expect(COMPONENT_TYPES.switch).toEqual(expect.objectContaining({
        label: 'switch',
        detail: 'switch <name> [model]',
        documentation: 'Voltage-controlled switch (SPICE S). Pins: p1, p2, ctrl_p, ctrl_n.',
        insertText: 'switch ${1:S1} ${2:KESSETSU_SW_V1}',
      }));
    });

    it('defines 4-pin mapping in PIN_MAP for switch', () => {
      expect(PIN_MAP.switch).toEqual(['p1', 'p2', 'ctrl_p', 'ctrl_n']);
    });

    it('extracts declarations for switch', () => {
      const sourceCode = `
        switch S1 KESSETSU_SW_V1
        resistor R1 1k
      `;
      const decls = extractDeclarations(sourceCode);
      expect(decls.get('S1')).toBe('switch');
      expect(decls.get('R1')).toBe('resistor');
    });

    it('provides pin completions for connect statements with switch', () => {
      const code = `
        switch S1 KESSETSU_SW_V1
        resistor R1 1k
      `;
      const decls = extractDeclarations(code);
      const pins = getPinCompletions('S1', decls).map((c) => c.label);
      expect(pins).toEqual(['p1', 'p2', 'ctrl_p', 'ctrl_n']);

      const pinCompletions = completionsForLine('connect S1.', code);
      expect(pinCompletions.map((c) => c.label)).toEqual(['p1', 'p2', 'ctrl_p', 'ctrl_n']);

      const targetPinCompletions = completionsForLine('connect R1.p1 to S1.', code);
      expect(targetPinCompletions.map((c) => c.label)).toEqual(['p1', 'p2', 'ctrl_p', 'ctrl_n']);
    });

    it('includes switch in componentTypes and monarch token provider', () => {
      expect(componentTypes).toContain('switch');

      const rootRules = kessetsuLanguageDefinition.tokenizer.root as Array<[RegExp, string]>;
      const typeRule = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'type');
      expect(typeRule).toBeDefined();
      const typeRegex = typeRule![0];
      expect(typeRegex.test('switch')).toBe(true);
    });
  });

  describe('Junction field-effect transistor (jfet)', () => {
    it('defines COMPONENT_TYPES entry with expected details and documentation', () => {
      expect(COMPONENT_TYPES.jfet).toEqual(expect.objectContaining({
        label: 'jfet',
        detail: 'jfet <name> <model>',
        documentation: 'Declare a JFET; polarity is determined by its model.',
        insertText: 'jfet ${1:J1} ${2:2N5457}',
      }));
    });

    it('defines 3-pin mapping in PIN_MAP for jfet', () => {
      expect(PIN_MAP.jfet).toEqual(['d', 'g', 's']);
    });

    it('extracts declarations for jfet', () => {
      const sourceCode = `
        jfet J1 2N5457
        resistor R1 1k
      `;
      const decls = extractDeclarations(sourceCode);
      expect(decls.get('J1')).toBe('jfet');
      expect(decls.get('R1')).toBe('resistor');
    });

    it('provides pin completions for connect statements with jfet', () => {
      const code = `
        jfet J1 2N5457
        resistor R1 1k
      `;
      const decls = extractDeclarations(code);
      const pins = getPinCompletions('J1', decls).map((c) => c.label);
      expect(pins).toEqual(['d', 'g', 's']);

      const pinCompletions = completionsForLine('connect J1.', code);
      expect(pinCompletions.map((c) => c.label)).toEqual(['d', 'g', 's']);

      const targetPinCompletions = completionsForLine('connect R1.p1 to J1.', code);
      expect(targetPinCompletions.map((c) => c.label)).toEqual(['d', 'g', 's']);
    });

    it('includes jfet in componentTypes and monarch token provider', () => {
      expect(componentTypes).toContain('jfet');

      const rootRules = kessetsuLanguageDefinition.tokenizer.root as Array<[RegExp, string]>;
      const typeRule = rootRules.find((rule) => Array.isArray(rule) && rule[1] === 'type');
      expect(typeRule).toBeDefined();
      const typeRegex = typeRule![0];
      expect(typeRegex.test('jfet')).toBe(true);
    });
  });

  describe('Phase 6 built-in model, initial condition, and assertion signal completions', () => {
    it('offers new built-in models by device kind and exposes hover docs', () => {
      const diodeLabels = completionsForLine('diode D1 ').map((c) => c.label);
      expect(diodeLabels).toEqual(expect.arrayContaining(['LED_RED', 'LED_GREEN', 'LED_BLUE', 'LED_WHITE', '1N5819']));

      const bjtNpn = completionsForLine('transistor Q1 npn ').map((c) => c.label);
      expect(bjtNpn).toContain('BC547');
      expect(bjtNpn).not.toContain('BC557');
      const bjtPnp = completionsForLine('transistor Q2 pnp ').map((c) => c.label);
      expect(bjtPnp).toContain('BC557');

      const mosLabels = completionsForLine('mosfet M1 ').map((c) => c.label);
      expect(mosLabels).toEqual(expect.arrayContaining(['2N7000', 'BSS84', 'IRF9540']));

      expect(completionsForLine('jfet J1 ').map((c) => c.label)).toContain('J201');
      expect(completionsForLine('opamp U1 ').map((c) => c.label)).toEqual(expect.arrayContaining(['LM358', 'TL072']));

      for (const expected of [
        'LED_RED', 'LED_GREEN', 'LED_BLUE', 'LED_WHITE', '1N5819', '1N4733A', 'BZX84C3V3', 'BZX84C5V1', 'BZX84C12',
        'BC547', 'BC557', 'KESSETSU_POWER_NPN_V1', 'KESSETSU_POWER_PNP_V1',
        '2N7000', 'BSS84', 'IRF9540', 'KESSETSU_PMOS_V1',
        'J201', '2N5457', 'J2N5457', '2N5460',
        'KESSETSU_OPAMP_V1', 'KESSETSU_OPAMP_V2', 'LM358', 'TL072',
      ]) {
        expect(BUILTIN_MODELS.some((m) => m.name === expected)).toBe(true);
        expect(hoverDocumentation.get(expected)).toBeTruthy();
      }
      expect(BUILTIN_MODELS).toHaveLength(32);

      const sourceDoc = COMPONENT_TYPES.find((c) => c.label === 'source')?.documentation ?? '';
      const currDoc = COMPONENT_TYPES.find((c) => c.label === 'current_source')?.documentation ?? '';
      expect(sourceDoc).toContain('exp');
      expect(currDoc).toContain('exp');

      expect(PIN_MAP.diode).toEqual(expect.arrayContaining(['p1', 'p2', 'anode', 'cathode']));
    });

    it('completes net names for initial conditions and signals inside assert metrics', () => {
      const code = [
        'net in',
        'net out',
        'source VIN 5V',
        'resistor R1 1k',
        'connect VIN.plus to in',
        'connect R1.p1 to in',
        'connect R1.p2 to out',
      ].join('\n');

      const initNets = completionsForLine('initial V(', code).map((c) => c.label);
      expect(initNets).toEqual(expect.arrayContaining(['in', 'out']));

      const initSnippets = completionsForLine('initial ', code).map((c) => c.label);
      expect(initSnippets).toEqual(expect.arrayContaining(['V(in)', 'V(out)', 'V(<net>)']));

      const assertSignals = completionsForLine('assert peak(', code).map((c) => c.label);
      expect(assertSignals).toEqual(expect.arrayContaining(['V(in)', 'V(out)', 'V(in, out)', 'I(R1)', 'P(R1)', 'I(VIN)']));

      expect(completionsForLine('assert max(V(', code).map((c) => c.label)).toEqual(['in', 'out']);
      expect(completionsForLine('assert avg(I(', code).map((c) => c.label)).toEqual(expect.arrayContaining(['VIN', 'R1']));
      expect(completionsForLine('assert dissipation(', code).map((c) => c.label)).toEqual(expect.arrayContaining(['VIN', 'R1']));
    });
  });
});
