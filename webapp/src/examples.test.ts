import { describe, expect, it, vi } from 'vitest';

vi.mock('kessetsu-core', () => ({
  default: vi.fn(),
  compile_kessetsu_with_resources: vi.fn(),
  compile_schema_version: vi.fn(),
  evaluate_browser_simulation_with_resources: vi.fn(),
  export_kessetsu_with_resources: vi.fn(),
  export_schema_version: vi.fn(),
  import_spice_netlist: vi.fn(),
  prepare_browser_simulation_with_resources: vi.fn(),
  local_model_requirements: vi.fn(),
  spice_import_schema_version: vi.fn(),
  supported_export_capabilities: vi.fn(),
}));

import { examples, type ExampleId } from './hooks/useKessetsuWorkspace';

describe('Web Hub Example Showcase Presets', () => {
  const expectedKeys: ExampleId[] = [
    'rc',
    'gain',
    'power',
    'filters',
    'amplifiers',
    'comparator',
    'memristor',
    'study_filter',
    'study_driver',
    'zener',
    'sample_hold',
    'current_sense',
  ];

  it('exposes all expected canonical presets including 1.5 showcases', () => {
    for (const key of expectedKeys) {
      expect(examples[key], `Missing example preset: ${key}`).toBeDefined();
      expect(examples[key].label.trim().length).toBeGreaterThan(0);
      expect(examples[key].description.trim().length).toBeGreaterThan(0);
      expect(examples[key].source.trim().length).toBeGreaterThan(0);
    }
  });

  it('ensures all example labels and sources are unique', () => {
    const labels = Object.values(examples).map((ex) => ex.label);
    const sources = Object.values(examples).map((ex) => ex.source);
    expect(new Set(labels).size).toBe(labels.length);
    expect(new Set(sources).size).toBe(sources.length);
  });

  it('includes built-in BZX84C5V1 Zener shunt regulator showcase', () => {
    const zener = examples.zener;
    expect(zener.label).toBe('Zener Regulator');
    expect(zener.source).toContain('diode DZ BZX84C5V1');
    expect(zener.source).toContain('simulate op');
    expect(zener.source).toContain('assert value(V(VREG)) > 4.9V');
    expect(zener.source).toContain('assert value(V(VREG)) < 5.3V');
  });

  it('includes voltage-controlled switch sample-and-hold showcase', () => {
    const sh = examples.sample_hold;
    expect(sh.label).toBe('Sample & Hold');
    expect(sh.source).toContain('switch S1');
    expect(sh.source).toContain('pulse(');
    expect(sh.source).toContain('capacitor CHOLD');
    expect(sh.source).toContain('simulate tran');
    expect(sh.source).toContain('assert value(V(OUT)) > 3.25V');
  });

  it('includes CCVS and KESSETSU_OPAMP_V2 current sense amplifier showcase', () => {
    const cs = examples.current_sense;
    expect(cs.label).toBe('Current Sense Amp');
    expect(cs.source).toContain('ccvs H1 100');
    expect(cs.source).toContain('opamp U1 KESSETSU_OPAMP_V2');
    expect(cs.source).toContain('assert value(V(SENSE)) > 0.99V');
    expect(cs.source).toContain('assert max(V(OUT)) > 10.7V');
    expect(cs.source).toContain('assert max(V(OUT)) < 11.0V');
  });
});
