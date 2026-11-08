import { CheckCircle2, Download, Plus, Trash2, Upload } from 'lucide-react';
import { downloadTextFile, sanitizeFileStem } from '../document';
import {
  column,
  engineering,
  researchUnits,
  sourceUnit,
  type ColumnMapping,
  type CsvDialect,
  type DataComparison,
  type DatasetSlot,
  type ResearchUnit,
} from '../research';

export function PrimaryMappingRow({
  mapping,
  headers,
  onChange,
  onRemove,
}: {
  mapping: ColumnMapping;
  headers: string[];
  onChange: (mapping: ColumnMapping) => void;
  onRemove?: () => void;
}) {
  const changeUnit = (unit: ResearchUnit) => onChange({ ...mapping, unit, source_unit: sourceUnit(unit) });
  return (
    <div className="research-mapping-row">
      <label>
        CSV column
        <select
          aria-label="CSV column"
          value={mapping.column}
          onChange={(event) => onChange({ ...mapping, column: Number(event.target.value) })}
        >
          {headers.map((header, index) => (
            <option key={index} value={index}>
              {index + 1}: {header || `Column ${index + 1}`}
            </option>
          ))}
        </select>
      </label>
      <label>
        Logical name
        <input
          aria-label="Logical name"
          value={mapping.name}
          onChange={(event) => onChange({ ...mapping, name: event.target.value })}
        />
      </label>
      <label>
        Quantity
        <select
          aria-label="Quantity"
          value={mapping.unit}
          onChange={(event) => changeUnit(event.target.value as ResearchUnit)}
        >
          {researchUnits.map((unit) => (
            <option key={unit} value={unit}>
              {unit}
            </option>
          ))}
        </select>
      </label>
      {onRemove && (
        <button
          type="button"
          className="research-icon-button"
          aria-label={`Remove signal ${mapping.name}`}
          onClick={onRemove}
        >
          <Trash2 size={14} />
        </button>
      )}
    </div>
  );
}

export function CalibrationRow({
  label,
  mapping,
  axis,
  onChange,
}: {
  label: string;
  mapping: ColumnMapping;
  axis?: boolean;
  onChange: (mapping: ColumnMapping) => void;
}) {
  return (
    <div className="research-calibration-row">
      <span className="research-calibration-target">
        {label} (<code>{mapping.name}</code>)
      </span>
      <label>
        CSV unit
        <input
          value={mapping.source_unit}
          aria-label={`${axis ? 'Axis' : 'Signal'} source unit`}
          onChange={(event) => onChange({ ...mapping, source_unit: event.target.value })}
        />
      </label>
      <label>
        Gain
        <input
          type="number"
          step="any"
          value={mapping.gain}
          onChange={(event) => onChange({ ...mapping, gain: Number(event.target.value) })}
        />
      </label>
      <label>
        Offset
        <input
          type="number"
          step="any"
          value={mapping.offset}
          onChange={(event) => onChange({ ...mapping, offset: Number(event.target.value) })}
        />
      </label>
    </div>
  );
}

function polyline(
  points: Array<{ axis: number; value: number }>,
  x: (value: number) => number,
  y: (value: number) => number
) {
  if (points.length > 2400) {
    const stride = Math.ceil(points.length / 2400);
    points = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
  }
  return points.map((point) => `${x(point.axis).toFixed(2)},${y(point.value).toFixed(2)}`).join(' ');
}

export function ComparisonPlot({ comparison, signalIndex }: { comparison: DataComparison; signalIndex: number }) {
  const signal = comparison.signals[signalIndex];
  if (!signal) return null;
  const matched = signal.points.filter((point) => point.predicted !== null);
  if (matched.length === 0) return <p className="research-note">No matched points are available to plot.</p>;
  const axes = matched.map((point) => point.axis);
  const values = matched.flatMap((point) => [point.observed, point.predicted!]);
  let minX = Math.min(...axes);
  let maxX = Math.max(...axes);
  let minY = Math.min(...values);
  let maxY = Math.max(...values);
  if (minX === maxX) {
    minX -= 0.5;
    maxX += 0.5;
  }
  if (minY === maxY) {
    const pad = Math.abs(minY) * 0.05 || 0.5;
    minY -= pad;
    maxY += pad;
  }
  const x = (value: number) => 62 + ((value - minX) / (maxX - minX)) * 786;
  const y = (value: number) => 220 - ((value - minY) / (maxY - minY)) * 184;
  const observed = matched.map((point) => ({ axis: point.axis, value: point.observed }));
  const predicted = matched.map((point) => ({ axis: point.axis, value: point.predicted! }));
  return (
    <figure className="research-plot">
      <svg viewBox="0 0 900 260" role="img" aria-label={`${signal.mapping.data_signal} observed and reference overlay`}>
        <rect x="62" y="36" width="786" height="184" />
        {[0, 0.25, 0.5, 0.75, 1].map((part) => (
          <g key={part}>
            <line x1="62" x2="848" y1={36 + part * 184} y2={36 + part * 184} />
            <text x="54" y={40 + part * 184} textAnchor="end">
              {engineering(maxY - part * (maxY - minY))}
            </text>
          </g>
        ))}
        <polyline className="research-observed" points={polyline(observed, x, y)} />
        <polyline className="research-reference" points={polyline(predicted, x, y)} />
        <text x="62" y="242">
          {engineering(minX)}
        </text>
        <text x="848" y="242" textAnchor="end">
          {engineering(maxX)} {comparison.axis_unit}
        </text>
      </svg>
      <figcaption>
        <span className="observed-key" /> Observed: {signal.mapping.data_signal}
        <span className="reference-key" /> Reference: {signal.mapping.reference_signal} · {signal.unit}
      </figcaption>
    </figure>
  );
}

export interface CsvDatasetImporterProps {
  fileInputAriaLabel: string;
  fileButtonText: string;
  fileTitle: string;
  slot: DatasetSlot & { rawText: string };
  headers: string[];
  notes: string;
  onFileSelect: (file: File) => void;
  onUpdateDraft: (change: Partial<DatasetSlot['draft']>) => void;
  onUpdateDialect: (change: Partial<CsvDialect>) => void;
  onNotesChange: (notes: string) => void;
}

export function CsvDatasetImporter({
  fileInputAriaLabel,
  fileButtonText,
  fileTitle,
  slot,
  headers,
  notes,
  onFileSelect,
  onUpdateDraft,
  onUpdateDialect,
  onNotesChange,
}: CsvDatasetImporterProps) {
  return (
    <>
      <div className="research-file-row">
        <label className="research-file-button">
          <Upload size={15} /> {fileButtonText}
          <input
            className="sr-only"
            type="file"
            accept=".csv,text/csv,text/plain"
            aria-label={fileInputAriaLabel}
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (file) void onFileSelect(file);
            }}
          />
        </label>
        <span title={fileTitle}>
          {fileTitle}
        </span>
        {slot.dataset && (
          <span className="research-ready">
            <CheckCircle2 size={14} /> Ready ({slot.dataset.spec.name})
          </span>
        )}
      </div>

      {slot.rawText && (
        <>
          {slot.preview && (
            <div className="research-preview">
              <div>
                <strong>Preview</strong> · {slot.preview.data_records.toLocaleString()} data records ·{' '}
                {headers.length} columns · {slot.preview.blank_records} blank
              </div>
              <div className="research-preview-table">
                <table>
                  <thead>
                    <tr>
                      <th>Record</th>
                      {headers.map((header, index) => (
                        <th key={index}>{header || `Column ${index + 1}`}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {slot.preview.sample.map((row) => (
                      <tr key={`${row.record}-${row.line}`}>
                        <td>{row.record}</td>
                        {row.fields.map((field, index) => (
                          <td key={index} title={field}>
                            {field}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {slot.preview && headers.length > 0 && (
            <div className="research-import-form">
              <div className="research-primary-meta">
                <label>
                  Dataset name
                  <input
                    value={slot.draft.name}
                    onChange={(event) => onUpdateDraft({ name: event.target.value })}
                  />
                </label>
                <label>
                  Origin
                  <select
                    value={slot.draft.origin}
                    onChange={(event) =>
                      onUpdateDraft({
                        origin: event.target.value as DatasetSlot['draft']['origin'],
                      })
                    }
                  >
                    <option value="measured">Measured</option>
                    <option value="published_simulation">Published simulation</option>
                    <option value="simulation">Simulation export</option>
                    <option value="synthetic">Synthetic</option>
                    <option value="unspecified">Unspecified</option>
                  </select>
                </label>
              </div>

              <h3>Comparison axis</h3>
              <PrimaryMappingRow
                mapping={slot.draft.axis}
                headers={headers}
                onChange={(axis) => onUpdateDraft({ axis })}
              />

              <div className="research-section-heading">
                <h3>Signals</h3>
                <button
                  type="button"
                  disabled={slot.draft.signals.length >= 32}
                  onClick={() => {
                    const next = slot.draft.signals.length + 1;
                    const used = new Set([
                      slot.draft.axis.column,
                      ...slot.draft.signals.map((s) => s.column),
                    ]);
                    const available = headers.findIndex((_, index) => !used.has(index));
                    onUpdateDraft({
                      signals: [
                        ...slot.draft.signals,
                        column(available >= 0 ? available : 0, `signal_${next}`, 'Volt'),
                      ],
                    });
                  }}
                >
                  <Plus size={14} /> Add signal
                </button>
              </div>
              {slot.draft.signals.map((signal, index) => (
                <PrimaryMappingRow
                  key={index}
                  mapping={signal}
                  headers={headers}
                  onChange={(next) =>
                    onUpdateDraft({
                      signals: slot.draft.signals.map((item, i) => (i === index ? next : item)),
                    })
                  }
                  onRemove={
                    slot.draft.signals.length > 1
                      ? () =>
                          onUpdateDraft({
                            signals: slot.draft.signals.filter((_, i) => i !== index),
                          })
                      : undefined
                  }
                />
              ))}

              <details>
                <summary>Advanced import settings</summary>
                <div className="research-dialect">
                  <label>
                    Delimiter
                    <select
                      value={slot.dialect.delimiter}
                      onChange={(event) =>
                        onUpdateDialect({
                          delimiter: event.target.value as CsvDialect['delimiter'],
                        })
                      }
                    >
                      <option value="comma">Comma</option>
                      <option value="semicolon">Semicolon</option>
                      <option value="tab">Tab</option>
                    </select>
                  </label>
                  <label>
                    Decimal
                    <select
                      value={slot.dialect.decimal}
                      onChange={(event) =>
                        onUpdateDialect({
                          decimal: event.target.value as CsvDialect['decimal'],
                        })
                      }
                    >
                      <option value="dot">Dot</option>
                      <option value="comma">Comma</option>
                    </select>
                  </label>
                  <label>
                    Header row
                    <select
                      value={slot.dialect.header ? 'yes' : 'no'}
                      onChange={(event) =>
                        onUpdateDialect({ header: event.target.value === 'yes' })
                      }
                    >
                      <option value="yes">First record</option>
                      <option value="no">No header</option>
                    </select>
                  </label>
                  <label>
                    Skip records
                    <input
                      type="number"
                      min="0"
                      max="128"
                      value={slot.dialect.preamble_records}
                      onChange={(event) =>
                        onUpdateDialect({ preamble_records: Number(event.target.value) })
                      }
                    />
                  </label>
                  <label>
                    Axis order
                    <select
                      value={slot.draft.axisOrder}
                      onChange={(event) =>
                        onUpdateDraft({
                          axisOrder: event.target.value as DatasetSlot['draft']['axisOrder'],
                        })
                      }
                    >
                      <option value="increasing">Increasing</option>
                      <option value="decreasing">Decreasing</option>
                    </select>
                  </label>
                  <label>
                    Missing values
                    <select
                      value={slot.draft.missing}
                      onChange={(event) =>
                        onUpdateDraft({
                          missing: event.target.value as DatasetSlot['draft']['missing'],
                        })
                      }
                    >
                      <option value="error">Stop with error</option>
                      <option value="skip_row">Skip whole row and record it</option>
                    </select>
                  </label>
                  <label className="research-wide">
                    Missing tokens
                    <input
                      value={slot.draft.missingTokens}
                      onChange={(event) => onUpdateDraft({ missingTokens: event.target.value })}
                    />
                  </label>
                </div>

                <h4>Unit scaling and offset</h4>
                <CalibrationRow
                  label="Axis"
                  mapping={slot.draft.axis}
                  axis
                  onChange={(axis) => onUpdateDraft({ axis })}
                />
                {slot.draft.signals.map((signal, index) => (
                  <CalibrationRow
                    key={index}
                    label={`Signal ${index + 1}`}
                    mapping={signal}
                    onChange={(next) =>
                      onUpdateDraft({
                        signals: slot.draft.signals.map((item, i) => (i === index ? next : item)),
                      })
                    }
                  />
                ))}

                <h4>Provenance & citation</h4>
                <div className="research-form-grid">
                  <label>
                    Device / bench setup
                    <input
                      value={slot.draft.device}
                      onChange={(event) => onUpdateDraft({ device: event.target.value })}
                    />
                  </label>
                  <label>
                    Sample / run ID
                    <input
                      value={slot.draft.sample}
                      onChange={(event) => onUpdateDraft({ sample: event.target.value })}
                    />
                  </label>
                  <label className="research-wide">
                    Citation / source URL
                    <textarea
                      rows={2}
                      value={slot.draft.citation}
                      onChange={(event) => onUpdateDraft({ citation: event.target.value })}
                    />
                  </label>
                  <label className="research-wide">
                    Import notes
                    <textarea
                      rows={2}
                      value={notes}
                      onChange={(event) => onNotesChange(event.target.value)}
                    />
                  </label>
                </div>
              </details>
            </div>
          )}
        </>
      )}

      {slot.error && (
        <p className="research-error" role="alert">
          {slot.error}
        </p>
      )}

      {slot.dataset && (
        <div className="research-import-summary">
          <div>
            <strong>{slot.dataset.axis.values.length.toLocaleString()}</strong> retained rows ·{' '}
            {slot.dataset.signals.length} signals · {slot.dataset.skipped.length} skipped
          </div>
          <code>
            {slot.dataset.raw_sha256
              ? `sha256:${slot.dataset.raw_sha256.slice(0, 16)}…`
              : slot.dataset.identity}
          </code>
          <button
            type="button"
            onClick={() =>
              downloadTextFile(
                JSON.stringify(slot.dataset, null, 2),
                `${sanitizeFileStem(slot.dataset!.spec.name)}.kessdata.json`
              )
            }
          >
            <Download size={14} /> Download dataset JSON
          </button>
        </div>
      )}
    </>
  );
}
