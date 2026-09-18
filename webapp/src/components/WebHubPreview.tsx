import { Activity, Check, CircuitBoard, Code2 } from 'lucide-react';
import { BrandWordmark } from './BrandWordmark';
import previewSource from '../../../examples/rc_low_pass.kess?raw';
import previewSvg from '../assets/rc-preview.svg?raw';

export function WebHubPreview() {
  return (
    <figure className="hub-preview" aria-label="Kessetsu Web Hub product preview">
      <div className="preview-header">
        <BrandWordmark />
        <small>Connectivity verified</small>
      </div>
      <div className="preview-workspace">
        <section className="preview-source" aria-label="Circuit source preview">
          <strong><Code2 size={13} /> Source</strong>
          <pre>{previewSource.replace(/^\/\/.*\n/, '').trim()}</pre>
        </section>
        <div className="preview-output">
          <section className="preview-schematic" aria-label="Verified RC schematic preview">
            <strong><CircuitBoard size={13} /> Schematic</strong>
            {/* Trusted checked-in Core export, never user-supplied markup. */}
            <div className="preview-generated-schematic" dangerouslySetInnerHTML={{ __html: previewSvg }} />
          </section>
          <section className="preview-results" aria-label="Simulation result preview">
            <strong><Activity size={13} /> Simulation</strong>
            <div>
              <span><Check size={12} /> 5 / 5 requirements passed</span>
              <code>cutoff = 1.000 kHz</code>
            </div>
          </section>
        </div>
      </div>
      <figcaption>One source. One Core. The same verified result in Web and CLI.</figcaption>
    </figure>
  );
}
