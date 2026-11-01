import { Check, Copy, Upload, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { CompileDiagnostic, ModelManifest } from '../domain';
import { hasBundledExampleModel, type LocalModelRequirement } from '../hooks/useKessetsuWorkspace';
import comparatorModelUrl from '../../../examples/models/comparator.lib?url&no-inline';
import memristorModelUrl from '../../../examples/models/memristor.lib?url&no-inline';
import memristorNoticeUrl from '../../../examples/models/MEMRISTOR_NOTICE.md?url&no-inline';

interface Props {
  open: boolean;
  spice: string;
  models: ModelManifest | null;
  diagnostics: CompileDiagnostic[];
  onClose: () => void;
  resources: LocalModelRequirement[];
  boundResources: string[];
  onBindFile: (resource: string, file: File) => Promise<void>;
  onBindBundled: (resource: string) => void;
  onClearFiles: () => void;
}

export function CircuitDetailsDialog({ open, spice, models, diagnostics, onClose, resources, boundResources, onBindFile, onBindBundled, onClearFiles }: Props) {
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
        <div className="model-list">
          {resources.map((item) => {
            const isBound = boundResources.includes(item.resource);
            const hasMismatchError = isBound && diagnostics.some((d) => ['KES-C016', 'KES-C017'].includes(d.code) && (d.message.includes(item.resource) || d.message.includes(item.model) || d.component === item.model));
            const canUseBundled = hasBundledExampleModel(item.resource, item.sha256) && (!isBound || hasMismatchError);
            return (
              <article key={`${item.model}:${item.resource}`} className="model-resource-card">
                <div className="model-resource-header">
                  <div>
                    <strong>{item.model}</strong>
                    <code className="model-resource-path">{item.resource}</code>
                  </div>
                  {!isBound ? (
                    <span className="model-status-badge is-missing">File required</span>
                  ) : hasMismatchError ? (
                    <span className="model-status-badge is-error">File selected · Verification failed</span>
                  ) : (
                    <span className="model-status-badge is-valid">File selected · Verified</span>
                  )}
                </div>
                <span className="model-simulator-note">
                  {item.simulator === 'ngspice_ps'
                    ? 'Native-only: use the installed CLI with Ngspice; selecting a file will not enable browser simulation'
                    : 'Checked locally in this browser tab before simulation'}
                </span>
                <div className="model-resource-actions">
                  <label className="model-file-button">
                    <Upload size={13} />
                    <span>{isBound ? 'Replace file…' : 'Choose file…'}</span>
                    <input
                      type="file"
                      className="sr-only"
                      aria-label={`Model file for ${item.model}`}
                      onChange={(event) => {
                        const file = event.currentTarget.files?.[0];
                        event.currentTarget.value = '';
                        if (!file) return;
                        setResourceError('');
                        void onBindFile(item.resource, file).catch((error: unknown) => setResourceError(String(error)));
                      }}
                    />
                  </label>
                  {canUseBundled && (
                    <button type="button" className="secondary-button" onClick={() => { setResourceError(''); onBindBundled(item.resource); }}>
                      Use bundled example file
                    </button>
                  )}
                </div>
                <details className="model-provenance-details">
                  <summary>Expected SHA-256</summary>
                  <small>SHA-256: {item.sha256}</small>
                </details>
              </article>
            );
          })}
        </div>
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
