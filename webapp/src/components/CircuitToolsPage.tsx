import { ArrowRight, Download, Terminal } from 'lucide-react';
import { useEffect, useState, type FormEvent, type ReactNode } from 'react';
import { downloadTextFile, stageToolCircuit } from '../document';
import { BrandWordmark } from './BrandWordmark';
import '../landing.css';
import '../tools.css';

const circuitTools = {
  divider: {
    slug: 'voltage-divider', name: 'Loaded voltage divider',
    description: 'Size a divider for the voltage you need with the actual load attached. See loading error, current and resistor dissipation.',
    fields: [
      { key: 'input_voltage', label: 'Input voltage', value: '12V', hint: 'Ideal DC supply, e.g. 12V' },
      { key: 'target_voltage', label: 'Target output voltage', value: '3V', hint: 'Between zero and input voltage' },
      { key: 'lower_resistance', label: 'Lower resistor R2', value: '10k', hint: 'Choose the resistance scale, e.g. 10kΩ' },
      { key: 'load_resistance', label: 'Load resistance RL', value: '10k', hint: 'Leave empty for an open-circuit load', optional: true },
    ],
    formula: 'R1 = (R2 || RL) × (Vin / Vtarget − 1)',
    assumption: 'Nominal linear resistors and an ideal DC source. A resistive load connects OUT to ground. Power values describe dissipation, not the component rating you should buy.',
  },
  rc_lowpass: {
    slug: 'rc-lowpass', name: 'RC low-pass filter',
    description: 'Choose a cutoff frequency and resistor. Calculate the capacitor, select standard values and open a circuit ready for AC simulation.',
    fields: [
      { key: 'cutoff', label: 'Target cutoff frequency', value: '1kHz', hint: 'The −3.01 dB frequency, e.g. 1kHz' },
      { key: 'resistance', label: 'Resistor R1', value: '1k', hint: 'Choose the resistance scale, e.g. 1kΩ' },
    ],
    formula: 'C = 1 / (2π × R × cutoff frequency)',
    assumption: 'Nominal R/C values, ideal voltage source and high-impedance output. Extra source resistance, output loading and component tolerances are not included.',
  },
} as const;

type ToolId = keyof typeof circuitTools;
type ValueSeries = 'exact' | 'e12' | 'e24';
interface Quantity { value: number; unit: string }
interface ToolResult {
  schema_version: string;
  tool: ToolId;
  name: string;
  inputs: Record<string, Quantity>;
  components: Record<string, Quantity>;
  ideal_components: Record<string, Quantity>;
  results: Record<string, Quantity>;
  equations: string[];
  assumptions: string[];
  source: string;
}
type ToolCalculator = (request: Record<string, unknown>) => ToolResult;

const resultLabels: Record<string, string> = {
  output_voltage: 'Loaded output', unloaded_voltage: 'Unloaded output', target_error: 'Error from target',
  loading_error: 'Change due to load', input_current: 'Supply current', lower_current: 'R2 current',
  load_current: 'Load current', r1_power: 'R1 dissipation', r2_power: 'R2 dissipation',
  load_power: 'Load power', output_resistance: 'Output resistance', cutoff: 'Achieved cutoff', time_constant: 'Time constant',
};

function displayQuantity({ value, unit }: Quantity): string {
  const symbols: Record<string, string> = { Ohm: 'Ω', Farad: 'F', Hertz: 'Hz', Volt: 'V', Ampere: 'A', Watt: 'W', Second: 's', Percent: '%' };
  if (unit === 'Percent') return `${Number(value.toPrecision(5))} %`;
  const exponent = value === 0 ? 0 : Math.floor(Math.log10(Math.abs(value)) / 3) * 3;
  const prefixes: Record<number, string> = { '-12': 'p', '-9': 'n', '-6': 'µ', '-3': 'm', 0: '', 3: 'k', 6: 'M', 9: 'G' };
  return exponent in prefixes
    ? `${Number((value / 10 ** exponent).toPrecision(6))} ${prefixes[exponent]}${symbols[unit] ?? unit}`
    : `${value.toExponential(4)} ${symbols[unit] ?? unit}`;
}

function ToolsShell({ children }: { children: ReactNode }) {
  return <main className="landing-page tools-page">
    <header className="landing-header">
      <a className="landing-wordmark" href={import.meta.env.BASE_URL} aria-label="Kessetsu home"><BrandWordmark /></a>
      <nav className="landing-nav" aria-label="Circuit tools navigation">
        <a href={`${import.meta.env.BASE_URL}tools/`}>Tools</a>
        <a href={`${import.meta.env.BASE_URL}docs/`}>Docs</a>
      </nav>
      <a className="button button-compact button-secondary" href={`${import.meta.env.BASE_URL}#editor`}>Open editor <ArrowRight size={15} /></a>
    </header>
    <div className="tools-content">{children}</div>
    <footer className="landing-footer"><span>Free · No account · Calculated locally with Kessetsu Core</span>
      <a href={`${import.meta.env.BASE_URL}install/`}>Also available through the CLI</a>
    </footer>
  </main>;
}

export function CircuitToolsIndex() {
  useEffect(() => { document.title = 'Circuit tools — Kessetsu'; }, []);
  return <ToolsShell>
    <div className="tools-heading"><p className="eyebrow">Circuit tools</p>
      <h1>Calculate. Open the circuit. Keep going.</h1>
      <p>Start with a practical calculation, inspect the assumptions and continue in the editor with an editable schematic and real SPICE simulation.</p>
    </div>
    <div className="tool-cards">{Object.entries(circuitTools).map(([id, tool]) =>
      <a className="tool-card" key={id} href={`${import.meta.env.BASE_URL}tools/${tool.slug}/`}>
        <h2>{tool.name}</h2><p>{tool.description}</p><span>Open tool <ArrowRight size={16} /></span>
      </a>)}</div>
  </ToolsShell>;
}

export function CircuitToolsPage({ toolId }: { toolId: ToolId }) {
  const definition = circuitTools[toolId];
  const [inputs, setInputs] = useState<Record<string, string>>(() => Object.fromEntries(definition.fields.map((field) => [field.key, field.value])));
  const [series, setSeries] = useState<ValueSeries>('e24');
  const [calculator, setCalculator] = useState<ToolCalculator | null>(null);
  const [result, setResult] = useState<ToolResult | null>(null);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    document.title = `${definition.name} — Kessetsu`;
    let mounted = true;
    void import('kessetsu-core').then(async (core) => {
      await core.default();
      if (mounted) setCalculator(() => (request: Record<string, unknown>) => core.calculate_circuit_tool(request) as ToolResult);
    }).catch((cause: unknown) => mounted && setError(`Could not load the calculation engine: ${String(cause)}`));
    return () => { mounted = false; };
  }, [definition.name]);

  const calculate = (event: FormEvent) => {
    event.preventDefault();
    if (!calculator) return;
    try {
      const request: Record<string, unknown> = { tool: toolId, ...inputs, preferred_values: series };
      if (toolId === 'divider' && !inputs.load_resistance.trim()) delete request.load_resistance;
      const calculation = calculator(request);
      if (calculation.schema_version !== 'kessetsu.tool.v1') throw new Error('Unsupported calculation contract');
      setResult(calculation); setError(''); setNotice('');
    } catch (cause: unknown) { setResult(null); setError(String(cause)); }
  };
  const openInEditor = () => {
    if (!result) return;
    try {
      stageToolCircuit(result.name, result.source);
      globalThis.location.assign(`${import.meta.env.BASE_URL}#editor`);
    } catch {
      setError('Browser storage is unavailable. Download the .kess source and open it in the editor instead.');
    }
  };
  const cliCommand = toolId === 'divider'
    ? `kess tool divider --vin "${inputs.input_voltage}" --target "${inputs.target_voltage}" --lower "${inputs.lower_resistance}"${inputs.load_resistance.trim() ? ` --load "${inputs.load_resistance}"` : ''} --values ${series} --output divider.kess`
    : `kess tool rc-lowpass --cutoff "${inputs.cutoff}" --resistance "${inputs.resistance}" --values ${series} --output rc-lowpass.kess`;

  return <ToolsShell>
    <a className="text-link" href={`${import.meta.env.BASE_URL}tools/`}>← All circuit tools</a>
    <div className="tools-heading"><p className="eyebrow">Calculate and design</p>
      <h1>{definition.name}</h1><p>{definition.description}</p>
    </div>
    <div className="tool-layout">
      <form className="tool-inputs" onSubmit={calculate}>
        <h2>Design inputs</h2>
        {definition.fields.map((field) => <label className="tool-field" key={field.key}>
          <span>{field.label}</span>
          <input type="text" aria-label={field.label} value={inputs[field.key]} required={!('optional' in field && field.optional)}
            spellCheck={false} aria-describedby={`${field.key}-hint`} onChange={(event) => {
              setInputs((current) => ({ ...current, [field.key]: event.target.value })); setResult(null); setError(''); setNotice('');
            }} />
          <small id={`${field.key}-hint`}>{field.hint}</small>
        </label>)}
        <label className="tool-field"><span>Component values</span>
          <select aria-label="Component values" value={series} onChange={(event) => { setSeries(event.target.value as ValueSeries); setResult(null); setError(''); }}>
            <option value="e24">E24 standard values</option><option value="e12">E12 standard values</option><option value="exact">Exact calculated values</option>
          </select><small>Nearest nominal values. This does not apply component tolerances.</small>
        </label>
        <button className="button button-primary" type="submit" disabled={!calculator}>{calculator ? 'Calculate circuit' : 'Loading Core…'}</button>
        {error && <p className="tool-error" role="alert">{error}</p>}
      </form>
      <section className="tool-output" aria-label="Calculation results" aria-live="polite">
        <h2>{result ? 'Your circuit' : 'From calculation to circuit'}</h2>
        {!result && <p className="tool-empty">Enter your inputs and calculate to see selected components, achieved results and editable circuit source.</p>}
        {result && <>
          <p className="tool-result-kind">Nominal analytical result · {series === 'exact' ? 'Exact values' : series.toUpperCase()}</p>
          <div className="tool-components">{Object.entries(result.components).map(([name, value]) =>
            <div key={name}><span>{name}</span><strong>{displayQuantity(value)}</strong>
              {result.ideal_components[name] && <small>Ideal: {displayQuantity(result.ideal_components[name])}</small>}
            </div>)}</div>
          <dl className="tool-measurements">{Object.keys(resultLabels).filter((name) => name in result.results).map((name) =>
            <div key={name}><dt>{resultLabels[name]}</dt><dd>{displayQuantity(result.results[name])}</dd></div>)}</dl>
          <div className="tool-actions">
            <button className="button button-primary" onClick={openInEditor}>Open in editor <ArrowRight size={16} /></button>
            <button className="button button-secondary" onClick={() => downloadTextFile(result.source, `${definition.slug}.kess`)}><Download size={16} /> Download .kess</button>
          </div>
          <p className="tool-caption">Run the generated simulation in the editor, then export the schematic or continue in KiCad/LTspice. Your previous browser circuit is retained under File → Restore previous circuit.</p>
          <details className="tool-source"><summary>Generated circuit source</summary><pre>{result.source}</pre></details>
        </>}
      </section>
    </div>
    <section className="tool-explanation"><h2>How this calculation works</h2>
      <code>{definition.formula}</code>
      {result ? <ul>{result.assumptions.map((assumption) => <li key={assumption}>{assumption}</li>)}</ul> : <p>{definition.assumption}</p>}
    </section>
    <section className="tool-cli"><h2><Terminal size={18} /> Use the same tool locally</h2>
      <p>These commands require a source build until the next CLI release; published 1.0.1 does not include them.
        {' '}Add <code>--format json</code> for an AI agent. <a href={`${import.meta.env.BASE_URL}install/`}>CLI installation guide</a>.</p>
      <pre>{cliCommand}</pre>
      <button className="button button-compact button-secondary" disabled={!result} onClick={() => {
        void navigator.clipboard.writeText(cliCommand).then(() => setNotice('Command copied.')).catch(() => setNotice('Select the command above and copy it manually.'));
      }}>Copy command</button><span className="tool-notice" role="status">{notice}</span>
    </section>
  </ToolsShell>;
}
