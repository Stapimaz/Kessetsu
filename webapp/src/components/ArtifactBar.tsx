import { Download, X } from 'lucide-react';
import { useRef, useState } from 'react';
import type { ExportArtifact, ExportDescriptor, ExportFormat } from '../domain';

interface Props {
  enabled: boolean;
  capabilities: ExportDescriptor[];
  message: string;
  filenameStem: string;
  onExport: (format: ExportFormat) => ExportArtifact;
}

const formatDescriptions: Record<ExportFormat, string> = {
  svg: 'Scalable vector image', png: 'Raster image', pdf: 'Print-ready document',
  schematic_json: 'Structured schematic data', spice: 'Simulation netlist',
  kicad: 'Editable KiCad schematic', ltspice: 'Editable LTspice schematic',
};

function downloadArtifact(artifact: ExportArtifact, filenameStem: string) {
  const blob = new Blob([new Uint8Array(artifact.bytes)], { type: artifact.mime_type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${filenameStem}.${artifact.extension}`;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function ArtifactBar({ enabled, capabilities, message, filenameStem, onExport }: Props) {
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDialogElement>(null);
  const exportOne = (format: ExportFormat) => {
    try {
      setError('');
      downloadArtifact(onExport(format), filenameStem);
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  return (
    <aside className="artifact-bar" aria-label="Exports">
      <button disabled={!enabled} onClick={() => dialogRef.current?.showModal()} aria-haspopup="dialog" aria-label="Export"><Download size={15} /><span>Export</span></button>
      <dialog ref={dialogRef} className="app-dialog export-dialog" aria-labelledby="export-title">
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
    </aside>
  );
}
