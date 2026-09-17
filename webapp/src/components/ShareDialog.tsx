import { Check, Copy, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { MAX_SHARE_NAME_LENGTH } from '../share';

interface Props {
  open: boolean;
  currentName: string;
  onClose: () => void;
  onCreateLink: (name: string) => Promise<string>;
  hasLocalModels?: boolean;
}

export function ShareDialog({ open, currentName, onClose, onCreateLink, hasLocalModels }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(currentName);
  const [url, setUrl] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      setName(currentName);
      setUrl('');
      setStatus('');
      dialog.showModal();
    }
    if (!open && dialog.open) dialog.close();
  }, [currentName, open]);

  const createAndCopy = async () => {
    const circuitName = name.trim();
    if (!circuitName) {
      setStatus('Enter a circuit name.');
      return;
    }
    setBusy(true);
    setStatus('Creating link…');
    try {
      const nextUrl = await onCreateLink(circuitName);
      setUrl(nextUrl);
      try {
        await navigator.clipboard.writeText(nextUrl);
        setStatus('Link copied to clipboard.');
      } catch {
        setStatus('Link created. Select and copy it below.');
      }
    } catch (cause: unknown) {
      setStatus(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <dialog ref={dialogRef} className="app-dialog share-dialog" aria-labelledby="share-title" onClose={onClose}>
      <header>
        <div>
          <h2 id="share-title">Share circuit</h2>
          <p>Name this circuit before creating its link.</p>
        </div>
        <button aria-label="Close share" onClick={() => dialogRef.current?.close()}><X size={18} /></button>
      </header>
      {hasLocalModels && <p>Local model files are not included in this link. Recipients must select matching files in View → Circuit details. Respect each model's redistribution terms.</p>}
      <label className="field-label" htmlFor="share-circuit-name">Circuit name</label>
      <input
        id="share-circuit-name"
        autoFocus
        maxLength={MAX_SHARE_NAME_LENGTH}
        value={name}
        onChange={(event) => { setName(event.target.value); setUrl(''); setStatus(''); }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            void createAndCopy();
          }
        }}
      />
      <p className="share-privacy">The source and exact package versions are compressed into the link. Nothing is uploaded, so complex circuits produce longer URLs.</p>
      {url && <label className="share-url">
        <span>Share link</span>
        <input readOnly value={url} onFocus={(event) => event.currentTarget.select()} />
      </label>}
      <div className="dialog-footer">
        <span className="dialog-status" role="status">{status && (status.startsWith('Link copied') ? <><Check size={14} />{status}</> : status)}</span>
        <button className="dialog-secondary" onClick={() => dialogRef.current?.close()}>Cancel</button>
        <button className="dialog-primary" disabled={busy || !name.trim()} onClick={() => void createAndCopy()}>
          <Copy size={15} />{busy ? 'Creating…' : 'Copy link'}
        </button>
      </div>
    </dialog>
  );
}
