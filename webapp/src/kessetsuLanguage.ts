export type KessetsuCompletion = {
  label: string;
  detail: string;
  insertText: string;
};

export const topLevelCompletions: readonly KessetsuCompletion[] = [
  { label: 'param', detail: 'Declare a typed quantity in the root or module scope; use {name} in numeric component values.', insertText: 'param ${1:supply}: ${2|V,A,Ohm,F,H,Hz,s,W,ratio,percent,deg|} = ${3:12V}' },
  { label: 'net', detail: 'Declare a named electrical net.', insertText: 'net ${1:GND}' },
  { label: 'source', detail: 'Declare an independent voltage source.', insertText: 'source ${1:VIN} ${2:5V}' },
  { label: 'current_source', detail: 'Declare an independent current source.', insertText: 'current_source ${1:I1} ${2:1mA}' },
  { label: 'resistor', detail: 'Declare a two-terminal resistor.', insertText: 'resistor ${1:R1} ${2:1k}' },
  { label: 'capacitor', detail: 'Declare a two-terminal capacitor.', insertText: 'capacitor ${1:C1} ${2:100nF}' },
  { label: 'inductor', detail: 'Declare a two-terminal inductor.', insertText: 'inductor ${1:L1} ${2:10mH}' },
  { label: 'diode', detail: 'Declare a diode with an optional typed model.', insertText: 'diode ${1:D1} ${2:1N4148}' },
  { label: 'transistor', detail: 'Declare a BJT; an optional polarity hint must match its model.', insertText: 'transistor ${1:Q1} ${2:npn} ${3:2N3904}' },
  { label: 'mosfet', detail: 'Declare a MOSFET; polarity is determined by its model.', insertText: 'mosfet ${1:M1} ${2:IRF540}' },
  { label: 'opamp', detail: 'Declare a five-pin typed op-amp.', insertText: 'opamp ${1:U1} ${2:KESSETSU_OPAMP_V1}' },
  { label: 'connect', detail: 'Connect one or more canonical pins to a pin or named net.', insertText: 'connect ${1:VIN.plus} to ${2:IN}' },
  { label: 'simulate', detail: 'Add an OP, transient, AC, or DC analysis.', insertText: 'simulate ${1|op,tran,ac,dc|}' },
  { label: 'assert', detail: 'Add a typed executable engineering requirement.', insertText: 'assert ${1:peak}(${2:V(OUT)}) ${3:<} ${4:5V}' },
  { label: 'model_include', detail: 'Resolve an exact Kessetsu model package version.', insertText: 'model_include ${1:kessetsu_analog} ${2:1.0.0}' },
  { label: 'model', detail: 'Declare an allowlisted typed diode, BJT, or MOSFET model.', insertText: 'model ${1|diode,bjt,mosfet|} ${2:ModelName} ${3:version=1.0.0} ${4:license=MIT}' },
  { label: 'subcircuit', detail: 'Declare a safe parameterized op-amp subcircuit.', insertText: 'subcircuit opamp ${1:SafeOp} (in_p,in_n,vcc,vee,out) version=${2:1.0.0} license=${3:MIT} gain=${4:100k} bandwidth=${5:2MHz}' },
  { label: 'external_subcircuit', detail: 'Bind an exact user-owned external op-amp model by hash.', insertText: 'external_subcircuit opamp ${1:OPA197} (in_p,in_n,vcc,vee,out) file="${2:models/model.lib}" entry=${3:OPA197} sha256=${4:64_hex_digest} version="${5:version}" license="${6:vendor terms}" source="${7:source URL}" simulator=${8|ngspice,ngspice_ps|} redistribution=${9|prohibited,permitted|}' },
  { label: 'module', detail: 'Define reusable topology that is flattened before Circuit IR.', insertText: 'module ${1:divider}(${2:input},${3:output}) {\n\t${4:// declarations and connections}\n}' },
  { label: 'use', detail: 'Instantiate a declared module with independent defaults; optional named overrides: use RC X(cutoff=500Hz). Braced overrides use caller parameters.', insertText: 'use ${1:module_name} ${2:instance_name}' },
] as const;

export const analysisCompletions: readonly KessetsuCompletion[] = [
  { label: 'op', detail: 'DC operating point.', insertText: 'op' },
  { label: 'tran', detail: 'Transient analysis: step and stop time.', insertText: 'tran ${1:10us} ${2:5ms}' },
  { label: 'ac', detail: 'AC analysis: scale, points, start, and stop frequency.', insertText: 'ac ${1|dec,oct,lin|} ${2:30} ${3:10Hz} ${4:10MHz}' },
  { label: 'dc', detail: 'Sweep one independent source.', insertText: 'dc ${1:VIN} ${2:-1V} ${3:1V} ${4:10mV}' },
] as const;

export const metricCompletions: readonly KessetsuCompletion[] = [
  { label: 'value', detail: 'Final scalar or series sample.', insertText: 'value(${1:V(OUT)}) ${2:>} ${3:0V}' },
  { label: 'min', detail: 'Signed minimum, optionally within a transient window.', insertText: 'min(${1:V(OUT)}) ${2:>} ${3:-5V}' },
  { label: 'max', detail: 'Signed maximum, optionally within a transient window.', insertText: 'max(${1:V(OUT)}) ${2:<} ${3:5V}' },
  { label: 'peak', detail: 'Absolute peak, optionally within a transient window.', insertText: 'peak(${1:V(OUT)}) ${2:<} ${3:5V}' },
  { label: 'average', detail: 'Arithmetic mean, optionally within a transient window.', insertText: 'average(${1:V(OUT)}) ${2:<} ${3:1V}' },
  { label: 'avg', detail: 'Alias of average.', insertText: 'avg(${1:V(OUT)}) ${2:<} ${3:1V}' },
  { label: 'rms', detail: 'Root mean square, optionally within a transient window.', insertText: 'rms(${1:V(OUT)},${2:2ms},${3:10ms}) ${4:>} ${5:1V}' },
  { label: 'gain', detail: 'Output-to-input RMS or complex AC magnitude ratio.', insertText: 'gain(${1:V(OUT)},${2:V(IN)}) ${3:>} ${4:1}' },
  { label: 'gain_at', detail: 'AC magnitude ratio at an exact frequency; interpolates along log frequency without extrapolation.', insertText: 'gain_at(${1:V(OUT)},${2:V(IN)},${3:1kHz}) ${4:>} ${5:1}' },
  { label: 'lower_cutoff', detail: 'Lower half-power edge of the band around an optional reference frequency. Multiple bands require a reference.', insertText: 'lower_cutoff(${1:V(OUT)},${2:V(IN)},${3:1kHz}) ${4:<} ${5:20Hz}' },
  { label: 'upper_cutoff', detail: 'Upper half-power edge of the band around an optional reference frequency. Multiple bands require a reference.', insertText: 'upper_cutoff(${1:V(OUT)},${2:V(IN)},${3:1kHz}) ${4:>} ${5:20kHz}' },
  { label: 'bandwidth', detail: 'Low-pass -3 dB bandwidth.', insertText: 'bandwidth(${1:V(OUT)},${2:V(IN)}) ${3:>} ${4:10kHz}' },
  { label: 'cutoff', detail: 'Alias of low-pass bandwidth.', insertText: 'cutoff(${1:V(OUT)},${2:V(IN)}) ${3:<} ${4:100kHz}' },
  { label: 'frequency', detail: 'Dominant transient frequency.', insertText: 'frequency(${1:V(OUT)}) ${2:>} ${3:990Hz}' },
  { label: 'phase', detail: 'Output/input AC phase at a frequency.', insertText: 'phase(${1:V(OUT)},${2:V(IN)},${3:1kHz}) ${4:>} ${5:-46deg}' },
  { label: 'output_power', detail: 'Load RMS power, optionally within a transient window.', insertText: 'output_power(${1:V(OUT)},${2:RL},${3:2ms},${4:10ms}) ${5:>} ${6:1W}' },
  { label: 'dissipation', detail: 'Positive device dissipation, optionally within a transient window.', insertText: 'dissipation(${1:Q1},${2:2ms},${3:10ms}) ${4:<} ${5:1W}' },
  { label: 'efficiency', detail: 'Output power divided by supply input power.', insertText: 'efficiency(${1:V(OUT)},${2:RL},${3:V(VCC)},${4:I(VP)},${5:V(VEE)},${6:I(VN)},${7:2ms},${8:10ms}) ${9:>} ${10:50%}' },
  { label: 'thd', detail: 'Windowed total harmonic distortion.', insertText: 'thd(${1:V(OUT)},${2:1kHz},${3:2ms},${4:10ms},${5:hann}) ${6:<} ${7:3%}' },
  { label: 'clipping', detail: 'Fraction of samples outside lower and upper limits.', insertText: 'clipping(${1:V(OUT)},${2:-10V},${3:10V}) ${4:<} ${5:0.1%}' },
] as const;

export const waveformCompletions: readonly KessetsuCompletion[] = [
  { label: 'DC value', detail: 'Constant source value.', insertText: '${1:5V}' },
  { label: 'ac', detail: 'Small-signal AC amplitude.', insertText: 'ac(${1:1V})' },
  { label: 'sine', detail: 'Sine offset, amplitude, and frequency.', insertText: 'sine(${1:0V},${2:1V},${3:1kHz})' },
  { label: 'sine_ac', detail: 'Sine transient plus small-signal AC amplitude.', insertText: 'sine_ac(${1:0V},${2:1V},${3:1kHz},${4:1V})' },
  { label: 'pulse', detail: 'Pulse low, high, delay, rise, fall, width, and period.', insertText: 'pulse(${1:0V},${2:5V},${3:0s},${4:1us},${5:1us},${6:500us},${7:1ms})' },
  { label: 'pwl', detail: 'Piecewise-linear time/value pairs.', insertText: 'pwl(${1:0s},${2:0V},${3:1ms},${4:5V})' },
] as const;

const hoverEntries = [
  ...topLevelCompletions,
  ...analysisCompletions,
  ...metricCompletions,
  ...waveformCompletions.filter((item) => item.label !== 'DC value'),
  { label: 'dec', detail: 'Logarithmic AC points per decade.' },
  { label: 'oct', detail: 'Logarithmic AC points per octave.' },
  { label: 'lin', detail: 'Linearly spaced AC points.' },
  { label: 'npn', detail: 'NPN BJT polarity hint or typed model polarity.' },
  { label: 'pnp', detail: 'PNP BJT polarity hint or typed model polarity.' },
  { label: 'nmos', detail: 'NMOS polarity for a typed MOSFET model declaration.' },
  { label: 'pmos', detail: 'PMOS polarity for a typed MOSFET model declaration.' },
] as const;

export const hoverDocumentation = new Map<string, string>();
for (const item of hoverEntries) {
  const existing = hoverDocumentation.get(item.label);
  hoverDocumentation.set(item.label, existing ? `${existing}\n\n${item.detail}` : item.detail);
}

export function completionsForLine(lineBeforeCursor: string): readonly KessetsuCompletion[] {
  const line = lineBeforeCursor.trimStart();
  if (/^simulate\s+[^\s]*$/i.test(line)) return analysisCompletions;
  if (/^assert\s+[A-Za-z_]*$/i.test(line)) return metricCompletions;
  if (/^(?:source|current_source)\s+[A-Za-z_][A-Za-z0-9_]*\s+[^\s]*$/i.test(line)) {
    return waveformCompletions;
  }
  return topLevelCompletions;
}
