import { Download, Plus, RotateCcw, X } from 'lucide-react';
import type { ReactNode } from 'react';
import {
  parameterAxis,
  suffix,
  type AxisValues,
  type StudyParameter,
  type StudyResults,
  type StudySpec,
  type Unit,
} from '../experiment';
import type { BrowserEvaluation } from '../simulation/types';

export type StudyStep = 'method' | 'variables' | 'checks' | 'review' | 'results';
type Tolerance = NonNullable<StudySpec['tolerances']>;

export interface StudyMethodStepProps {
  spec: StudySpec;
  mode: 'sweep' | 'corners' | 'monte_carlo';
  parameters: StudyParameter[];
  parameterError: string;
  onUpdate: (change: Partial<StudySpec>) => void;
  onChangeMode: (mode: string) => void;
}

export function StudyMethodStep({
  spec,
  mode,
  parameters,
  parameterError,
  onUpdate,
  onChangeMode,
}: StudyMethodStepProps) {
  return (
    <div className="workflow-step-pane">
      <div className="workflow-step-intro">
        <div>
          <h3>Study method</h3>
          <p>
            Choose whether to step through explicit parameter values, compute min/max tolerance corners, or sample random
            Monte Carlo variations.
          </p>
        </div>
      </div>

      <div className="study-row">
        <label>
          Study name
          <input
            aria-label="Study name"
            value={spec.name}
            onChange={(event) => onUpdate({ name: event.target.value })}
          />
        </label>
        <label>
          Mode
          <select
            aria-label="Mode"
            value={mode}
            onChange={(event) => onChangeMode(event.target.value)}
          >
            <option value="sweep">Parameter sweep</option>
            <option value="corners">Nominal + tolerance corners</option>
            <option value="monte_carlo">Nominal + Monte Carlo samples</option>
          </select>
        </label>
      </div>

      <div className="workflow-card-grid">
        <button
          type="button"
          className={`workflow-choice-card ${mode === 'sweep' ? 'is-selected' : ''}`}
          aria-pressed={mode === 'sweep'}
          onClick={() => onChangeMode('sweep')}
        >
          <strong>Parameter sweep</strong>
          <span>Step through explicit parameter values or multi-parameter combinations.</span>
        </button>
        <button
          type="button"
          className={`workflow-choice-card ${mode === 'corners' ? 'is-selected' : ''}`}
          aria-pressed={mode === 'corners'}
          onClick={() => onChangeMode('corners')}
        >
          <strong>Nominal + tolerance corners</strong>
          <span>Run the nominal case plus every min/max tolerance corner.</span>
        </button>
        <button
          type="button"
          className={`workflow-choice-card ${mode === 'monte_carlo' ? 'is-selected' : ''}`}
          aria-pressed={mode === 'monte_carlo'}
          onClick={() => onChangeMode('monte_carlo')}
        >
          <strong>Nominal + Monte Carlo samples</strong>
          <span>Sample random parameter variations around nominal with a deterministic seed.</span>
        </button>
      </div>

      {parameters.length === 0 && !parameterError && spec.revisions.length === 0 && (
        <p className="study-note">
          No sweepable param declarations in this circuit. Proceed to <strong>Variables</strong> to inspect details or open
          a preconfigured example.
        </p>
      )}
    </div>
  );
}

export interface StudyVariablesStepProps {
  spec: StudySpec;
  mode: 'sweep' | 'corners' | 'monte_carlo';
  parameters: StudyParameter[];
  parameterError: string;
  onUpdate: (change: Partial<StudySpec>) => void;
  onAxisUpdate: (index: number, change: Partial<StudySpec['axes'][number]>) => void;
  onToleranceUpdate: (index: number, change: Partial<Tolerance['parameters'][number]>) => void;
  onTryExample: () => void;
}

export function StudyVariablesStep({
  spec,
  mode,
  parameters,
  parameterError,
  onUpdate,
  onAxisUpdate,
  onToleranceUpdate,
  onTryExample,
}: StudyVariablesStepProps) {
  const parameterOptions: ReactNode[] = parameters.map((p) => (
    <option key={p.name} value={p.name}>
      {p.name} ({suffix[p.declared_unit] || 'ratio'})
    </option>
  ));

  return (
    <div className="workflow-step-pane">
      <div className="workflow-step-intro">
        <div>
          <h3>Variables</h3>
          <p>Configure which parameters vary across cases and how values are stepped.</p>
        </div>
      </div>

      {parameterError && (
        <p role="alert">
          {parameterError} Open the study source in the editor and choose its matching model files.
        </p>
      )}

      {parameters.length === 0 && !parameterError && spec.revisions.length === 0 ? (
        <div className="study-empty-params">
          <strong>
            No sweepable <code>param</code> declarations in this circuit
          </strong>
          <p className="study-note">
            Parameter studies vary top-level <code>param</code> values across sweeps, tolerance corners, or Monte Carlo runs.
            Declare a parameter in the editor and reference it on a component:
          </p>
          <pre className="study-param-example">{`param R_load = 1kOhm\nresistor R1 R_load`}</pre>
          <p className="study-note">
            Or choose <strong>Open example</strong> to inspect a 6-case filter study, or <strong>Open study / results…</strong> to load a saved <code>.kessstudy.json</code> file.
          </p>
          <button
            type="button"
            className="study-example-button"
            onClick={onTryExample}
          >
            Open example
          </button>
        </div>
      ) : (
        <>
          {mode === 'sweep' && (
            <section>
              <h3>Sweep parameters</h3>
              {spec.axes.map((axis, index) => (
                <div className="study-axis" key={index}>
                  <label>
                    Parameter
                    <select
                      aria-label={`Sweep parameter ${index + 1}`}
                      value={axis.parameter}
                      onChange={(event) => {
                        const parameter = parameters.find((p) => p.name === event.target.value);
                        if (parameter) onAxisUpdate(index, parameterAxis(parameter));
                      }}
                    >
                      {parameterOptions}
                    </select>
                  </label>
                  <label>
                    Values
                    <select
                      aria-label={`Sweep method ${index + 1}`}
                      value={axis.values.kind}
                      onChange={(event) => {
                        const p = parameters.find((param) => param.name === axis.parameter);
                        const value = p?.resolved.value ?? 1;
                        const u = p ? suffix[p.declared_unit] : '';
                        const values: AxisValues =
                          event.target.value === 'list'
                            ? { kind: 'list', values: [`${value}${u}`] }
                            : {
                                kind: event.target.value as 'linear' | 'log',
                                start: `${value * 0.5}${u}`,
                                stop: `${value * 2}${u}`,
                                points: 5,
                              };
                        onAxisUpdate(index, { values });
                      }}
                    >
                      <option value="list">Explicit list</option>
                      <option value="linear">Linear range</option>
                      <option value="log">Log range</option>
                    </select>
                  </label>
                  {axis.values.kind === 'list' ? (
                    <label className="study-values">
                      Comma-separated values
                      <input
                        aria-label={`Sweep values ${index + 1}`}
                        value={axis.values.values.join(', ')}
                        onChange={(event) =>
                          onAxisUpdate(index, {
                            values: {
                              kind: 'list',
                              values: event.target.value.split(',').map((v) => v.trim()),
                            },
                          })
                        }
                      />
                    </label>
                  ) : (
                    <>
                      <label>
                        Start
                        <input
                          aria-label={`Sweep start ${index + 1}`}
                          value={axis.values.start}
                          onChange={(event) => {
                            if (axis.values.kind !== 'list')
                              onAxisUpdate(index, { values: { ...axis.values, start: event.target.value } });
                          }}
                        />
                      </label>
                      <label>
                        Stop
                        <input
                          aria-label={`Sweep stop ${index + 1}`}
                          value={axis.values.stop}
                          onChange={(event) => {
                            if (axis.values.kind !== 'list')
                              onAxisUpdate(index, { values: { ...axis.values, stop: event.target.value } });
                          }}
                        />
                      </label>
                      <label>
                        Points
                        <input
                          aria-label={`Sweep points ${index + 1}`}
                          type="number"
                          min="2"
                          max="256"
                          value={axis.values.points}
                          onChange={(event) => {
                            if (axis.values.kind !== 'list')
                              onAxisUpdate(index, {
                                values: { ...axis.values, points: Number(event.target.value) },
                              });
                          }}
                        />
                      </label>
                    </>
                  )}
                  <button
                    type="button"
                    aria-label={`Remove sweep axis ${index + 1}`}
                    onClick={() => onUpdate({ axes: spec.axes.filter((_, i) => i !== index) })}
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              <button
                type="button"
                disabled={spec.axes.length >= 8 || parameters.length === 0}
                onClick={() => {
                  const p = parameters.find((param) => !spec.axes.some((a) => a.parameter === param.name));
                  if (p) onUpdate({ axes: [...spec.axes, parameterAxis(p)] });
                }}
              >
                <Plus size={14} /> Add parameter
              </button>
            </section>
          )}

          {mode === 'monte_carlo' && spec.tolerances && (
            <section>
              <h3>Monte Carlo sampling</h3>
              <div className="study-row">
                <label>
                  Sample count
                  <input
                    aria-label="Sample count"
                    type="number"
                    min="1"
                    max="255"
                    value={spec.tolerances.samples}
                    onChange={(event) =>
                      onUpdate({
                        tolerances: { ...spec.tolerances!, samples: Number(event.target.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Random seed
                  <input
                    aria-label="Random seed"
                    type="number"
                    min="0"
                    max="4294967295"
                    value={spec.tolerances.seed}
                    onChange={(event) =>
                      onUpdate({
                        tolerances: { ...spec.tolerances!, seed: Number(event.target.value) },
                      })
                    }
                  />
                </label>
              </div>
              <p className="study-note">
                Same-group parameters share a signed deviation. Gaussian samples are unbounded; invalid values stay failed cases.
              </p>
            </section>
          )}

          {(mode === 'corners' || mode === 'monte_carlo') && spec.tolerances && (
            <section>
              <h3>Tolerance corners</h3>
              {spec.tolerances.parameters.map((p, index) => (
                <div className="study-axis" key={index}>
                  <label>
                    Parameter
                    <select
                      aria-label={`Corner parameter ${index + 1}`}
                      value={p.parameter}
                      onChange={(event) => onToleranceUpdate(index, { parameter: event.target.value })}
                    >
                      {parameterOptions}
                    </select>
                  </label>
                  <label>
                    {p.distribution === 'normal' ? 'Standard deviation (%)' : 'Half-width (±%)'}
                    <input
                      aria-label={`Corner tolerance ${index + 1}`}
                      type="number"
                      min="0.001"
                      max="99"
                      value={p.relative * 100}
                      onChange={(event) =>
                        onToleranceUpdate(index, { relative: Number(event.target.value) / 100 })
                      }
                    />
                  </label>
                  {mode === 'monte_carlo' && (
                    <>
                      <label>
                        Distribution
                        <select
                          aria-label={`Corner mode ${index + 1}`}
                          value={p.distribution}
                          onChange={(event) =>
                            onToleranceUpdate(index, {
                              distribution: event.target.value as 'uniform' | 'normal',
                            })
                          }
                        >
                          <option value="uniform">Uniform</option>
                          <option value="normal">Gaussian</option>
                        </select>
                      </label>
                      <label>
                        Correlation group (optional)
                        <input
                          aria-label={`Corner group ${index + 1}`}
                          value={p.group ?? ''}
                          onChange={(event) =>
                            onToleranceUpdate(index, { group: event.target.value || null })
                          }
                        />
                      </label>
                    </>
                  )}
                  <button
                    type="button"
                    aria-label={`Remove tolerance ${index + 1}`}
                    onClick={() =>
                      onUpdate({
                        tolerances: {
                          ...spec.tolerances!,
                          parameters: spec.tolerances!.parameters.filter((_, i) => i !== index),
                        },
                      })
                    }
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              <button
                type="button"
                disabled={spec.tolerances.parameters.length >= 8 || parameters.length === 0}
                onClick={() => {
                  const p = parameters.find(
                    (param) => !spec.tolerances!.parameters.some((t) => t.parameter === param.name),
                  );
                  if (p)
                    onUpdate({
                      tolerances: {
                        ...spec.tolerances!,
                        parameters: [
                          ...spec.tolerances!.parameters,
                          { parameter: p.name, relative: 0.05, distribution: 'uniform', group: null },
                        ],
                      },
                    });
                }}
              >
                <Plus size={14} /> Add tolerance
              </button>
            </section>
          )}
        </>
      )}
    </div>
  );
}

export interface StudyChecksStepProps {
  spec: StudySpec;
  reqFileName: string;
  setReqFileName: (name: string) => void;
  temperatureText: string;
  setTemperatureText: (text: string) => void;
  useAdvancedJson: boolean;
  setUseAdvancedJson: (val: boolean) => void;
  advancedJson: string;
  setAdvancedJson: (val: string) => void;
  onUpdate: (change: Partial<StudySpec>) => void;
  onMeasureUpdate: (index: number, change: Partial<StudySpec['measurements'][number]>) => void;
  onDownloadSpec: () => void;
  onApplyJson: () => void;
}

export function StudyChecksStep({
  spec,
  reqFileName,
  setReqFileName,
  temperatureText,
  setTemperatureText,
  useAdvancedJson,
  setUseAdvancedJson,
  advancedJson,
  setAdvancedJson,
  onUpdate,
  onMeasureUpdate,
  onDownloadSpec,
  onApplyJson,
}: StudyChecksStepProps) {
  return (
    <div className="workflow-step-pane">
      <div className="workflow-step-intro">
        <div>
          <h3>Checks and conditions</h3>
          <p>
            Encoded <code>assert</code> statements run automatically on every case. Define optional metric measurements,
            objectives, or requirement files below.
          </p>
        </div>
      </div>

      <section>
        <h3>Measurements (optional)</h3>
        <p className="study-note">
          Use metric expressions without comparisons to record numerical outputs across all evaluated cases.
        </p>
        {spec.measurements.map((m, index) => (
          <div className="study-axis" key={index}>
            <label>
              Name
              <input
                aria-label={`Measurement id ${index + 1}`}
                value={m.name}
                onChange={(event) => onMeasureUpdate(index, { name: event.target.value })}
              />
            </label>
            <label className="study-values">
              Metric expression
              <input
                aria-label={`Target ${index + 1}`}
                placeholder="cutoff(V(OUT),V(IN))"
                value={m.expression}
                onChange={(event) => onMeasureUpdate(index, { expression: event.target.value })}
              />
            </label>
            <label>
              Unit
              <select
                aria-label={`Unit ${index + 1}`}
                value={m.unit}
                onChange={(event) => onMeasureUpdate(index, { unit: event.target.value as Unit })}
              >
                {Object.keys(suffix).map((unit) => (
                  <option key={unit} value={unit}>
                    {unit}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              aria-label={`Remove measurement ${index + 1}`}
              onClick={() =>
                onUpdate({
                  measurements: spec.measurements.filter((_, i) => i !== index),
                  objective: spec.objective?.measurement === m.name ? null : spec.objective,
                })
              }
            >
              <X size={14} />
            </button>
          </div>
        ))}
        <button
          type="button"
          onClick={() =>
            onUpdate({
              measurements: [
                ...spec.measurements,
                { name: `metric_${spec.measurements.length + 1}`, expression: 'cutoff(V(OUT),V(IN))', unit: 'Hertz' },
              ],
            })
          }
        >
          <Plus size={14} /> Add measurement
        </button>
      </section>

      <section>
        <h3>Search objective (optional)</h3>
        <div className="study-row">
          <label>
            Objective measurement
            <select
              aria-label="Objective measurement"
              value={spec.objective?.measurement ?? ''}
              onChange={(event) =>
                onUpdate({
                  objective: event.target.value
                    ? { measurement: event.target.value, direction: 'minimize' }
                    : null,
                })
              }
            >
              <option value="">None — compare all cases</option>
              {spec.measurements.map((m) => (
                <option key={m.name} value={m.name}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
          {spec.objective && (
            <label>
              Goal
              <select
                aria-label="Goal"
                value={spec.objective.direction}
                onChange={(event) =>
                  onUpdate({
                    objective: {
                      ...spec.objective!,
                      direction: event.target.value as 'minimize' | 'maximize',
                    },
                  })
                }
              >
                <option value="minimize">Minimize</option>
                <option value="maximize">Maximize</option>
              </select>
            </label>
          )}
        </div>
        <p className="study-note">
          Search selects only the best feasible candidate. Only cases where all assertions and requirements pass are considered.
        </p>
      </section>

      <details>
        <summary>Independent requirement constraints (.kessreq)</summary>
        <div className="study-row">
          <label>
            Requirement file name
            <input
              aria-label="Requirement file name"
              value={reqFileName}
              onChange={(event) => setReqFileName(event.target.value)}
              placeholder="study.kessreq"
            />
          </label>
        </div>
        <label>
          Requirement statements (.kessreq)
          <textarea
            aria-label="Requirement statements (.kessreq)"
            rows={3}
            value={spec.requirements ?? ''}
            placeholder="assert gain_at(V(OUT),V(IN),1kHz) > 0.6"
            onChange={(event) => onUpdate({ requirements: event.target.value || null })}
          />
        </label>
        <p className="study-note">
          Independent requirements cannot mix with inline assertions or reference design parameters.
        </p>
      </details>

      <details>
        <summary>Execution conditions &amp; advanced specification</summary>
        <div className="study-row">
          <label>
            Temperatures (°C)
            <input
              aria-label="Temperatures (°C)"
              value={temperatureText}
              onChange={(event) => {
                setTemperatureText(event.target.value);
                onUpdate({
                  temperatures_c: event.target.value.split(',').map((v) => (v.trim() ? Number(v.trim()) : NaN)),
                });
              }}
            />
          </label>
          <label>
            Per-case timeout (ms)
            <input
              aria-label="Per-case timeout (ms)"
              type="number"
              min="1"
              max="120000"
              value={spec.timeout_ms}
              onChange={(event) => onUpdate({ timeout_ms: Math.round(Number(event.target.value)) })}
            />
          </label>
        </div>
        <p className="study-note">
          Temperature affects only models with temperature-dependent equations. Ideal passives and generic op-amps do not acquire new temperature coefficients.
        </p>
        <button
          type="button"
          onClick={onDownloadSpec}
        >
          <Download size={14} /> Download specification
        </button>

        <details>
          <summary>Advanced JSON: revisions and combined studies</summary>
          <label className="study-checkbox-label">
            <input
              type="checkbox"
              checked={useAdvancedJson}
              onChange={(e) => setUseAdvancedJson(e.target.checked)}
            />
            Use advanced JSON specification
          </label>
          <textarea
            rows={10}
            aria-label="Advanced study specification"
            value={advancedJson || JSON.stringify(spec, null, 2)}
            onChange={(event) => setAdvancedJson(event.target.value)}
          />
          <button
            type="button"
            aria-label="Apply JSON"
            onClick={onApplyJson}
          >
            Apply JSON
          </button>
        </details>
      </details>
    </div>
  );
}

export interface StudyReviewStepProps {
  spec: StudySpec;
  source: string;
  name: string;
  running: boolean;
  mode: 'sweep' | 'corners' | 'monte_carlo';
  plannedCasesPreview: { count: number; error: string | null };
  onUseEditorCircuit: () => void;
  onSetStep: (step: StudyStep) => void;
}

export function StudyReviewStep({
  spec,
  source,
  name,
  running,
  mode,
  plannedCasesPreview,
  onUseEditorCircuit,
  onSetStep,
}: StudyReviewStepProps) {
  return (
    <div className="workflow-step-pane">
      <div className="workflow-step-intro">
        <div>
          <h3>Review and run study</h3>
          <p>Verify planned cases, variable axes, and checks before running simulations.</p>
        </div>
      </div>

      <div className="study-review-grid">
        <div className="study-review-card">
          <div className="study-review-card-header">
            <strong>Circuit snapshot</strong>
          </div>
          <div className="study-review-card-body">
            {spec.source === source ? (
              <p>
                Current editor circuit: <strong>{name}</strong>
              </p>
            ) : (
              <div>
                <p>
                  Studying snapshot: <strong>{spec.name}</strong>{' '}
                  <small>(differs from editor circuit “{name}”)</small>
                </p>
                <button type="button" onClick={onUseEditorCircuit} disabled={running}>
                  <RotateCcw size={13} /> Use editor circuit
                </button>
              </div>
            )}
          </div>
        </div>

        <div className="study-review-card">
          <div className="study-review-card-header">
            <strong>Method</strong>
          </div>
          <div className="study-review-card-body">
            <p>
              <strong>
                {mode === 'sweep'
                  ? 'Parameter sweep'
                  : mode === 'corners'
                  ? 'Nominal + tolerance corners'
                  : 'Nominal + Monte Carlo samples'}
              </strong>
            </p>
            <p className="study-review-highlight">
              {plannedCasesPreview.error ? (
                <span className="study-error">{plannedCasesPreview.error}</span>
              ) : (
                <span>{plannedCasesPreview.count} planned cases</span>
              )}
            </p>
            {plannedCasesPreview.count > 32 && (
              <p className="study-note study-error">
                Browser limit is 32 cases. Download the specification and run with the CLI for larger studies.
              </p>
            )}
          </div>
        </div>

        <div className="study-review-card">
          <div className="study-review-card-header">
            <strong>Variables</strong>
            <button type="button" onClick={() => onSetStep('variables')} disabled={running}>
              Edit variables
            </button>
          </div>
          <div className="study-review-card-body">
            {mode === 'sweep' ? (
              spec.axes.length > 0 ? (
                <ul className="study-review-list">
                  {spec.axes.map((axis, i) => (
                    <li key={i}>
                      <code>{axis.parameter}</code>:{' '}
                      {axis.values.kind === 'list'
                        ? `${axis.values.values.length} explicit values (${axis.values.values.join(', ')})`
                        : `${axis.values.points} points (${axis.values.start} to ${axis.values.stop})`}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="study-note">No sweep axes configured.</p>
              )
            ) : spec.tolerances && spec.tolerances.parameters.length > 0 ? (
              <ul className="study-review-list">
                {spec.tolerances.parameters.map((p, i) => (
                  <li key={i}>
                    <code>{p.parameter}</code>: ±{(p.relative * 100).toFixed(1)}%{' '}
                    {mode === 'monte_carlo' ? `(${p.distribution})` : ''}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="study-note">No tolerances configured.</p>
            )}
          </div>
        </div>

        <div className="study-review-card">
          <div className="study-review-card-header">
            <strong>Checks &amp; conditions</strong>
            <button type="button" onClick={() => onSetStep('checks')} disabled={running}>
              Edit checks
            </button>
          </div>
          <div className="study-review-card-body">
            <ul className="study-review-list">
              <li>
                <strong>Measurements:</strong>{' '}
                {spec.measurements.length > 0
                  ? spec.measurements.map((m) => m.name).join(', ')
                  : 'None (circuit assertions only)'}
              </li>
              <li>
                <strong>Objective:</strong>{' '}
                {spec.objective
                  ? `${spec.objective.direction} ${spec.objective.measurement}`
                  : 'None — compare all cases'}
              </li>
              <li>
                <strong>Constraints:</strong>{' '}
                {spec.requirements?.trim() ? 'Literal constraints defined' : 'Circuit assertions only'}
              </li>
              <li>
                <strong>Temperatures:</strong> {spec.temperatures_c.join(', ')} °C
              </li>
              <li>
                <strong>Per-case timeout:</strong> {spec.timeout_ms} ms
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}

export interface StudyResultsStepProps {
  results: StudyResults;
  running: boolean;
  resultsModeLabel: string;
  previousResults: StudyResults | null;
  comparison: StudyResults | null;
  counts: Record<string, number> | undefined;
  firstSimulation: BrowserEvaluation['simulation'] | null | undefined;
  analysis: number;
  setAnalysis: (val: number) => void;
  signal: string;
  setSignal: (val: string) => void;
  signals: string[];
  setComparison: (val: StudyResults | null) => void;
  plot: string;
  plotError: string;
  selected: string[];
  setSelected: React.Dispatch<React.SetStateAction<string[]>>;
  exportTarget: string;
  setExportTarget: (val: string) => void;
  onSetStep: (step: StudyStep) => void;
  onCompareInputClick: () => void;
  onDownloadResultsJson: () => void;
  onDownloadSummaryCsv: () => void;
  onExportFile: () => void;
  onApplyCase: (index: number) => void;
}

export function StudyResultsStep({
  results,
  running,
  resultsModeLabel,
  previousResults,
  comparison,
  counts,
  firstSimulation,
  analysis,
  setAnalysis,
  signal,
  setSignal,
  signals,
  setComparison,
  plot,
  plotError,
  selected,
  setSelected,
  exportTarget,
  setExportTarget,
  onSetStep,
  onCompareInputClick,
  onDownloadResultsJson,
  onDownloadSummaryCsv,
  onExportFile,
  onApplyCase,
}: StudyResultsStepProps) {
  return (
    <section className="study-results" aria-label="Study results">
      <div className="study-results-header">
        <div className="study-results-provenance">
          <strong>{results.plan.spec.name}</strong>
          <span className="study-badge">{resultsModeLabel}</span>
          <span className="study-badge">
            {results.cases.filter((c) => c.status !== 'pending').length}/{results.plan.cases.length} cases
          </span>
          <small>
            {results.simulator.executable} {results.simulator.version}
          </small>
        </div>
        <div className="study-results-actions">
          <button type="button" disabled={running} onClick={() => onSetStep('variables')}>
            Edit setup
          </button>
          {previousResults && !comparison && (
            <button type="button" disabled={running} onClick={() => setComparison(previousResults)}>
              Compare with previous run
            </button>
          )}
          <button type="button" disabled={running} onClick={onCompareInputClick}>
            Compare another Results JSON…
          </button>
          <button type="button" disabled={running} onClick={onDownloadResultsJson}>
            Download Results JSON
          </button>
          <button type="button" disabled={running} onClick={onDownloadSummaryCsv}>
            Download CSV
          </button>
        </div>
      </div>

      <p className="study-summary" data-testid="study-summary">
        {results.plan.cases.length} total · {counts?.passed ?? 0} passed ·{' '}
        {counts?.completed ?? 0} completed without constraints · {counts?.failed ?? 0} failed ·{' '}
        {counts?.error ?? 0} errors · {counts?.cancelled ?? 0} cancelled · {counts?.pending ?? 0} pending
      </p>

      {!running && results.summary.best_case && (
        <p>
          Best feasible evaluated case: <strong>{results.summary.best_case}</strong>. Search budget:{' '}
          {results.summary.total} cases.
        </p>
      )}

      {!running && results.plan.spec.objective && !results.summary.best_case && (
        <p>No feasible evaluated candidate met the study constraints.</p>
      )}

      <div className="study-row">
        <label>
          Analysis
          <select
            aria-label="Analysis"
            value={analysis}
            onChange={(event) => setAnalysis(Number(event.target.value))}
          >
            {firstSimulation?.datasets.map((d) => (
              <option key={d.index} value={d.index}>
                {d.index + 1}: {d.analysis.kind}
              </option>
            ))}
          </select>
        </label>
        <label>
          Overlay signal
          <select
            aria-label="Overlay signal"
            value={signal}
            onChange={(event) => setSignal(event.target.value)}
          >
            <option value={signal}>{signal}</option>
            {signals
              .filter((s) => s.toLowerCase() !== signal.toLowerCase())
              .map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
          </select>
        </label>
        {comparison && (
          <button type="button" onClick={() => setComparison(null)}>
            Clear comparison
          </button>
        )}
      </div>

      {comparison && (
        <p>
          Comparing with {comparison.plan.spec.name}. Up to six cases per report are plotted; original files retain every case.
        </p>
      )}

      {plot && <div className="study-plot" data-testid="study-plot" dangerouslySetInnerHTML={{ __html: plot }} />}
      {!running && plotError && <p className="study-note">{plotError}</p>}

      <div className="study-table">
        <table>
          <thead>
            <tr>
              <th>Overlay</th>
              <th>Case / conditions</th>
              <th>Status</th>
              <th>Measured outcomes</th>
              <th>Continue</th>
            </tr>
          </thead>
          <tbody>
            {results.plan.cases.map((point, index) => {
              const row = results.cases[index];
              return (
                <tr key={point.id} className={`study-case-${row.status}`}>
                  <td>
                    <input
                      type="checkbox"
                      aria-label={`Overlay ${point.id}`}
                      title={`Plot ${point.id}`}
                      checked={selected.includes(point.id)}
                      disabled={running || comparison !== null}
                      onChange={(event) =>
                        setSelected((current) =>
                          event.target.checked
                            ? [...current, point.id].slice(0, 12)
                            : current.filter((id) => id !== point.id),
                        )
                      }
                    />
                  </td>
                  <td>
                    <strong>{point.id}</strong>
                    <br />
                    {point.name} · {point.temperature_c} °C
                    <br />
                    <code>
                      {Object.entries(point.parameters)
                        .map(([k, v]) => `${k}=${v}`)
                        .join(', ') || 'Nominal parameters'}
                    </code>
                  </td>
                  <td>{row.status}</td>
                  <td>
                    {Object.entries(row.measurements).map(([mName, m]) => (
                      <div key={mName}>
                        {mName}:{' '}
                        {m.value === null ? m.error : `${m.value.toPrecision(6)} ${suffix[m.unit]}`}
                      </div>
                    ))}
                    {row.assertions?.assertions.map((a) => (
                      <div key={a.code}>
                        {a.metric}({a.signal}): {a.actual?.toPrecision(6) ?? 'unavailable'} · {a.status}
                        {a.message && ` — ${a.message}`}
                      </div>
                    ))}
                    {row.errors.map((e, i) => (
                      <div key={i}>{e}</div>
                    ))}
                  </td>
                  <td>
                    <button
                      type="button"
                      title={`Open case ${point.id} in editor`}
                      disabled={running || row.status === 'pending' || row.status === 'error'}
                      onClick={() => onApplyCase(index)}
                    >
                      Apply parameters
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="study-row">
        <label>
          Export current study
          <select
            aria-label="Export current study"
            value={exportTarget}
            onChange={(event) => setExportTarget(event.target.value)}
          >
            <option value="json">Results JSON — full data + resume</option>
            <option value="csv">Measurement summary CSV</option>
            <option value="data_csv">Full numeric samples CSV</option>
            <option value="svg">Visible overlay SVG</option>
            <option value="html">Printable HTML report</option>
          </select>
        </label>
        <button type="button" disabled={running} onClick={onExportFile}>
          <Download size={14} /> Download
        </button>
      </div>
    </section>
  );
}
