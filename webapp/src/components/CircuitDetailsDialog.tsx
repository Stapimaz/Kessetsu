import { X } from 'lucide-react';
import { useEffect, useRef } from 'react';
import type { ModelManifest } from '../domain';

interface Props {
  open: boolean;
  spice: string;
  models: ModelManifest | null;
  onClose: () => void;
}

export function CircuitDetailsDialog({ open, spice, models, onClose }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog ref={dialogRef} className="app-dialog circuit-details-dialog" aria-labelledby="circuit-details-title" onClose={onClose}>
      <header>
        <div>
          <h2 id="circuit-details-title">Circuit details</h2>
          <p>Inspect resolved models and the generated simulation netlist.</p>
        </div>
        <button aria-label="Close circuit details" onClick={() => dialogRef.current?.close()}><X size={18} /></button>
      </header>
      <section className="model-details" data-testid="model-manifest" data-manifest={models ? JSON.stringify(models) : ''}>
        <div className="dialog-section-heading">
          <h3>Resolved models</h3><span>{models?.models.length ?? 0}</span>
        </div>
        <p>Models are selected only through typed Kessetsu declarations or exact package imports.</p>
        <div className="model-list">
          {models?.models.length
            ? models.models.map((model) => (
              <article key={model.name}>
                <strong>{model.name}</strong>
                <span>{model.provenance.version} · {model.provenance.license} · {model.provenance.simulator}</span>
                <small>{model.provenance.source}</small>
              </article>
            ))
            : <span className="empty-detail">No packaged models are resolved for this circuit.</span>}
        </div>
      </section>
      <section className="spice-details">
        <div className="dialog-section-heading"><h3>Generated SPICE Netlist</h3></div>
        <pre>{spice || 'Waiting for a valid circuit…'}</pre>
      </section>
    </dialog>
  );
}
