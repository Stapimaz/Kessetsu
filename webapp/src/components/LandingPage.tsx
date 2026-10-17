import {
  ArrowRight,
  Bot,
  Check,
  CircuitBoard,
  FileDown,
  Globe2,
  ShieldCheck,
  Terminal,
} from 'lucide-react';
import { useEffect } from 'react';
import productVersionSource from '../../../VERSION?raw';
import '../landing.css';
import { BrandWordmark } from './BrandWordmark';
import { WebHubPreview } from './WebHubPreview';

const exports = ['SVG', 'PNG', 'PDF', 'Schematic JSON', 'SPICE', 'KiCad', 'LTspice', 'BOM CSV', 'Handoff JSON'];
const productVersion = productVersionSource.trim();

export function LandingPage() {
  useEffect(() => {
    document.documentElement.dataset.theme = 'dark';
    document.title = 'Kessetsu — Circuit simulation, verification and schematics';
  }, []);

  return (
    <main className="landing-page">
      <header className="landing-header">
        <a className="landing-wordmark" href="./" aria-label="Kessetsu home">
          <BrandWordmark />
        </a>
        <nav className="landing-nav" aria-label="Main navigation">
          <a href="#modes">Ways to work</a>
          <a href="#agent-workflow">Agent workflow</a>
          <a href="#web-hub">Web Hub</a>
          <a href={`${import.meta.env.BASE_URL}install/`}>CLI &amp; MCP</a>
          <a href={`${import.meta.env.BASE_URL}docs/`}>Docs</a>
        </nav>
        <a className="button button-compact button-secondary" href="#editor">
          Open Web Hub <ArrowRight size={15} />
        </a>
      </header>

      <section className="landing-hero" aria-labelledby="hero-title">
        <div className="hero-copy">
          <p className="eyebrow">SPICE Simulation · Auto-Schematic · AI Agent Verification</p>
          <h1 id="hero-title">Circuit engineering you can execute.</h1>
          <p className="hero-lead">
            Run existing SPICE netlists (<code>.cir</code>, <code>.sp</code>) or readable <code>.kess</code> circuits.
            Get real Ngspice simulation, automatic connectivity-verified schematics, and hard <code>PASS / FAIL</code> requirement
            checks—in your browser or connected to your AI agent via <strong>MCP</strong> and the <strong>local CLI</strong>.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="#editor">
              <Globe2 size={18} /> Open Web Hub
            </a>
            <a className="button button-secondary" href={`${import.meta.env.BASE_URL}install/`}>
              <Terminal size={18} /> Install CLI &amp; MCP
            </a>
          </div>
          <div className="trust-line">
            <span><Check size={15} /> Native SPICE &amp; .kess</span>
            <span><Check size={15} /> Built-in MCP server (<code>kess mcp</code>)</span>
            <span><Check size={15} /> Real Ngspice simulation</span>
            <span><Check size={15} /> No account or cloud upload</span>
          </div>
        </div>

        <div className="hero-console" aria-label="Kessetsu agent verification example">
          <div className="console-bar">
            <span /><span /><span />
            <code>agent-loop.kess + stage.kessreq</code>
            <small className="console-bar-tag">kess mcp · kess test</small>
          </div>
          <div className="requirement-note">
            <small>LOCKED REQUIREMENT CONTRACT (.KESSREQ) · GENERIC MODELS</small>
            <p>Four-stage amplifier · 8 Ω load · output_power ≥ 1.8 W · gain ≈ 55 · device dissipation &lt; 2 W</p>
          </div>
          <pre><span className="prompt">$</span> kess test agent-loop.kess --requirements stage.kessreq{`\n`}
<span className="muted">iteration 01</span>  <span className="fail">FAIL</span>  output_power = 997.7 mW  <span className="muted">(requires ≥ 1.80 W)</span>{`\n`}
<span className="muted">agent action</span>  revises output stage load matching (16 Ω → 8 Ω){`\n`}
<span className="muted">iteration 02</span>  <span className="pass">PASS</span>  output_power = 1.995 W · max_pd = 1.41 W</pre>
          <div className="console-artifacts">
            <span>ERC: 0 warnings</span>
            <span>Auto-Schematic: SVG / PNG</span>
            <span>Exports: KiCad · SPICE · BOM</span>
          </div>
          <div className="console-result">
            <span><Check size={15} /> 12 / 12 encoded assertions passed</span>
            <small>kessetsu.cli.v1 · stdio MCP</small>
          </div>
        </div>
      </section>

      <section className="landing-section modes-section" id="modes" aria-labelledby="modes-title">
        <div className="section-heading">
          <p className="eyebrow">Three ways to use Kessetsu</p>
          <h2 id="modes-title">Browser workspace, AI tool server, or terminal CLI.</h2>
          <p>
            Every interface runs the exact same Rust engineering core. Start visually in the browser, or install the single{' '}
            <code>kess</code> binary to get both the command-line tool and the Model Context Protocol (MCP) server.
          </p>
        </div>
        <div className="modes-grid">
          <article className="mode-card">
            <div className="mode-card-top">
              <span className="mode-badge"><Globe2 size={14} /> Browser · Zero install</span>
              <h3>Interactive Web Hub</h3>
              <p>
                Write <code>.kess</code> or import existing SPICE netlists directly in your browser. Inspect automatic schematics,
                explore interactive Bode and transient plots, and export engineering files immediately.
              </p>
            </div>
            <div className="mode-card-foot">
              <code>kessetsu.com/#editor</code>
              <a className="text-link" href="#editor">Open Web Hub <ArrowRight size={14} /></a>
            </div>
          </article>

          <article className="mode-card is-highlighted">
            <div className="mode-card-top">
              <span className="mode-badge"><Bot size={14} /> AI Tool Connector · MCP</span>
              <h3>Built-in MCP Server (<code>kess mcp</code>)</h3>
              <p>
                Plug Kessetsu directly into Claude Code, Claude Desktop, Cursor, Windsurf, or VS Code. Instead of guessing shell flags,
                your AI calls native tools to compile, simulate, check <code>.kessreq</code> limits, and render schematics.
              </p>
            </div>
            <div className="mode-card-foot">
              <code>claude mcp add kessetsu -- kess mcp</code>
              <a className="text-link" href={`${import.meta.env.BASE_URL}install/`}>Set up MCP &amp; CLI <ArrowRight size={14} /></a>
            </div>
          </article>

          <article className="mode-card">
            <div className="mode-card-top">
              <span className="mode-badge"><Terminal size={14} /> Terminal &amp; CI · CLI</span>
              <h3>Local CLI (<code>kess test</code>)</h3>
              <p>
                Run deterministic ERC checks, Ngspice simulations, parameter sweeps, and requirement verification from your terminal,
                scripts, or GitHub Actions pipelines with structured JSON output.
              </p>
            </div>
            <div className="mode-card-foot">
              <code>kess test amp.cir --requirements amp.kessreq</code>
              <a className="text-link" href={`${import.meta.env.BASE_URL}docs/reference/cli/`}>CLI reference <ArrowRight size={14} /></a>
            </div>
          </article>
        </div>
      </section>

      <section className="landing-section workflow-section" id="agent-workflow" aria-labelledby="workflow-title">
        <div className="section-heading">
          <p className="eyebrow">Closed-loop verification</p>
          <h2 id="workflow-title">Give your agent requirements, not blind trust.</h2>
          <p>An AI agent proposes and revises the circuit; Kessetsu enforces real electrical rules and Ngspice measurements at every step.</p>
        </div>
        <ol className="workflow-grid">
          <li>
            <span className="step-number">01</span>
            <strong>Specify (.kessreq)</strong>
            <p>Pin acceptance criteria—gain, cutoff, power, ripple, or dissipation—in a separate requirement file the design cannot rewrite.</p>
          </li>
          <li>
            <span className="step-number">02</span>
            <strong>Design (.kess or SPICE)</strong>
            <p>Work with standard SPICE netlists (<code>.cir</code>, <code>.sp</code>) or readable <code>.kess</code> source through <code>kess mcp</code> or the CLI.</p>
          </li>
          <li>
            <span className="step-number">03</span>
            <strong>Check and test</strong>
            <p>ERC catches floating pins and shorted sources before Ngspice runs; structured PASS/FAIL assertions report exact measured values.</p>
          </li>
          <li>
            <span className="step-number">04</span>
            <strong>Review and hand off</strong>
            <p>Render a connectivity-verified schematic (PNG/SVG) for human review and export to KiCad, LTspice, SPICE, or BOM CSV.</p>
          </li>
        </ol>
        <p className="provider-note">
          <ShieldCheck size={18} />
          <span>Kessetsu is AI-provider independent. Connect any agent via <code>kess mcp</code> or the versioned JSON CLI; see the <a href={`${import.meta.env.BASE_URL}docs/guides/agent-workflow/`}>agent workflow guide</a>.</span>
        </p>
      </section>

      <section className="landing-section hub-section" id="web-hub" aria-labelledby="hub-title">
        <div className="hub-copy">
          <p className="eyebrow">Zero-install browser workspace</p>
          <h2 id="hub-title">The complete workflow, in your browser.</h2>
          <p>
            Edit <code>.kess</code> or convert SPICE netlists, inspect a clean schematic, switch seamlessly between schematic and
            high-readability simulation plots, and export engineering artifacts without an account.
          </p>
          <ul className="check-list">
            <li><Check size={16} /> Direct SPICE netlist import &amp; live conversion preview</li>
            <li><Check size={16} /> Connectivity-verified automatic schematics</li>
            <li><Check size={16} /> Interactive OP, DC, AC &amp; transient plots with PASS/FAIL checks</li>
          </ul>
          <p className="scope-note">
            Work with analog circuits, reusable stages and local device models.{' '}
            <a href={`${import.meta.env.BASE_URL}docs/guides/tutorial/`}>Follow the circuit tutorial</a> or{' '}
            <a href={`${import.meta.env.BASE_URL}docs/guides/spice-import/`}>bring an existing SPICE netlist</a>.
          </p>
          <details className="hub-advanced">
            <summary>Sweeps, tolerance studies &amp; measured CSV comparison</summary>
            <p>Use <a href={`${import.meta.env.BASE_URL}docs/guides/parameter-studies/`}>parameter studies</a> to compare component choices, loads, temperatures and tolerances. Keep failed cases alongside successful ones.</p>
            <p>Bring measured CSV data into <a href={`${import.meta.env.BASE_URL}docs/guides/research-data/`}>research comparison</a> to inspect residuals against a simulation or reference dataset.</p>
          </details>
        </div>
        <WebHubPreview />
      </section>

      <section className="landing-section engine-section" id="outputs" aria-labelledby="engine-title">
        <div className="section-heading compact-heading">
          <p className="eyebrow">Shared engineering core</p>
          <h2 id="engine-title">Verify once. Export anywhere.</h2>
        </div>
        <div className="engine-flow" aria-label="Kessetsu shared Core architecture">
          <div className="surface-card"><Globe2 size={20} /><span>Browser</span><strong>Web Hub</strong></div>
          <span className="flow-arrow">→</span>
          <div className="core-card"><CircuitBoard size={22} /><span>Kessetsu Rust Core</span><small>SPICE &amp; .kess · ERC · Ngspice · .kessreq · Auto-Layout</small></div>
          <span className="flow-arrow">←</span>
          <div className="surface-card"><Terminal size={20} /><span>AI Agents &amp; CI</span><strong>MCP Server + CLI</strong></div>
        </div>
        <div className="export-row">
          <FileDown size={18} aria-hidden="true" />
          {exports.map((format) => <span key={format}>{format}</span>)}
        </div>
      </section>

      <footer className="landing-footer" role="contentinfo">
        <div>
          <strong>Kessetsu {productVersion}</strong>
          <span>© 2026 Stapimaz · AGPL-3.0-only · No telemetry</span>
        </div>
        <nav aria-label="Footer navigation">
          <a href={`${import.meta.env.BASE_URL}docs/`}>Docs</a>
          <a href={`${import.meta.env.BASE_URL}install/`}>Install CLI &amp; MCP</a>
          <a href={`${import.meta.env.BASE_URL}changelog/`}>Changelog</a>
          <a href={`${import.meta.env.BASE_URL}tools/`}>Calculators</a>
          <a href="https://github.com/Stapimaz/Kessetsu">Source</a>
          <a href="https://github.com/Stapimaz/Kessetsu/issues">Feedback</a>
        </nav>
      </footer>
    </main>
  );
}

