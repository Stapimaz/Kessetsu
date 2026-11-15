export interface ComponentTypeDetail {
  label: string;
  detail: string;
  documentation: string;
  insertText: string;
}

export interface KessetsuCompletion {
  label: string;
  detail: string;
  documentation?: string;
  insertText: string;
}

export const PIN_MAP: Record<string, readonly string[]> = {
  resistor: ['p1', 'p2'],
  capacitor: ['p1', 'p2'],
  inductor: ['p1', 'p2'],
  source: ['plus', 'minus'],
  current_source: ['plus', 'minus'],
  diode: ['p1', 'p2', 'anode', 'cathode'],
  device: ['p1', 'p2'],
  transistor: ['b', 'c', 'e'],
  bjt: ['b', 'c', 'e'],
  mosfet: ['d', 'g', 's', 'b'],
  jfet: ['d', 'g', 's'],
  opamp: ['in_p', 'in_n', 'vcc', 'vee', 'out'],
  comparator: ['in_p', 'in_n', 'vcc', 'vee', 'out'],
  vcvs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  vccs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  ccvs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  cccs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  switch: ['p1', 'p2', 'ctrl_p', 'ctrl_n'],
};

const componentList: readonly ComponentTypeDetail[] = [
  {
    label: 'resistor',
    detail: 'resistor <name> <value>',
    documentation: 'Declare a two-terminal resistor.',
    insertText: 'resistor ${1:R1} ${2:1k}',
  },
  {
    label: 'capacitor',
    detail: 'capacitor <name> <value>',
    documentation: 'Declare a two-terminal capacitor.',
    insertText: 'capacitor ${1:C1} ${2:100nF}',
  },
  {
    label: 'inductor',
    detail: 'inductor <name> <value>',
    documentation: 'Declare a two-terminal inductor.',
    insertText: 'inductor ${1:L1} ${2:10mH}',
  },
  {
    label: 'source',
    detail: 'source <name> <value>',
    documentation: 'Declare an independent voltage source (DC scalar, ac(amplitude), biased ac(amplitude, dc_offset), sine, sine_ac, pulse, exp, pwl).',
    insertText: 'source ${1:VIN} ${2:5V}',
  },
  {
    label: 'current_source',
    detail: 'current_source <name> <value>',
    documentation: 'Declare an independent current source (DC scalar, ac(amplitude), biased ac(amplitude, dc_offset), sine, sine_ac, pulse, exp, pwl).',
    insertText: 'current_source ${1:I1} ${2:1mA}',
  },
  {
    label: 'vcvs',
    detail: 'vcvs <name> <gain>',
    documentation: 'Voltage-controlled voltage source (SPICE E). Pins: p_out, n_out, ctrl_p, ctrl_n.',
    insertText: 'vcvs ${1:E1} ${2:1}',
  },
  {
    label: 'vccs',
    detail: 'vccs <name> <gm>',
    documentation: 'Voltage-controlled current source (SPICE G, in S/mS). Pins: p_out, n_out, ctrl_p, ctrl_n.',
    insertText: 'vccs ${1:G1} ${2:1mS}',
  },
  {
    label: 'ccvs',
    detail: 'ccvs <name> <rm>',
    documentation: 'Current-controlled voltage source (SPICE H, in Ohm/kOhm). Pins: p_out, n_out, ctrl_p, ctrl_n.',
    insertText: 'ccvs ${1:H1} ${2:1kOhm}',
  },
  {
    label: 'cccs',
    detail: 'cccs <name> <gain>',
    documentation: 'Current-controlled current source (SPICE F). Pins: p_out, n_out, ctrl_p, ctrl_n.',
    insertText: 'cccs ${1:F1} ${2:1}',
  },
  {
    label: 'diode',
    detail: 'diode <name> <model>',
    documentation: 'Declare a diode with an optional typed model.',
    insertText: 'diode ${1:D1} ${2:1N4148}',
  },
  {
    label: 'transistor',
    detail: 'transistor <name> <polarity> <model>',
    documentation: 'Declare a BJT; an optional polarity hint must match its model.',
    insertText: 'transistor ${1:Q1} ${2:npn} ${3:2N3904}',
  },
  {
    label: 'mosfet',
    detail: 'mosfet <name> <model>',
    documentation: 'Declare a MOSFET; polarity is determined by its model.',
    insertText: 'mosfet ${1:M1} ${2:IRF540}',
  },
  {
    label: 'jfet',
    detail: 'jfet <name> <model>',
    documentation: 'Declare a JFET; polarity is determined by its model.',
    insertText: 'jfet ${1:J1} ${2:2N5457}',
  },
  {
    label: 'opamp',
    detail: 'opamp <name> <model>',
    documentation: 'Declare a five-pin typed op-amp.',
    insertText: 'opamp ${1:U1} ${2:KESSETSU_OPAMP_V1}',
  },
  {
    label: 'switch',
    detail: 'switch <name> [model]',
    documentation: 'Voltage-controlled switch (SPICE S). Pins: p1, p2, ctrl_p, ctrl_n.',
    insertText: 'switch ${1:S1} ${2:KESSETSU_SW_V1}',
  },
];

export type ComponentTypesCollection = readonly ComponentTypeDetail[] & {
  readonly [key: string]: ComponentTypeDetail;
};

function createComponentTypes(): ComponentTypesCollection {
  const byLabel = Object.fromEntries(componentList.map((item) => [item.label, item]));
  return Object.freeze(Object.assign([...componentList], byLabel)) as ComponentTypesCollection;
}

export const COMPONENT_TYPES: ComponentTypesCollection = createComponentTypes();

export interface Declaration {
  name: string;
  kind: string;
}

/**
 * Extract component declarations from source text to enable intelligent pin completion.
 * Lines matching component declaration syntax map instance identifiers to their component kinds.
 */
export function extractDeclarations(code: string): Map<string, string> {
  const declarations = new Map<string, string>();
  if (!code) return declarations;
  const lines = code.split(/\r?\n/);

  // Line 141 area: Matches component declarations for pin autocompletion
  const declMatch = /^\s*(resistor|capacitor|inductor|diode|transistor|bjt|mosfet|jfet|opamp|source|current_source|vcvs|vccs|ccvs|cccs|switch)\s+([A-Za-z_][A-Za-z0-9_]*)/i;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('//')) continue;
    const match = trimmed.match(declMatch);
    if (match) {
      const kind = match[1].toLowerCase();
      const name = match[2];
      declarations.set(name, kind);
    }
  }
  return declarations;
}

/**
 * Generate pin completions for a specific declared component instance.
 */
export function getPinCompletions(
  componentName: string,
  declarations: Map<string, string>
): readonly KessetsuCompletion[] {
  const kind = declarations.get(componentName);
  if (!kind) return [];
  const pins = PIN_MAP[kind];
  if (!pins) return [];
  return pins.map((pin) => ({
    label: pin,
    detail: `Pin ${pin} (${componentName}: ${kind})`,
    documentation: `Pin '${pin}' for component '${componentName}' of kind '${kind}'.`,
    insertText: pin,
  }));
}

/**
 * Check if the text before cursor is requesting a pin for a component in a connect statement.
 * Supports patterns like `connect U1.` or `connect U1.out to R1.`
 */
export function getConnectPinCompletions(
  lineBeforeCursor: string,
  code: string
): readonly KessetsuCompletion[] | null {
  const match = /(?:connect\s+|to\s+|^)([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z0-9_]*)$/i.exec(lineBeforeCursor);
  if (!match) return null;
  const componentName = match[1];
  const declarations = extractDeclarations(code);
  const completions = getPinCompletions(componentName, declarations);
  return completions.length > 0 ? completions : null;
}

/**
 * Standard waveform completions for independent sources, including small-signal AC
 * and biased small-signal AC waveforms.
 */
export const waveformCompletions: readonly KessetsuCompletion[] = [
  { label: 'DC value', detail: 'Constant source value.', insertText: '${1:5V}' },
  {
    label: 'ac',
    detail: 'Small-signal AC amplitude: ac(amplitude) or biased ac(amplitude, dc_offset).',
    documentation: 'Small-signal AC stimulus: ac(amplitude) or biased small-signal source ac(amplitude, dc_offset) mapping to SPICE `DC <offset> AC <amplitude>`.',
    insertText: 'ac(${1:1V})',
  },
  { label: 'sine', detail: 'Sine offset, amplitude, and frequency.', insertText: 'sine(${1:0V},${2:1V},${3:1kHz})' },
  { label: 'sine_ac', detail: 'Sine transient plus small-signal AC amplitude.', insertText: 'sine_ac(${1:0V},${2:1V},${3:1kHz},${4:1V})' },
  { label: 'pulse', detail: 'Pulse low, high, delay, rise, fall, width, and period.', insertText: 'pulse(${1:0V},${2:5V},${3:0s},${4:1us},${5:1us},${6:500us},${7:1ms})' },
  { label: 'exp', detail: 'Exponential waveform: exp(v1, v2, td1, tau1, td2, tau2).', insertText: 'exp(${1:0V},${2:5V},${3:0s},${4:1ms},${5:5ms},${6:2ms})' },
  { label: 'pwl', detail: 'Piecewise-linear time/value pairs.', insertText: 'pwl(${1:0s},${2:0V},${3:1ms},${4:5V})' },
];

export const WAVEFORM_COMPLETIONS = waveformCompletions;
export const COMPONENT_PINS = PIN_MAP;

export interface BuiltinModelEntry {
  name: string;
  kind: 'diode' | 'bjt' | 'mosfet' | 'jfet' | 'opamp' | 'switch';
  polarity?: 'npn' | 'pnp';
  detail: string;
}

export const BUILTIN_MODELS: readonly BuiltinModelEntry[] = [
  { name: '1N4148', kind: 'diode', detail: 'Fast small-signal silicon switching diode.' },
  { name: '1N4007', kind: 'diode', detail: '1000 V / 1 A silicon rectifier diode.' },
  { name: '1N5819', kind: 'diode', detail: '40 V / 1 A low-Vf Schottky barrier diode.' },
  { name: '1N4733A', kind: 'diode', detail: '5.1 V, 1 W Zener voltage reference diode.' },
  { name: 'BZX84C3V3', kind: 'diode', detail: '3.3 V SMD Zener voltage regulator diode.' },
  { name: 'BZX84C5V1', kind: 'diode', detail: '5.1 V SMD Zener voltage regulator diode.' },
  { name: 'BZX84C12', kind: 'diode', detail: '12 V SMD Zener voltage regulator diode.' },
  { name: 'LED_RED', kind: 'diode', detail: 'Standard red LED (~1.8–2.0 V forward drop).' },
  { name: 'LED_GREEN', kind: 'diode', detail: 'Standard green LED (~2.1 V forward drop).' },
  { name: 'LED_BLUE', kind: 'diode', detail: 'High-Vf blue LED (~3.0–3.2 V forward drop).' },
  { name: 'LED_WHITE', kind: 'diode', detail: 'White phosphor LED (~3.1 V forward drop).' },
  { name: '2N3904', kind: 'bjt', polarity: 'npn', detail: 'General-purpose NPN small-signal BJT.' },
  { name: '2N2222', kind: 'bjt', polarity: 'npn', detail: 'High-current NPN switching BJT.' },
  { name: 'BC547', kind: 'bjt', polarity: 'npn', detail: 'Low-noise NPN small-signal BJT.' },
  { name: 'KESSETSU_POWER_NPN_V1', kind: 'bjt', polarity: 'npn', detail: 'Kessetsu power NPN bipolar junction transistor.' },
  { name: '2N3906', kind: 'bjt', polarity: 'pnp', detail: 'General-purpose PNP small-signal BJT.' },
  { name: 'BC557', kind: 'bjt', polarity: 'pnp', detail: 'Low-noise PNP small-signal BJT.' },
  { name: 'KESSETSU_POWER_PNP_V1', kind: 'bjt', polarity: 'pnp', detail: 'Kessetsu power PNP bipolar junction transistor.' },
  { name: 'IRF540', kind: 'mosfet', detail: '100 V N-channel power MOSFET.' },
  { name: '2N7000', kind: 'mosfet', detail: '60 V N-channel small-signal MOSFET.' },
  { name: 'BSS84', kind: 'mosfet', detail: '50 V P-channel small-signal MOSFET.' },
  { name: 'IRF9540', kind: 'mosfet', detail: '100 V P-channel power MOSFET.' },
  { name: 'KESSETSU_PMOS_V1', kind: 'mosfet', detail: 'Kessetsu generic P-channel power MOSFET.' },
  { name: '2N5457', kind: 'jfet', detail: 'N-channel general-purpose JFET.' },
  { name: 'J2N5457', kind: 'jfet', detail: 'N-channel general-purpose JFET (2N5457 alias).' },
  { name: '2N5460', kind: 'jfet', detail: 'P-channel general-purpose JFET.' },
  { name: 'J201', kind: 'jfet', detail: 'Low-cutoff N-channel audio/analog JFET.' },
  { name: 'KESSETSU_OPAMP_V1', kind: 'opamp', detail: 'Idealized Kessetsu 5-pin operational amplifier.' },
  { name: 'KESSETSU_OPAMP_V2', kind: 'opamp', detail: 'Rail- and slew-limited Kessetsu 5-pin operational amplifier.' },
  { name: 'LM358', kind: 'opamp', detail: 'Single-supply dual operational amplifier macro model.' },
  { name: 'TL072', kind: 'opamp', detail: 'Low-noise JFET-input dual operational amplifier.' },
  { name: 'KESSETSU_SW_V1', kind: 'switch', detail: 'Default Kessetsu voltage-controlled switch.' },
];

export function extractNets(code: string): string[] {
  if (!code) return [];
  const decls = extractDeclarations(code);
  const nets = new Set<string>();
  const lines = code.split(/\r?\n/);

  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    if (!trimmed || trimmed.startsWith('//')) continue;

    const netMatch = /^net\s+([A-Za-z_][A-Za-z0-9_]*)\b/i.exec(trimmed);
    if (netMatch && !/^(0|gnd)$/i.test(netMatch[1])) {
      nets.add(netMatch[1]);
      continue;
    }

    const connectMatch = /^connect\s+(.+?)\s+to\s+([A-Za-z0-9_.]+)\s*$/i.exec(trimmed);
    if (connectMatch) {
      const lhsTokens = connectMatch[1].split(',').map((token) => token.trim());
      for (const token of [...lhsTokens, connectMatch[2].trim()]) {
        if (
          /^[A-Za-z_][A-Za-z0-9_]*$/.test(token)
          && !decls.has(token)
          && !/^(0|gnd)$/i.test(token)
        ) {
          nets.add(token);
        }
      }
    }
  }

  return [...nets];
}

export function getModelCompletions(lineBeforeCursor: string): readonly KessetsuCompletion[] | null {
  const line = lineBeforeCursor.trimStart();
  const bjtMatch = /^(?:transistor|bjt)\s+[A-Za-z_][A-Za-z0-9_]*(?:\s+(npn|pnp))?\s+[A-Za-z0-9_]*$/i.exec(line);
  if (bjtMatch) {
    const polarityHint = bjtMatch[1]?.toLowerCase() as 'npn' | 'pnp' | undefined;
    const models = BUILTIN_MODELS
      .filter((m) => m.kind === 'bjt' && (!polarityHint || m.polarity === polarityHint))
      .map((m) => ({
        label: m.name,
        detail: `Built-in ${m.polarity?.toUpperCase() ?? 'BJT'} model`,
        documentation: m.detail,
        insertText: m.name,
      }));
    if (!polarityHint) {
      return [
        { label: 'npn', detail: 'NPN BJT polarity hint', insertText: 'npn ${1:2N3904}' },
        { label: 'pnp', detail: 'PNP BJT polarity hint', insertText: 'pnp ${1:2N3906}' },
        ...models,
      ];
    }
    return models;
  }

  const deviceMatch = /^(diode|mosfet|jfet|opamp|switch)\s+[A-Za-z_][A-Za-z0-9_]*\s+[A-Za-z0-9_]*$/i.exec(line);
  if (!deviceMatch) return null;
  const kind = deviceMatch[1].toLowerCase() as BuiltinModelEntry['kind'];
  return BUILTIN_MODELS
    .filter((m) => m.kind === kind)
    .map((m) => ({
      label: m.name,
      detail: `Built-in ${kind} model`,
      documentation: m.detail,
      insertText: m.name,
    }));
}

export function getInitialCompletions(
  lineBeforeCursor: string,
  fullCode = '',
): readonly KessetsuCompletion[] | null {
  const line = lineBeforeCursor.trimStart();
  if (/^initial\s+v\(\s*[A-Za-z0-9_]*$/i.test(line)) {
    const nets = extractNets(fullCode);
    const choices = nets.length > 0 ? nets : ['OUT', 'VCC'];
    return choices.map((net) => ({
      label: net,
      detail: `Initial condition net ${net}`,
      documentation: `Set initial voltage on net '${net}' for transient analysis.`,
      insertText: net,
    }));
  }
  if (/^initial\s+[A-Za-z0-9_]*$/i.test(line)) {
    const nets = extractNets(fullCode);
    const items: KessetsuCompletion[] = nets.map((net) => ({
      label: `V(${net})`,
      detail: `initial V(${net}) = 0V`,
      documentation: `Set initial transient voltage on net '${net}'.`,
      insertText: `V(${net}) = \${1:0V}`,
    }));
    items.push({
      label: 'V(<net>)',
      detail: 'initial V(<net>) = 0V',
      documentation: 'Set initial transient node voltage (.ic).',
      insertText: 'V(${1:OUT}) = ${2:0V}',
    });
    return items;
  }
  return null;
}

const CURRENT_CAPABLE_KINDS = new Set([
  'resistor', 'source', 'current_source', 'inductor', 'diode',
  'transistor', 'bjt', 'mosfet', 'jfet', 'vcvs', 'ccvs',
]);

export function getAssertSignalCompletions(
  lineBeforeCursor: string,
  fullCode = '',
): readonly KessetsuCompletion[] | null {
  const line = lineBeforeCursor.trimStart();
  const assertMatch = /^assert\s+([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)$/i.exec(line);
  if (!assertMatch) return null;

  const metricName = assertMatch[1].toLowerCase();
  const innerBefore = assertMatch[2];
  const nets = extractNets(fullCode);
  const decls = extractDeclarations(fullCode);
  const components = [...decls.entries()]
    .filter(([, kind]) => CURRENT_CAPABLE_KINDS.has(kind))
    .map(([name]) => name);

  if (/\bV\(\s*(?:[A-Za-z0-9_]+\s*,\s*)?[A-Za-z0-9_]*$/i.test(innerBefore)) {
    const choices = nets.length > 0 ? nets : ['OUT', 'IN'];
    return choices.map((net) => ({
      label: net,
      detail: `Net ${net}`,
      documentation: `Node voltage net '${net}'.`,
      insertText: net,
    }));
  }

  if (/\b[IP]\(\s*[A-Za-z0-9_]*$/i.test(innerBefore) || (metricName === 'dissipation' && !innerBefore.includes(','))) {
    const choices = components.length > 0 ? components : ['R1', 'VIN'];
    return choices.map((comp) => ({
      label: comp,
      detail: `Component ${comp}`,
      documentation: `Reference component '${comp}'.`,
      insertText: comp,
    }));
  }

  const completions: KessetsuCompletion[] = [];
  for (const net of nets.length > 0 ? nets : ['OUT', 'IN']) {
    completions.push({
      label: `V(${net})`,
      detail: `Node voltage on ${net}`,
      insertText: `V(${net})`,
    });
  }
  if (nets.length >= 2) {
    completions.push({
      label: `V(${nets[0]}, ${nets[1]})`,
      detail: 'Differential node voltage V(A, B)',
      insertText: `V(${nets[0]}, ${nets[1]})`,
    });
  } else {
    completions.push({
      label: 'V(A, B)',
      detail: 'Differential node voltage V(A, B)',
      insertText: 'V(${1:A}, ${2:B})',
    });
  }
  for (const comp of components.length > 0 ? components : ['R1']) {
    completions.push({
      label: `I(${comp})`,
      detail: `Current through ${comp}`,
      insertText: `I(${comp})`,
    });
    completions.push({
      label: `P(${comp})`,
      detail: `Power dissipation of ${comp}`,
      insertText: `P(${comp})`,
    });
  }
  return completions;
}

