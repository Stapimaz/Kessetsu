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
  diode: ['p1', 'p2'],
  device: ['p1', 'p2'],
  transistor: ['b', 'c', 'e'],
  bjt: ['b', 'c', 'e'],
  mosfet: ['g', 'd', 's'],
  opamp: ['in_p', 'in_n', 'vcc', 'vee', 'out'],
  comparator: ['in_p', 'in_n', 'vcc', 'vee', 'out'],
  vcvs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  vccs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  ccvs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
  cccs: ['p_out', 'n_out', 'ctrl_p', 'ctrl_n'],
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
    documentation: 'Declare an independent voltage source.',
    insertText: 'source ${1:VIN} ${2:5V}',
  },
  {
    label: 'current_source',
    detail: 'current_source <name> <value>',
    documentation: 'Declare an independent current source.',
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
    label: 'opamp',
    detail: 'opamp <name> <model>',
    documentation: 'Declare a five-pin typed op-amp.',
    insertText: 'opamp ${1:U1} ${2:KESSETSU_OPAMP_V1}',
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
  const declMatch = /^\s*(resistor|capacitor|inductor|diode|transistor|bjt|mosfet|opamp|source|current_source|vcvs|vccs|ccvs|cccs)\s+([A-Za-z_][A-Za-z0-9_]*)/i;

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
