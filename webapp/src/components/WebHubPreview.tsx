import { Activity, Check, CircuitBoard, Code2, Columns2 } from 'lucide-react';
import { BrandWordmark } from './BrandWordmark';
import previewSource from '../../../examples/rc_low_pass.kess?raw';
import previewSvg from '../assets/rc-preview.svg?raw';

export function WebHubPreview() {
  return (
    <figure className="hub-preview" aria-label="Kessetsu Web Hub product preview">
      <div className="preview-header">
        <div className="preview-header-left">
          <BrandWordmark />
          <span className="preview-doc-pill">rc_low_pass.kess</span>
        </div>
        <div className="preview-mode-tabs" aria-hidden="true">
          <span><CircuitBoard size={10} /> Schematic</span>
          <span><Activity size={10} /> Simulation</span>
          <span className="is-active"><Columns2 size={10} /> Split</span>
        </div>
        <small><Check size={10} /> Source valid · ERC pass</small>
      </div>
      <div className="preview-workspace">
        <section className="preview-source" aria-label="Circuit source preview">
          <strong><Code2 size={13} /> Source (.kess or SPICE)</strong>
          <pre>{previewSource.replace(/^\/\/.*\n/, '').trim()}</pre>
        </section>
        <div className="preview-output">
          <section className="preview-schematic" aria-label="Verified RC schematic preview">
            <strong>
              <span><CircuitBoard size={13} /> Auto-Schematic</span>
              <span className="preview-panel-badge">Connectivity verified</span>
            </strong>
            {/* Trusted checked-in Core export, never user-supplied markup. */}
            <div className="preview-generated-schematic" dangerouslySetInnerHTML={{ __html: previewSvg }} />
          </section>
          <section className="preview-results" aria-label="Simulation result preview">
            <strong>
              <span><Activity size={13} /> Ngspice Simulation &amp; Assertions</span>
              <span className="preview-panel-badge is-pass">5 / 5 PASS</span>
            </strong>
            <div className="preview-metrics-bar">
              <span><Check size={12} /> All 5 requirements verified</span>
              <div className="preview-metric-pills">
                <code>cutoff = 1.000 kHz</code>
                <code>dc_gain = -0.004 dB</code>
                <code>phase@1kHz = -45.0°</code>
              </div>
            </div>
          </section>
        </div>
      </div>
      <figcaption>One deterministic Rust Core powers the browser Web Hub, the local CLI, and the built-in MCP server.</figcaption>
    </figure>
  );
}

