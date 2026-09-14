import { Check, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { MAX_DOCUMENT_NAME_LENGTH, normalizeDocumentName } from '../document';

interface Props {
  open: boolean;
  currentName: string;
  onClose: () => void;
  onRename: (name: string) => void;
}

export function RenameDialog({ open, currentName, onClose, onRename }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(currentName);
  const [error, setError] = useState('');

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      setName(currentName);
      setError('');
      dialog.showModal();
    }
    if (!open && dialog.open) dialog.close();
  }, [currentName, open]);

  const submit = () => {
    try {
      onRename(normalizeDocumentName(name));
      dialogRef.current?.close();
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  return (
    <dialog ref={dialogRef} className="app-dialog name-dialog" aria-labelledby="rename-title" onClose={onClose}>
      <header>
        <div><h2 id="rename-title">Rename circuit</h2><p>This name is used for source and export filenames.</p></div>
        <button aria-label="Close rename" onClick={() => dialogRef.current?.close()}><X size={18} /></button>
      </header>
      <label className="field-label" htmlFor="document-name">Circuit name</label>
      <input
        id="document-name"
        autoFocus
        maxLength={MAX_DOCUMENT_NAME_LENGTH}
        value={name}
        onChange={(event) => { setName(event.target.value); setError(''); }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            submit();
          }
        }}
      />
      <div className="dialog-footer">
        <span className="dialog-status" role="status">{error}</span>
        <button className="dialog-secondary" onClick={() => dialogRef.current?.close()}>Cancel</button>
        <button className="dialog-primary" disabled={!name.trim()} onClick={submit}><Check size={15} />Rename</button>
      </div>
    </dialog>
  );
}
