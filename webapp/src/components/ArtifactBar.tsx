import { Download, X } from 'lucide-react';
import { useRef, useState } from 'react';
import type { ExportArtifact, ExportDescriptor, ExportFormat, ModelManifest } from '../domain';

interface Props {
  spice: string;
  models: ModelManifest | null;
  enabled: boolean;
  capabilities: ExportDescriptor[];
  message: string;
  onExport: (format: ExportFormat) => ExportArtifact;
}

const formatDescriptions: Record<ExportFormat, string> = {
  svg: 'Scalable vector image', png: 'Raster image', pdf: 'Print-ready document',
  schematic_json: 'Structured schematic data', spice: 'Simulation netlist',
  kicad: 'Editable KiCad schematic', ltspice: 'Editable LTspice schematic',
};

function downloadArtifact(artifact: ExportArtifact) {
  const blob = new Blob([new Uint8Array(artifact.bytes)], { type: artifact.mime_type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `circuit.${artifact.extension}`;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function ArtifactBar({ spice, models, enabled, capabilities, message, onExport }: Props) {
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDialogElement>(null);
  const exportOne = (format: ExportFormat) => {
    try {
      setError('');
      downloadArtifact(onExport(format));
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  return (
    <aside className="artifact-bar" aria-label="Exports">
      <button disabled={!enabled} onClick={() => dialogRef.current?.showModal()} aria-haspopup="dialog"><Download size={15} /> Export</button>
      <dialog ref={dialogRef} className="export-dialog" aria-labelledby="export-title">
      <header><div><h2 id="export-title">Export circuit</h2><p>Choose a format to download.</p></div>
        <button aria-label="Close export" onClick={() => dialogRef.current?.close()}><X size={18} /></button>
      </header>
      <div className="export-buttons" data-testid="export-capabilities">
        {capabilities.map((descriptor) => (
          <button
            key={descriptor.format}
            disabled={!enabled}
            onClick={() => exportOne(descriptor.format)}
            title={`${descriptor.capability.editable ? 'Editable' : 'View-only'} · ${descriptor.capability.preserves_connectivity ? 'connectivity-safe' : 'visual only'}`}
            data-export-format={descriptor.format}
          >
            <strong>{descriptor.label}</strong>
            <small>{formatDescriptions[descriptor.format]}</small>
          </button>
        ))}
      </div>
      <details className="export-details">
        <summary>Format details</summary>
        <div className="export-popover">
          {capabilities.map((descriptor) => (
            <article key={descriptor.format}>
              <strong>{descriptor.label}</strong>
              <span>
                {descriptor.capability.editable ? 'editable' : 'view-only'} ·{' '}
                {descriptor.capability.machine_readable ? 'machine-readable' : 'visual'} ·{' '}
                {descriptor.capability.preserves_connectivity ? 'connectivity preserved' : 'visual projection'}
              </span>
              <small>
                models {descriptor.capability.preserves_models ? 'preserved' : 'not represented'} · analyses{' '}
                {descriptor.capability.preserves_analysis ? 'preserved' : 'not represented'}
              </small>
            </article>
          ))}
        </div>
      </details>
      {(error || message) && <span className={error ? 'export-status export-error' : 'export-status'} role="status">{error || message}</span>}
      </dialog>
      <details className="circuit-details"><summary>Details</summary><div className="circuit-details-popover">
      <details className="model-details" data-testid="model-manifest" data-manifest={models ? JSON.stringify(models) : ''}>
        <summary>Models ({models?.models.length ?? 0})</summary>
        <div className="model-popover">
          <p>Models are selected only through typed Kessetsu declarations or exact package imports.</p>
          {models?.models.map((model) => (
            <article key={model.name}>
              <strong>{model.name}</strong>
              <span>{model.provenance.version} · {model.provenance.license} · {model.provenance.simulator}</span>
              <small>{model.provenance.source}</small>
            </article>
          ))}
        </div>
      </details>
      <details className="spice-details">
        <summary>Generated SPICE Netlist</summary>
        <pre>{spice || 'Waiting for a valid circuit…'}</pre>
      </details>
      <details className="legal-details">
        <summary>Legal</summary>
        <div className="legal-popover">
          <strong>Kessetsu © 2026 Stapimaz</strong>
          <span>AGPL-3.0-only free software, provided without warranty.</span>
          <span>
            <a href="https://github.com/Stapimaz/Kessetsu" target="_blank" rel="noreferrer">Corresponding Source</a>
            {' · '}
            <a href={`${import.meta.env.BASE_URL}LICENSE.txt`} target="_blank" rel="noreferrer">Full license</a>
          </span>
        </div>
      </details>
      </div></details>
    </aside>
  );
}
