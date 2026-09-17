import { CheckCircle2, ChevronRight, CircleAlert, LoaderCircle, Share2 } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import productVersionSource from '../../../VERSION?raw';
import {
  downloadTextFile,
  nativeFileSavingSupported,
  openWithNativeFilePicker,
  saveWithNativeFilePicker,
  sanitizeFileStem,
  type KessetsuFileHandle,
  decodeWorkspaceDraft,
  PREVIOUS_CIRCUIT_STORAGE_KEY,
  writeWorkspaceDraft,
} from '../document';
import { useKessetsuWorkspace, examples, type ExampleId } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { BrandWordmark } from './BrandWordmark';
import { CircuitDetailsDialog } from './CircuitDetailsDialog';
import { EditorPanel } from './EditorPanel';
import { RenameDialog } from './RenameDialog';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';
import { ShareDialog } from './ShareDialog';
import { WorkspaceLayout } from './WorkspaceLayout';

type MenuId = 'file' | 'view' | 'help';
const productVersion = productVersionSource.trim();

export function WorkspaceApp() {
  const {
    state, setCode, loadExample, newDocument, openDocument, markSaved, saveBrowserDocument, renameDocument,
    run, cancel, createExport, share,
    modelRequirements, boundModelResources, bindModelFile, clearModelFiles,
  } = useKessetsuWorkspace();
  const [openMenu, setOpenMenu] = useState<MenuId | null>(null);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [resetRequest, setResetRequest] = useState(0);
  const [circuitDetailsOpen, setCircuitDetailsOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);
  const [documentError, setDocumentError] = useState('');
  const [documentNotice, setDocumentNotice] = useState('');
  const menusRef = useRef<HTMLElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const fileHandleRef = useRef<KessetsuFileHandle | null>(null);
  const noticeTimeoutRef = useRef<number | null>(null);
  const nativeFileSaving = nativeFileSavingSupported();
  const [previousCircuitAvailable, setPreviousCircuitAvailable] = useState(() => {
    try { return Boolean(decodeWorkspaceDraft(localStorage.getItem(PREVIOUS_CIRCUIT_STORAGE_KEY))); }
    catch { return false; }
  });
  const selectedExample = Object.entries(examples).find(([, example]) => example.source === state.code)?.[0] as ExampleId | undefined;
  const documentName = state.circuitName ?? (selectedExample ? examples[selectedExample].label : 'Untitled circuit');
  const errorCount = state.diagnostics.filter((diagnostic) => diagnostic.severity === 'error').length;
  const compileStatus = state.compileState === 'loading'
    ? 'Loading Core'
    : state.compileState === 'checking'
      ? 'Auto-checking…'
      : state.compileState === 'valid'
        ? 'Checked'
        : errorCount > 0
          ? `${errorCount} ${errorCount === 1 ? 'error' : 'errors'}`
          : 'Needs attention';
  const CompileStatusIcon = state.compileState === 'valid'
    ? CheckCircle2
    : state.compileState === 'invalid'
      ? CircleAlert
      : LoaderCircle;

  useEffect(() => {
    document.documentElement.dataset.theme = 'dark';
    document.title = `${state.isDirty ? '● ' : ''}${documentName} — Kessetsu`;
  }, [documentName, state.isDirty]);

  useEffect(() => () => {
    if (noticeTimeoutRef.current !== null) globalThis.clearTimeout(noticeTimeoutRef.current);
  }, []);

  useEffect(() => {
    if (state.isDirty) setDocumentNotice('');
  }, [state.isDirty]);

  const showDocumentNotice = useCallback((message: string) => {
    if (noticeTimeoutRef.current !== null) globalThis.clearTimeout(noticeTimeoutRef.current);
    setDocumentNotice(message);
    noticeTimeoutRef.current = globalThis.setTimeout(() => setDocumentNotice(''), 2600);
  }, []);

  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!menusRef.current?.contains(event.target as Node)) {
        setOpenMenu(null);
        setExamplesOpen(false);
      }
    };
    const closeWithEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpenMenu(null);
        setExamplesOpen(false);
      }
    };
    window.addEventListener('pointerdown', closeOutside);
    window.addEventListener('keydown', closeWithEscape);
    return () => {
      window.removeEventListener('pointerdown', closeOutside);
      window.removeEventListener('keydown', closeWithEscape);
    };
  }, []);

  useEffect(() => {
    const runShortcut = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 'Enter'
        && state.compileSucceeded && state.simulationState !== 'running') {
        event.preventDefault();
        void run();
      }
    };
    window.addEventListener('keydown', runShortcut);
    return () => window.removeEventListener('keydown', runShortcut);
  }, [run, state.compileSucceeded, state.simulationState]);

  const toggleMenu = (menu: MenuId) => {
    setExamplesOpen(false);
    setOpenMenu((current) => current === menu ? null : menu);
  };
  const selectExample = (id: ExampleId) => {
    if (state.isDirty && !globalThis.confirm('Replace the current unsaved circuit with this example?')) return;
    fileHandleRef.current = null;
    loadExample(id);
    setExamplesOpen(false);
    setOpenMenu(null);
  };
  const startNewDocument = () => {
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and create a new circuit?')) return;
    fileHandleRef.current = null;
    newDocument();
    setOpenMenu(null);
  };
  const chooseDocument = () => {
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and open another circuit?')) return;
    setOpenMenu(null);
    setDocumentError('');
    void openWithNativeFilePicker()
      .then(async (result) => {
        if (result.status === 'unsupported') {
          fileInputRef.current?.click();
          return;
        }
        if (result.status === 'cancelled') return;
        await openDocument(result.file);
        fileHandleRef.current = result.handle;
      })
      .catch((cause: unknown) => {
        setDocumentError(`Could not open circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
      });
  };

  const restorePreviousCircuit = async () => {
    if (state.isDirty && !globalThis.confirm('Replace the current unsaved circuit with the previous browser circuit? Download it first if you need both.')) return;
    try {
      const draft = decodeWorkspaceDraft(localStorage.getItem(PREVIOUS_CIRCUIT_STORAGE_KEY));
      if (!draft) return;
      fileHandleRef.current = null;
      await openDocument(new File([draft.source], `${draft.name ?? 'Previous circuit'}.kess`, { type: 'text/plain' }));
      if (draft.dirty) setCode(draft.source);
      localStorage.removeItem(PREVIOUS_CIRCUIT_STORAGE_KEY);
      setPreviousCircuitAvailable(false);
      setOpenMenu(null);
    } catch (error) { setDocumentError(error instanceof Error ? error.message : String(error)); }
  };
  const saveDocument = useCallback(async (forceSaveAs = false) => {
    setOpenMenu(null);
    setDocumentError('');
    const fileName = `${sanitizeFileStem(documentName)}.kess`;
    try {
      if (!nativeFileSaving) {
        if (forceSaveAs) {
          downloadTextFile(state.code, fileName);
          markSaved();
          showDocumentNotice(`Downloaded ${fileName}`);
        } else {
          saveBrowserDocument();
          showDocumentNotice('Saved in this browser');
        }
        return;
      }
      const result = await saveWithNativeFilePicker(
        state.code,
        fileName,
        fileHandleRef.current,
        forceSaveAs,
      );
      if (result.status === 'cancelled') return;
      if (result.status === 'unsupported') {
        saveBrowserDocument();
        fileHandleRef.current = null;
        showDocumentNotice('Saved in this browser');
      } else {
        fileHandleRef.current = result.handle;
        markSaved();
        showDocumentNotice(`Saved to ${result.handle.name}`);
      }
    } catch (cause: unknown) {
      setDocumentError(`Could not save circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }, [documentName, markSaved, nativeFileSaving, saveBrowserDocument, showDocumentNotice, state.code]);

  const renameCurrentDocument = useCallback((name: string) => {
    fileHandleRef.current = null;
    renameDocument(name);
  }, [renameDocument]);

  const shareCircuit = useCallback(async (name: string) => {
    if (name.trim() !== documentName) fileHandleRef.current = null;
    return share(name);
  }, [documentName, share]);

  useEffect(() => {
    const saveShortcut = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
        event.preventDefault();
        void saveDocument(event.shiftKey);
      }
    };
    window.addEventListener('keydown', saveShortcut);
    return () => window.removeEventListener('keydown', saveShortcut);
  }, [saveDocument]);

  return (
    <main className="app-shell">
      <header className="app-menubar">
        <a className="wordmark" href="./" aria-label="Kessetsu home"><BrandWordmark /></a>
        <nav className="application-menus" aria-label="Application menu" ref={menusRef}>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'file'} onClick={() => toggleMenu('file')}>File</button>
            {openMenu === 'file' && <div className="menu-popover file-menu" role="menu" aria-label="File menu">
              <button role="menuitem" onClick={startNewDocument}><span>New circuit</span></button>
              <button role="menuitem" onClick={chooseDocument}><span>Open .kess…</span></button>
              <button role="menuitem" aria-label={nativeFileSaving ? 'Save' : 'Save in browser'} onClick={() => void saveDocument()}>
                <span>{nativeFileSaving ? 'Save' : 'Save in browser'}</span><kbd>Ctrl+S</kbd>
              </button>
              <button role="menuitem" aria-label={nativeFileSaving ? 'Save As' : 'Download .kess'} onClick={() => void saveDocument(true)}>
                <span>{nativeFileSaving ? 'Save As...' : 'Download .kess…'}</span><kbd>Ctrl+Shift+S</kbd>
              </button>
              <button role="menuitem" onClick={() => { setRenameOpen(true); setOpenMenu(null); }}><span>Rename…</span></button>
              {previousCircuitAvailable && <button role="menuitem" onClick={() => void restorePreviousCircuit()}>Restore previous circuit</button>}
              <a role="menuitem" href={`${import.meta.env.BASE_URL}tools/`} onClick={(event) => {
                try { writeWorkspaceDraft(localStorage, state.circuitName, state.code, state.isDirty); }
                catch { event.preventDefault(); setDocumentError('Browser storage is unavailable. Download this circuit before opening tools.'); }
              }}>Circuit tools</a>
              {!nativeFileSaving && <p className="menu-note">Saved projects stay in this browser. Download a .kess copy to use elsewhere.</p>}
              <div className="menu-separator" role="separator" />
              <div className="menu-submenu">
                <button role="menuitem" aria-haspopup="menu" aria-expanded={examplesOpen} onClick={() => setExamplesOpen((current) => !current)}>
                  <span>Examples</span><ChevronRight size={14} />
                </button>
                {examplesOpen && <div className="menu-popover examples-menu" role="menu" aria-label="Examples menu">
                  <span className="menu-group-label">Fundamentals</span>
                  <button role="menuitem" aria-label={examples.rc.label} onClick={() => selectExample('rc')}>
                    <strong>{examples.rc.label}</strong><small>{examples.rc.description}</small>
                  </button>
                  <span className="menu-group-label">Amplifiers</span>
                  <button role="menuitem" aria-label={examples.gain.label} onClick={() => selectExample('gain')}>
                    <strong>{examples.gain.label}</strong><small>{examples.gain.description}</small>
                  </button>
                  <button role="menuitem" aria-label={examples.power.label} onClick={() => selectExample('power')}>
                    <strong>{examples.power.label}</strong><small>{examples.power.description}</small>
                  </button>
                  <span className="menu-group-label">Reusable blocks</span>
                  {(['filters', 'amplifiers'] as const).map((id) => (
                    <button key={id} role="menuitem" aria-label={examples[id].label} onClick={() => selectExample(id)}>
                      <strong>{examples[id].label}</strong><small>{examples[id].description}</small>
                    </button>
                  ))}
                  <span className="menu-group-label">Local model files</span>
                  {(['comparator', 'memristor'] as const).map((id) => (
                    <button key={id} role="menuitem" aria-label={examples[id].label} onClick={() => selectExample(id)}>
                      <strong>{examples[id].label}</strong><small>{examples[id].description}</small>
                    </button>
                  ))}
                </div>}
              </div>
            </div>}
          </div>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'view'} onClick={() => toggleMenu('view')}>View</button>
            {openMenu === 'view' && <div className="menu-popover" role="menu" aria-label="View menu">
              <button role="menuitem" onClick={() => { setCircuitDetailsOpen(true); setOpenMenu(null); }}>
                <span>Circuit details…</span>
              </button>
              <button role="menuitem" onClick={() => { setResetRequest((current) => current + 1); setOpenMenu(null); }}>
                <span>Reset panel layout</span>
              </button>
            </div>}
          </div>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'help'} onClick={() => toggleMenu('help')}>Help</button>
            {openMenu === 'help' && <div className="menu-popover" role="menu" aria-label="Help menu">
              <a role="menuitem" href="https://github.com/Stapimaz/Kessetsu/blob/main/docs/README.md" target="_blank" rel="noreferrer">Documentation</a>
              <a role="menuitem" href="https://github.com/Stapimaz/Kessetsu/blob/main/CHANGELOG.md" target="_blank" rel="noreferrer">What’s new in {productVersion}</a>
              <a role="menuitem" href="https://github.com/Stapimaz/Kessetsu" target="_blank" rel="noreferrer" aria-label="Kessetsu corresponding source code">Corresponding source</a>
              <a role="menuitem" href={`${import.meta.env.BASE_URL}LICENSE.txt`} target="_blank" rel="noreferrer">License</a>
              <span className="menu-version">Kessetsu {productVersion}</span>
            </div>}
          </div>
        </nav>
        <div
          className={`document-title${state.isDirty ? ' document-dirty' : ''}`}
          title={`${documentName}${state.isDirty ? ' — unsaved changes' : ''}`}
          aria-label={`${documentName}${state.isDirty ? ', unsaved changes' : ''}`}
        >{documentName}</div>
        <div className="global-actions">
          <span className={`compile-status compile-${state.compileState}`} role="status" aria-label={`Automatic circuit check: ${compileStatus}`} data-testid="compile-status" title={compileStatus}>
            <CompileStatusIcon size={14} /><span>{compileStatus}</span>
          </span>
          <ArtifactBar enabled={state.compileSucceeded}
            capabilities={state.exportCapabilities} message={state.exportMessage}
            filenameStem={sanitizeFileStem(documentName)} onExport={createExport} />
          <button className="header-action-button share-button" onClick={() => setShareOpen(true)} disabled={!state.compileSucceeded} aria-label="Share circuit">
            <Share2 size={15} /> <span>Share</span>
          </button>
        </div>
      </header>
      <input
        ref={fileInputRef}
        className="sr-only"
        type="file"
        accept=".kess,text/plain"
        aria-label="Open Kessetsu source file"
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (!file) return;
          setDocumentError('');
          fileHandleRef.current = null;
          void openDocument(file).catch((cause: unknown) => {
            setDocumentError(`Could not open circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
          });
        }}
      />
      {state.draftRestored && <div className="draft-notice" role="status">Unsaved browser draft restored. Use File to save it here or download a portable .kess copy.</div>}
      {documentError && <div className="global-error" role="alert">{documentError}</div>}
      {documentNotice && <div className="workspace-toast" role="status">{documentNotice}</div>}
      {state.wasmError && <div className="global-error" role="alert">Core failed to initialize: {state.wasmError}</div>}
      <CircuitDetailsDialog
        open={circuitDetailsOpen}
        spice={state.spiceNetlist}
        models={state.modelManifest}
        resources={modelRequirements}
        boundResources={boundModelResources}
        onBindFile={bindModelFile}
        onClearFiles={clearModelFiles}
        onClose={() => setCircuitDetailsOpen(false)}
      />
      <ShareDialog
        open={shareOpen}
        currentName={documentName}
        onClose={() => setShareOpen(false)}
        onCreateLink={shareCircuit}
        hasLocalModels={modelRequirements.length > 0}
      />
      <RenameDialog
        open={renameOpen}
        currentName={documentName}
        onClose={() => setRenameOpen(false)}
        onRename={renameCurrentDocument}
      />
      <WorkspaceLayout
        resetRequest={resetRequest}
        source={(panelControls) => <EditorPanel
          code={state.code}
          diagnostics={state.diagnostics}
          compileSucceeded={state.compileSucceeded}
          onCodeChange={setCode}
          panelControls={panelControls}
        />}
        schematic={(panelControls) => <SchematicPanel schematic={state.schematic} svg={state.schematicSvg} panelControls={panelControls} />}
        results={(panelControls) => <ResultsPanel
          state={state.simulationState}
          message={state.simulationMessage}
          evaluation={state.evaluation}
          compileSucceeded={state.compileSucceeded}
          onRun={() => void run()}
          onCancel={cancel}
          panelControls={panelControls}
        />}
      />
    </main>
  );
}
