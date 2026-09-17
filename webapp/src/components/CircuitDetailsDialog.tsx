import { X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { ModelManifest } from '../domain';
import type { LocalModelRequirement } from '../hooks/useKessetsuWorkspace';
import comparatorModelUrl from '../../../examples/models/comparator.lib?url&no-inline';
import memristorModelUrl from '../../../examples/models/memristor.lib?url&no-inline';
import memristorNoticeUrl from '../../../examples/models/MEMRISTOR_NOTICE.md?url&no-inline';

interface Props {
  open: boolean;
  spice: string;
  models: ModelManifest | null;
  onClose: () => void;
  resources: LocalModelRequirement[];
  boundResources: string[];
  onBindFile: (resource: string, file: File) => Promise<void>;
  onClearFiles: () => void;
}

export function CircuitDetailsDialog({ open, spice, models, onClose, resources, boundResources, onBindFile, onClearFiles }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [resourceError, setResourceError] = useState('');

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
      {resources.length > 0 && <section className="model-details">
        <div className="dialog-section-heading"><h3>Local model files</h3>
          <button onClick={onClearFiles} disabled={boundResources.length === 0}>Clear files</button>
        </div>
        <p>Choose the exact files named by your source. Files stay in memory on this device: they are not uploaded, saved with the circuit or included in shared links. Select them again after reloading.</p>
        <p>Portable Ngspice analog libraries can simulate here. PSpice compatibility and unsupported model constructs require the CLI. Exports retain file references; keep matching files beside SPICE/LTspice exports.</p>
        <p>Files for the provided examples: <a href={comparatorModelUrl} download="comparator.lib">comparator.lib</a> · <a href={memristorModelUrl} download="memristor.lib">memristor.lib</a> · <a href={memristorNoticeUrl} download="MEMRISTOR_NOTICE.md">memristor attribution and license</a>. Download, then choose the matching file below. Custom declarations need their own exact files.</p>
        <div className="model-list">{resources.map((item) => <article key={`${item.model}:${item.resource}`}>
          <strong>{item.model} · {item.resource}</strong>
          <span>{item.simulator === 'ngspice_ps' ? 'Native-only compatibility' : 'Browser profile checked before simulation'} · {boundResources.includes(item.resource) ? 'File selected; see compilation status for validation' : 'File required'}</span>
          <small>SHA-256: {item.sha256}</small>
          <label>Choose model file
            <input type="file" aria-label={`Model file for ${item.model}`} onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (!file) return;
              setResourceError('');
              void onBindFile(item.resource, file).catch((error: unknown) => setResourceError(String(error)));
            }} />
          </label>
        </article>)}</div>
        {resourceError && <p role="alert">{resourceError}</p>}
      </section>}
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
