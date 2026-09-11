const SCOPES = Object.freeze({
  U1: {
    operating_conditions: ['Ideal 1 V source; 100 kohm load; nominal AC sweep from 10 Hz to 1 MHz.'],
    model_assumptions: ['Ideal source, resistor, and capacitor behavior.'],
    untested_effects: ['Component tolerance and temperature drift.', 'Parasitics, noise, transient behavior, and hardware measurements.']
  },
  U2: {
    operating_conditions: ['+/-6 V supplies; 10 kohm load; 50 mV-peak at 100 Hz; OP, 10 Hz-1 MHz AC, and settled transient analyses.'],
    model_assumptions: ['One/two-section RC input ladder and a declared finite-gain single-pole linear op-amp template.'],
    untested_effects: ['Rail saturation, output-current limits, slew rate, supply current, noise, tolerance, temperature, and hardware behavior.']
  },
  U3: {
    operating_conditions: ['9 V supply; 10 kohm AC-coupled load; 5 mV-peak at 1 kHz; nominal OP, AC, and settled transient analyses.'],
    model_assumptions: ['Exact frozen generic 2N3904 model; divider bias; unbypassed emitter degeneration.'],
    untested_effects: ['Manufacturer-lot variation, tolerance, temperature, noise, clipping margin beyond the fixed stimulus, and hardware behavior.']
  },
  U4: {
    operating_conditions: ['12 V supply; 120 ohm load; 0-10 V pulse; final four periods with 50 us excluded around every transition.'],
    model_assumptions: ['Exact frozen generic IRF540 VDMOS model and a direct or bounded resistive gate network.'],
    untested_effects: ['Total switching loss, gate-driver loss, tolerance, temperature, package/SOA/avalanche limits, EMC, and hardware behavior.']
  },
  U5: {
    operating_conditions: ['+/-9 V supplies; 4 ohm load; 100 mV-peak at 1 kHz; 10-30 ms nominal transient integration.'],
    model_assumptions: ['Exact generic linear op-amp and power-BJT models in the bounded buffer/gain/error-driver/complementary-output topology.'],
    untested_effects: ['Driver supply power and total efficiency.', 'Rail/current/slew limits, bias crossover realism, tolerance, thermal/package/SOA behavior, manufacturability, and hardware measurements.']
  },
  U6: {
    operating_conditions: ['+/-6 V supplies; 10 kohm load; 100 mV-peak at 1 kHz; OP, 10 Hz-1 MHz AC, and settled transient analyses.'],
    model_assumptions: ['Exact hash-verified TI OPAx197 Rev. D / Final 1.3 macromodel supplied locally by the user.'],
    untested_effects: ['Component tolerance, board parasitics, thermal behavior, EMC, production variation, and hardware measurements.']
  }
});

export function verificationScope(task, checks) {
  const boundary = SCOPES[task];
  if (!boundary || !checks || Object.values(checks).some((value) => typeof value !== 'boolean')) {
    throw new Error('Invalid evaluation verification scope');
  }
  return {
    schema_version: 'kessetsu.evaluation-verification-scope.v1',
    specification_id: `unseen-design-v1/${task}`,
    conclusion: Object.values(checks).every(Boolean) ? 'requirements_satisfied_within_scope' : 'requirements_not_satisfied_within_scope',
    requirement_checks: Object.keys(checks).sort(),
    hardware_validated: false,
    ...boundary
  };
}

export const verificationTasks = Object.freeze(Object.keys(SCOPES));
