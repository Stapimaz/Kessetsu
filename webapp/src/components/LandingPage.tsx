import {
  ArrowRight,
  Check,
  CircuitBoard,
  Code2,
  FileDown,
  Globe2,
  ShieldCheck,
  Terminal,
} from 'lucide-react';
import { useEffect } from 'react';
import '../landing.css';
import { WebHubPreview } from './WebHubPreview';

const exports = ['SVG', 'PNG', 'PDF', 'Schematic JSON', 'SPICE', 'KiCad', 'LTspice'];

export function LandingPage() {
  useEffect(() => {
    document.documentElement.dataset.theme = 'dark';
    document.title = 'Kessetsu — Circuit engineering, executable';
  }, []);

  return (
    <main className="landing-page">
      <header className="landing-header">
        <a className="landing-wordmark" href="./" aria-label="Kessetsu home">
          <CircuitBoard aria-hidden="true" size={22} />
          <span>KESSETSU</span>
        </a>
        <nav className="landing-nav" aria-label="Main navigation">
          <a href="#agent-workflow">Agent workflow</a>
          <a href="#web-hub">Web Hub</a>
          <a href="#outputs">Exports</a>
          <a href="https://github.com/Stapimaz/Kessetsu/blob/main/docs/README.md">Docs</a>
        </nav>
        <a className="button button-compact button-secondary" href="#editor">
          Open Web Hub <ArrowRight size={15} />
        </a>
      </header>

      <section className="landing-hero" aria-labelledby="hero-title">
        <div className="hero-copy">
          <p className="eyebrow">Executable circuit engineering</p>
          <h1 id="hero-title">Circuit engineering you can execute.</h1>
          <p className="hero-lead">
            Design circuits in the browser or give an AI agent measurable requirements. Kessetsu compiles,
            simulates, verifies, and exports every iteration through the same engineering core.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="#editor">
              <Globe2 size={18} /> Open Web Hub
            </a>
            <a className="button button-secondary" href="https://github.com/Stapimaz/Kessetsu/releases">
              <Terminal size={18} /> Install the CLI
            </a>
          </div>
          <div className="trust-line">
            <span><Check size={15} /> No account required</span>
            <span><Check size={15} /> Real SPICE simulation</span>
            <span><Check size={15} /> Local browser runtime</span>
          </div>
        </div>

        <div className="hero-console" aria-label="Kessetsu agent verification example">
          <div className="console-bar">
            <span /><span /><span />
            <code>agent-loop.kess</code>
          </div>
          <div className="requirement-note">
            <small>ENGINEERING REQUIREMENT</small>
            <p>Four-stage amplifier · 8 Ω load · ≈2 W output · gain ≈55 · device dissipation &lt;2 W</p>
          </div>
          <pre><span className="prompt">$</span> kess test - --format json{`\n`}
<span className="muted">iteration 01</span>  <span className="fail">FAIL</span>  output_power = 997.7 mW{`\n`}
<span className="muted">agent revision</span>  load 16 Ω → 8 Ω{`\n`}
<span className="muted">iteration 02</span>  <span className="pass">PASS</span>  output_power = 1.995 W</pre>
          <div className="console-result">
            <span><Check size={15} /> 12 / 12 requirements passed</span>
            <small>kessetsu.cli.v1</small>
          </div>
        </div>
      </section>

      <section className="landing-section workflow-section" id="agent-workflow" aria-labelledby="workflow-title">
        <div className="section-heading">
          <p className="eyebrow">Agent workflow</p>
          <h2 id="workflow-title">Give your agent requirements, not blind trust.</h2>
          <p>A capable agent designs and revises the circuit. Kessetsu gives it deterministic engineering feedback at every step.</p>
        </div>
        <ol className="workflow-grid">
          <li>
            <span className="step-number">01</span>
            <strong>Specify</strong>
            <p>Describe topology constraints, load, gain, power, stress, or any other measurable requirement.</p>
          </li>
          <li>
            <span className="step-number">02</span>
            <strong>Design</strong>
            <p>The agent writes a complete `.kess` circuit and calls the CLI directly through stdin.</p>
          </li>
          <li>
            <span className="step-number">03</span>
            <strong>Verify and revise</strong>
            <p>Structured diagnostics and assertion results expose exactly what failed, without scraping terminal prose.</p>
          </li>
          <li>
            <span className="step-number">04</span>
            <strong>Continue anywhere</strong>
            <p>Keep the verified source or export the schematic, SPICE netlist, KiCad, and LTspice artifacts.</p>
          </li>
        </ol>
        <p className="provider-note">
          <ShieldCheck size={18} /> Kessetsu is AI-provider independent. Your agent uses a versioned CLI contract; the circuit engine stays the same.
        </p>
      </section>

      <section className="landing-section hub-section" id="web-hub" aria-labelledby="hub-title">
        <div className="hub-copy">
          <p className="eyebrow">Prefer to work directly?</p>
          <h2 id="hub-title">The complete workflow, in your browser.</h2>
          <p>
            Edit source, inspect a professional schematic, run real OP/transient/AC/DC simulation, evaluate
            requirements, and download engineering artifacts without an account or installation.
          </p>
          <ul className="check-list">
            <li><Check size={16} /> The same canonical Rust Core as the CLI</li>
            <li><Check size={16} /> Connectivity-verified automatic schematics</li>
            <li><Check size={16} /> Interactive results and explicit PASS/FAIL assertions</li>
          </ul>
          <a className="text-link" href="#editor">Open the browser workspace <ArrowRight size={16} /></a>
        </div>
        <WebHubPreview />
      </section>

      <section className="landing-section engine-section" id="outputs" aria-labelledby="engine-title">
        <div className="section-heading compact-heading">
          <p className="eyebrow">Shared engineering core</p>
          <h2 id="engine-title">Verify once. Continue anywhere.</h2>
        </div>
        <div className="engine-flow" aria-label="Kessetsu shared Core architecture">
          <div className="surface-card"><Globe2 size={20} /><span>Human</span><strong>Web Hub</strong></div>
          <span className="flow-arrow">→</span>
          <div className="core-card"><CircuitBoard size={22} /><span>Kessetsu Core</span><small>Compile · ERC · SPICE · Assert · Layout</small></div>
          <span className="flow-arrow">←</span>
          <div className="surface-card"><Terminal size={20} /><span>AI agent</span><strong>CLI + JSON</strong></div>
        </div>
        <div className="export-row">
          <FileDown size={18} aria-hidden="true" />
          {exports.map((format) => <span key={format}>{format}</span>)}
        </div>
      </section>

      <section className="final-cta">
        <div>
          <p className="eyebrow">Ready when you are</p>
          <h2>Start with a circuit. Leave with verified engineering artifacts.</h2>
        </div>
        <div className="hero-actions">
          <a className="button button-primary" href="#editor">Open Web Hub <ArrowRight size={17} /></a>
          <a className="button button-secondary" href="https://github.com/Stapimaz/Kessetsu">
            <Code2 size={17} /> View source
          </a>
        </div>
      </section>

      <footer className="landing-footer">
        <span>Kessetsu © 2026 Stapimaz</span>
        <span>AGPL-3.0-only · No telemetry in the first release</span>
      </footer>
    </main>
  );
}
