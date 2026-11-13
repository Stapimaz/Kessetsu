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
            Write a circuit or paste an existing SPICE netlist. Kessetsu simulates it with Ngspice, draws the schematic
            automatically, and checks whether your engineering requirements actually pass—in your browser, connected to
            your AI coding tool, or from the terminal.
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
            <span><Check size={15} /> Standard SPICE (.cir, .sp) &amp; .kess</span>
            <span><Check size={15} /> Works with Claude, Cursor &amp; VS Code</span>
            <span><Check size={15} /> Real Ngspice simulation</span>
            <span><Check size={15} /> Free, local &amp; no account</span>
          </div>
        </div>

        <div className="hero-console" aria-label="Kessetsu agent verification example">
          <div className="console-bar">
            <span /><span /><span />
            <code>rc_low_pass.kess · 1 kHz filter</code>
            <small className="console-bar-tag">Browser · AI · CLI</small>
          </div>
          <div className="requirement-note">
            <small>1. CIRCUIT &amp; TARGET REQUIREMENTS</small>
            <p>First-order RC low-pass filter (R1 = 1 kΩ, C1 = 159.15 nF) · Verify −3 dB cutoff ≈ 1 kHz and −45° phase</p>
          </div>
          <pre><span className="prompt">$</span> kess test rc_low_pass.kess{`\n`}
<span className="muted">check 01</span>  <span className="pass">PASS</span>  dc passband gain  = 1.000      <span className="muted">(requires &gt; 0.99)</span>{`\n`}
<span className="muted">check 02</span>  <span className="pass">PASS</span>  -3 dB cutoff      = 1.000 kHz  <span className="muted">(requires 990 Hz .. 1.01 kHz)</span>{`\n`}
<span className="muted">check 03</span>  <span className="pass">PASS</span>  phase @ 1.000 kHz = -45.0°     <span className="muted">(requires -46° .. -44°)</span></pre>
          <div className="console-artifacts">
            <span>Connections: 0 warnings</span>
            <span>Auto-Schematic: SVG / PNG</span>
            <span>Exports: KiCad · LTspice · SPICE · BOM</span>
          </div>
          <div className="console-result">
            <span><Check size={15} /> 3 / 3 engineering requirements passed</span>
            <small>Ngspice verified · Ready to export</small>
          </div>
        </div>
      </section>

      <section className="landing-section modes-section" id="modes" aria-labelledby="modes-title">
        <div className="section-heading">
          <p className="eyebrow">Three ways to use Kessetsu</p>
          <h2 id="modes-title">Start in your browser, connect your AI, or run in the terminal.</h2>
          <p>
            Every workflow uses the same Rust simulation and verification core. Start immediately in your browser with zero setup, or install Kessetsu once for your AI coding assistant and terminal.
          </p>
        </div>
        <div className="modes-grid">
          <article className="mode-card">
            <div className="mode-card-top">
              <span className="mode-badge"><Globe2 size={14} /> Browser · Web Hub</span>
              <h3>Use it in your browser</h3>
              <p>
                Open a starter example, write <code>.kess</code>, or import an existing SPICE netlist. Inspect the automatic schematic,
                explore interactive Bode and transient plots, and export to KiCad, LTspice, or SVG with zero installation.
              </p>
            </div>
            <div className="mode-card-foot">
              <code>kessetsu.com/#editor</code>
              <a className="text-link" href="#editor">Open Web Hub <ArrowRight size={14} /></a>
            </div>
          </article>

          <article className="mode-card is-highlighted">
            <div className="mode-card-top">
              <span className="mode-badge"><Bot size={14} /> AI Assistant · MCP (kess mcp)</span>
              <h3>Use it with your AI</h3>
              <p>
                Connect Claude Code, Cursor, VS Code, Claude Desktop, or Windsurf. When you ask your AI to design or debug a circuit,
                it calls Kessetsu locally to run Ngspice and verify that your requirements pass.
              </p>
            </div>
            <div className="mode-card-foot">
              <code>claude mcp add kessetsu -- kess mcp</code>
              <a className="text-link" href={`${import.meta.env.BASE_URL}install/`}>Connect your AI tool <ArrowRight size={14} /></a>
            </div>
          </article>

          <article className="mode-card">
            <div className="mode-card-top">
              <span className="mode-badge"><Terminal size={14} /> Terminal &amp; CI · CLI (kess test)</span>
              <h3>Use it from the terminal</h3>
              <p>
                Check circuit connections, run Ngspice simulations, sweep component tolerances, and enforce pass/fail requirements from your command line,
                scripts, or GitHub Actions pipelines.
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
          <p>AI models can suggest circuits that look plausible on paper but fail electrically. Kessetsu puts real Ngspice simulation and locked pass/fail checks in the loop.</p>
        </div>
        <ol className="workflow-grid">
          <li>
            <span className="step-number">01</span>
            <strong>Set the goal (.kessreq)</strong>
            <p>Write your target limits—gain, cutoff, ripple, or power dissipation—in a separate requirement file so the designing AI cannot quietly weaken the spec to make a test pass.</p>
          </li>
          <li>
            <span className="step-number">02</span>
            <strong>Write or import a circuit</strong>
            <p>Work with standard SPICE netlists (<code>.cir</code>, <code>.sp</code>) or readable <code>.kess</code> source in the browser, through your AI assistant, or from the CLI.</p>
          </li>
          <li>
            <span className="step-number">03</span>
            <strong>Check connections &amp; simulate</strong>
            <p>Electrical Rules Check (ERC) catches floating pins and shorted sources before simulation; Ngspice then runs and grades every requirement as PASS or FAIL with exact measured values.</p>
          </li>
          <li>
            <span className="step-number">04</span>
            <strong>Inspect &amp; export</strong>
            <p>Review the connectivity-verified schematic and waveform plots, then export to KiCad, LTspice, SPICE, SVG, PNG, PDF, or BOM CSV.</p>
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

