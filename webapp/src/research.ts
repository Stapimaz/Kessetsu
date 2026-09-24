export const researchUnits = [
  'Volt', 'Ampere', 'Second', 'Hertz', 'Ohm', 'Farad', 'Henry',
  'Watt', 'Joule', 'Ratio', 'Percent', 'Degree',
] as const;

export type ResearchUnit = typeof researchUnits[number];
export type Delimiter = 'comma' | 'semicolon' | 'tab';
export type Decimal = 'dot' | 'comma';
export type Origin = 'measured' | 'published_simulation' | 'synthetic' | 'unspecified';

export interface CsvDialect {
  delimiter: Delimiter;
  decimal: Decimal;
  header: boolean;
  preamble_records: number;
}

export interface CsvPreview {
  schema_version: 'kessetsu.csv-preview.v1';
  raw_sha256: string;
  headers: string[];
  truncated_headers: number[];
  sample: Array<{ record: number; line: number; fields: string[]; truncated_fields: number[] }>;
  data_records: number;
  blank_records: number;
}

export interface ColumnMapping {
  column: number;
  name: string;
  unit: ResearchUnit;
  source_unit: string;
  gain: number;
  offset: number;
}

export interface ResearchData {
  schema_version: 'kessetsu.research-data.v1';
  identity: string;
  raw_sha256: string;
  raw_csv: string;
  spec: {
    schema_version: 'kessetsu.data-import.v1';
    name: string;
    file_name: string;
    dialect: CsvDialect;
    metadata: {
      origin: Origin;
      device: string | null;
      sample: string | null;
      citation: string | null;
      conditions: Record<string, string>;
    };
    axis: ColumnMapping;
    axis_order: 'increasing' | 'decreasing';
    signals: ColumnMapping[];
    missing: 'error' | 'skip_row';
    missing_tokens: string[];
  };
  axis: { name: string; unit: ResearchUnit; values: number[] };
  signals: Array<{ name: string; unit: ResearchUnit; values: number[] }>;
  source_records: number[];
  skipped: Array<{ record: number; line: number; reason: string }>;
  records_seen: number;
}

export interface DataComparison {
  schema_version: 'kessetsu.data-comparison.v1';
  identity: string;
  data_identity: string;
  reference_identity: string;
  data_origin: Origin;
  reference_origin: Origin;
  axis_unit: ResearchUnit;
  spec: ComparisonDraft & {
    schema_version: 'kessetsu.data-comparison.v1';
    reference_axis_shift: number;
    window: [number, number] | null;
    max_reference_gap: number | null;
  };
  signals: Array<{
    mapping: SignalPair;
    unit: ResearchUnit;
    metrics: {
      total: number;
      matched: number;
      excluded_by_window: number;
      unmatched: number;
      bias: number;
      mae: number;
      rmse: number;
      max_absolute: number;
    };
    points: Array<{
      source_record: number;
      axis: number;
      observed: number;
      predicted: number | null;
      residual: number | null;
      status: string;
    }>;
  }>;
}

export interface SignalPair { data_signal: string; reference_signal: string }

export interface ComparisonDraft {
  name: string;
  signals: SignalPair[];
  coverage: 'require_full' | 'overlap_only';
  interpolation: 'linear' | 'log_axis';
}

export interface ImportDraft {
  name: string;
  origin: Origin;
  device: string;
  sample: string;
  citation: string;
  axisOrder: 'increasing' | 'decreasing';
  axis: ColumnMapping;
  signals: ColumnMapping[];
  missing: 'error' | 'skip_row';
  missingTokens: string;
}

export interface DatasetSlot {
  file: File | null;
  csv: string;
  dialect: CsvDialect;
  preview: CsvPreview | null;
  draft: ImportDraft;
  dataset: ResearchData | null;
  error: string;
}

export function sourceUnit(unit: ResearchUnit): string {
  return ({
    Volt: 'V', Ampere: 'A', Second: 's', Hertz: 'Hz', Ohm: 'Ohm', Farad: 'F',
    Henry: 'H', Watt: 'W', Joule: 'J', Ratio: '', Percent: '%', Degree: 'deg',
  } as Record<ResearchUnit, string>)[unit];
}

export function logicalName(value: string, fallback: string): string {
  const normalized = value.trim().replace(/[^A-Za-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '');
  return normalized || fallback;
}

export function column(columnIndex: number, name: string, unit: ResearchUnit): ColumnMapping {
  return { column: columnIndex, name, unit, source_unit: sourceUnit(unit), gain: 1, offset: 0 };
}

export function emptySlot(label: string): DatasetSlot {
  return {
    file: null,
    csv: '',
    dialect: { delimiter: 'comma', decimal: 'dot', header: true, preamble_records: 0 },
    preview: null,
    draft: {
      name: label,
      origin: 'unspecified',
      device: '', sample: '', citation: '', axisOrder: 'increasing',
      axis: column(0, 'time', 'Second'),
      signals: [column(1, 'signal', 'Volt')],
      missing: 'error', missingTokens: ',NA,N/A',
    },
    dataset: null,
    error: '',
  };
}

export function mappingSpec(slot: DatasetSlot) {
  if (!slot.file) throw new Error('Choose a CSV file first');
  const tokens = slot.draft.missingTokens.split(',').map((token) => token.trim());
  return {
    schema_version: 'kessetsu.data-import.v1',
    name: slot.draft.name,
    file_name: slot.file.name,
    dialect: slot.dialect,
    metadata: {
      origin: slot.draft.origin,
      device: slot.draft.device.trim() || null,
      sample: slot.draft.sample.trim() || null,
      citation: slot.draft.citation.trim() || null,
      conditions: {},
    },
    axis: slot.draft.axis,
    axis_order: slot.draft.axisOrder,
    signals: slot.draft.signals,
    missing: slot.draft.missing,
    missing_tokens: tokens,
  };
}

export function parseOptional(value: string, label: string): number | null {
  if (!value.trim()) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) throw new Error(`${label} must be a finite number`);
  return parsed;
}

export function engineering(value: number): string {
  if (value === 0) return '0';
  const magnitude = Math.abs(value);
  if (magnitude >= 1e-3 && magnitude < 1e4) return value.toPrecision(6).replace(/\.?0+$/, '');
  return value.toExponential(5);
}
