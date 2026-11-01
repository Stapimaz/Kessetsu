import { Check, Copy, X } from 'lucide-react';
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
  const [copiedSpice, setCopiedSpice] = useState(false);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      setCopiedSpice(false);
      dialog.showModal();
    }
    if (!open && dialog.open) dialog.close();
  }, [open]);

  const usesBundledExampleModels = resources.some((item) => item.resource.endsWith('comparator.lib') || item.resource.endsWith('memristor.lib'));

  const copySpice = async () => {
    if (!spice) return;
    try {
      await navigator.clipboard.writeText(spice);
      setCopiedSpice(true);
      globalThis.setTimeout(() => setCopiedSpice(false), 2000);
    } catch {
      // Fallback: user can still select from <pre>.
    }
  };

  return (
    <dialog ref={dialogRef} className="app-dialog circuit-details-dialog" aria-labelledby="circuit-details-title" onClose={onClose}>
      <header>
        <div>
          <h2 id="circuit-details-title">Circuit details</h2>
          <p>Inspect resolved models, bind local model files, and view the generated SPICE netlist.</p>
        </div>
        <button aria-label="Close circuit details" onClick={() => dialogRef.current?.close()}><X size={18} /></button>
      </header>
      <p className="dialog-help-link"><a href={`${import.meta.env.BASE_URL}docs/reference/model-catalog/`} target="_blank" rel="noreferrer">Model catalog, local files and simulation limits</a></p>
      {resources.length > 0 && <section className="model-details">
        <div className="dialog-section-heading"><h3>Local model files</h3>
          <button className="secondary-button" onClick={onClearFiles} disabled={boundResources.length === 0}>Clear files</button>
        </div>
        <p>Select the exact model files referenced by your circuit. Files stay in memory in this browser tab and are never uploaded or included in shared links.</p>
        {usesBundledExampleModels && <p>Example model files: <a href={comparatorModelUrl} download="comparator.lib">comparator.lib</a> · <a href={memristorModelUrl} download="memristor.lib">memristor.lib</a> (<a href={memristorNoticeUrl} download="MEMRISTOR_NOTICE.md">attribution &amp; license</a>).</p>}
        <div className="model-list">{resources.map((item) => <article key={`${item.model}:${item.resource}`}>
          <strong>{item.model} · {item.resource}</strong>
          <span>{item.simulator === 'ngspice_ps' ? 'Native-only: use the installed CLI with Ngspice; selecting a file will not enable browser simulation' : 'Browser profile checked before simulation'} · {boundResources.includes(item.resource) ? 'File selected; see compilation status for validation' : 'File required'}</span>
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
        <div className="model-list">
          {models?.models.length
            ? models.models.map((model) => (
              <article key={model.name}>
                <strong>{model.name}</strong>
                <span>{model.provenance.version} · {model.provenance.license} · {model.provenance.simulator}</span>
                <small>{model.provenance.source}</small>
                {!!model.external?.instance_parameters?.length && <small>
                  Instance parameters: {model.external.instance_parameters.map((parameter) => parameter.name).join(', ')}
                </small>}
              </article>
            ))
            : <span className="empty-detail">No packaged models resolved for this circuit.</span>}
        </div>
      </section>
      <section className="spice-details">
        <div className="dialog-section-heading">
          <h3>Generated SPICE Netlist</h3>
          {spice && <button type="button" className="secondary-button spice-copy-button" onClick={() => void copySpice()}>
            {copiedSpice ? <><Check size={13} /> Copied</> : <><Copy size={13} /> Copy netlist</>}
          </button>}
        </div>
        <pre>{spice || 'Waiting for a valid circuit…'}</pre>
      </section>
    </dialog>
  );
}
