import { Download, FileCode2 } from 'lucide-react';
import { useState } from 'react';
import type { ExportArtifact, ExportDescriptor, ExportFormat, ModelManifest } from '../domain';

interface Props {
  spice: string;
  models: ModelManifest | null;
  enabled: boolean;
  capabilities: ExportDescriptor[];
  message: string;
  onExport: (format: ExportFormat) => ExportArtifact;
}

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
      <div className="header-title"><FileCode2 size={16} /><strong>Export</strong></div>
      <div className="export-buttons" data-testid="export-capabilities">
        {capabilities.map((descriptor) => (
          <button
            key={descriptor.format}
            disabled={!enabled}
            onClick={() => exportOne(descriptor.format)}
            title={`${descriptor.capability.editable ? 'Editable' : 'View-only'} · ${descriptor.capability.preserves_connectivity ? 'connectivity-safe' : 'visual only'}`}
            data-export-format={descriptor.format}
          >
            <Download size={13} /> {descriptor.label}
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
      <details className="model-details" data-testid="model-manifest" data-manifest={models ? JSON.stringify(models) : ''}>
        <summary>Models ({models?.models.length ?? 0})</summary>
        <div className="model-popover">
          <p>Models are selected only through typed NetLang declarations or exact package imports.</p>
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
        <pre>{spice || 'Geçerli devre bekleniyor…'}</pre>
      </details>
    </aside>
  );
}
