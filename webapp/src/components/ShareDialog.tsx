import { Check, Copy, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { MAX_SHARE_NAME_LENGTH } from '../share';

interface Props {
  open: boolean;
  source: string;
  currentName: string;
  onClose: () => void;
  onCreateLink: (name: string, signal: AbortSignal) => Promise<string>;
  hasLocalModels?: boolean;
}

export function ShareDialog({ open, source, currentName, onClose, onCreateLink, hasLocalModels }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(currentName);
  const [url, setUrl] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const operationRef = useRef<AbortController | null>(null);
  const latestSourceRef = useRef(source);
  latestSourceRef.current = source;
  const previousSourceRef = useRef(source);

  const abandonPending = useCallback(() => {
    operationRef.current?.abort();
    operationRef.current = null;
    setBusy(false);
  }, []);
  const close = () => {
    abandonPending();
    dialogRef.current?.close();
  };

  useEffect(() => () => operationRef.current?.abort(), []);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      abandonPending();
      setName(currentName);
      setUrl('');
      setStatus('');
      dialog.showModal();
    }
    if (!open) {
      abandonPending();
      if (dialog.open) dialog.close();
    }
  }, [abandonPending, currentName, open]);

  useEffect(() => {
    if (previousSourceRef.current === source) return;
    previousSourceRef.current = source;
    abandonPending();
    setUrl('');
    if (open) setStatus('The circuit changed. Create a new link for this version.');
  }, [abandonPending, open, source]);

  const createAndCopy = async () => {
    if (operationRef.current || !dialogRef.current?.open) return;
    const circuitName = name.trim();
    if (!circuitName) {
      setStatus('Enter a circuit name.');
      return;
    }
    setBusy(true);
    const operation = new AbortController();
    operationRef.current = operation;
    const sharedSource = source;
    const isCurrent = () => operationRef.current === operation && !operation.signal.aborted
      && dialogRef.current?.open && latestSourceRef.current === sharedSource;
    setStatus(url ? 'Copying link…' : 'Creating link…');
    try {
      const nextUrl = url || await onCreateLink(circuitName, operation.signal);
      if (!isCurrent()) return;
      setUrl(nextUrl);
      try {
        await navigator.clipboard.writeText(nextUrl);
        if (!isCurrent()) return;
        setStatus('Link copied to clipboard.');
      } catch {
        if (!isCurrent()) return;
        setStatus('Link created. Select and copy it below.');
      }
    } catch (cause: unknown) {
      if (!isCurrent()) return;
      setStatus(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (operationRef.current === operation) {
        operationRef.current = null;
        setBusy(false);
      }
    }
  };

  return (
    <dialog ref={dialogRef} className="app-dialog share-dialog" aria-labelledby="share-title"
      onCancel={abandonPending} onClose={() => { abandonPending(); onClose(); }}>
      <header>
        <div>
          <h2 id="share-title">Share circuit</h2>
          <p>Send a named copy of the circuit as it is now.</p>
        </div>
        <button aria-label="Close share" onClick={close}><X size={18} /></button>
      </header>
      {hasLocalModels && <p>Local model files are not included in this link. Recipients must select matching files in View → Circuit details. Respect each model's redistribution terms.</p>}
      <label className="field-label" htmlFor="share-circuit-name">Circuit name</label>
      <input
        id="share-circuit-name"
        autoFocus
        maxLength={MAX_SHARE_NAME_LENGTH}
        disabled={busy}
        value={name}
        onChange={(event) => { setName(event.target.value); setUrl(''); setStatus(''); }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            void createAndCopy();
          }
        }}
      />
      <p className="share-privacy">The link includes this source and exact package versions, not simulation results or local model/data files. Nothing is uploaded. Anyone with the link can read the circuit; complex circuits produce longer URLs.</p>
      <p className="share-privacy">Recipients open their own editable copy. Later edits do not update a link you already sent.</p>
      {url && <label className="share-url">
        <span>Share link</span>
        <input readOnly value={url} onFocus={(event) => event.currentTarget.select()} />
      </label>}
      <div className="dialog-footer">
        <span className="dialog-status" role="status">{status && (status.startsWith('Link copied') ? <><Check size={14} />{status}</> : status)}</span>
        <button className="dialog-secondary" onClick={close}>{url ? 'Close' : 'Cancel'}</button>
        <button className="dialog-primary" disabled={busy || !name.trim()} onClick={() => void createAndCopy()}>
          <Copy size={15} />{busy ? (url ? 'Copying…' : 'Creating…') : 'Copy link'}
        </button>
      </div>
    </dialog>
  );
}
