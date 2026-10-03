import { CheckCircle2, ChevronRight, CircleAlert, LoaderCircle, Share2 } from 'lucide-react';
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';
import type { SourceFeedback } from '../domain';
import productVersionSource from '../../../VERSION?raw';
import {
  downloadTextFile,
  nativeFileSavingSupported,
  openWithNativeFilePicker,
  saveWithNativeFilePicker,
  sanitizeFileStem,
  type KessetsuFileHandle,
  type DocumentRevision,
  decodeWorkspaceDraft,
  PREVIOUS_CIRCUIT_STORAGE_KEY,
  writeWorkspaceDraft,
} from '../document';
import { useKessetsuWorkspace, examples, type ExampleId } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { AgentProposalDialog } from './AgentProposalDialog';
import { BrandWordmark } from './BrandWordmark';
import { CircuitDetailsDialog } from './CircuitDetailsDialog';
import { EditorPanel } from './EditorPanel';
import { RenameDialog } from './RenameDialog';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';
import { ShareDialog } from './ShareDialog';
import { WorkspaceLayout } from './WorkspaceLayout';
import { StudyDialog } from './StudyDialog';

type MenuId = 'file' | 'view' | 'analyze' | 'help';
const productVersion = productVersionSource.trim();
const ResearchDataDialog = lazy(async () => {
  const module = await import('./ResearchDataDialog');
  return { default: module.ResearchDataDialog };
});

export function WorkspaceApp() {
  const {
    state, setCode, loadExample, newDocument, openDocument, importSpiceDocument, markSaved, saveBrowserDocument, renameDocument,
    run, cancel, createExport, share,
    captureDocumentRevision, isCurrentDocument, isCurrentDocumentRevision,
    modelRequirements, boundModelResources, bindModelFile, clearModelFiles,
    modelResources,
  } = useKessetsuWorkspace();
  const [openMenu, setOpenMenu] = useState<MenuId | null>(null);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [resetRequest, setResetRequest] = useState(0);
  const [sourceRevealRequest, setSourceRevealRequest] = useState(0);
  const [circuitDetailsOpen, setCircuitDetailsOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);
  const [studyOpen, setStudyOpen] = useState(false);
  const [researchDataOpen, setResearchDataOpen] = useState(false);
  const [researchDataMounted, setResearchDataMounted] = useState(false);
  const [agentProposalOpen, setAgentProposalOpen] = useState(false);
  const [acceptedProposal, setAcceptedProposal] = useState<{ before: string; after: string } | null>(null);
  const [documentError, setDocumentError] = useState('');
  const [documentNotice, setDocumentNotice] = useState('');
  const [documentSaving, setDocumentSaving] = useState(false);
  const menusRef = useRef<HTMLElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const spiceInputRef = useRef<HTMLInputElement>(null);
  const fileHandleRef = useRef<KessetsuFileHandle | null>(null);
  const fileAssociationRef = useRef(0);
  const fileInputRevisionRef = useRef<DocumentRevision | null>(null);
  const savePendingRef = useRef(false);
  const detachFileHandle = useCallback(() => {
    fileHandleRef.current = null;
    fileAssociationRef.current += 1;
  }, []);
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
        ? 'Source valid'
        : errorCount > 0
          ? `${errorCount} ${errorCount === 1 ? 'error' : 'errors'}`
          : 'Needs attention';
  const CompileStatusIcon = state.compileState === 'valid'
    ? CheckCircle2
    : state.compileState === 'invalid'
      ? CircleAlert
      : LoaderCircle;
  const modelFileError = modelRequirements.length > 0
    && state.diagnostics.some((diagnostic) => ['KES-C015', 'KES-C016', 'KES-C017'].includes(diagnostic.code));
  const sourceFeedback: SourceFeedback | null = state.compileState === 'valid' ? null
    : state.wasmError ? { title: 'Editor unavailable', message: 'The editor could not load this circuit. See the error above before reloading.' }
    : state.compileState === 'loading' ? { title: 'Loading editor', message: 'Preparing local circuit checks. Simulation and export will be available when the source is valid.' }
    : state.compileState === 'checking' ? { title: 'Checking source', message: 'Checking your changes before simulation and export.' }
    : modelFileError ? { title: 'Model file needed', message: 'Choose the exact local model file declared in Source. Missing or mismatched files cannot be simulated.', actionLabel: 'Choose model files' }
    : { title: 'Fix source errors', message: 'Simulation and export are paused. Open the source diagnostics; select a reported line to fix it.', actionLabel: 'Show source errors' };
  const resolveSource = () => {
    if (modelFileError) setCircuitDetailsOpen(true);
    else setSourceRevealRequest((current) => current + 1);
  };

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
    detachFileHandle();
    loadExample(id);
    setExamplesOpen(false);
    setOpenMenu(null);
  };
  const startNewDocument = () => {
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and create a new circuit?')) return;
    detachFileHandle();
    newDocument();
    setOpenMenu(null);
  };
  const chooseDocument = () => {
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and open another circuit?')) return;
    setOpenMenu(null);
    setDocumentError('');
    const snapshot = captureDocumentRevision();
    void openWithNativeFilePicker()
      .then(async (result) => {
        if (result.status === 'unsupported') {
          fileInputRevisionRef.current = snapshot;
          fileInputRef.current?.click();
          return;
        }
        if (result.status === 'cancelled') return;
        if (!isCurrentDocumentRevision(snapshot)) throw new Error('The circuit changed while choosing a file. Open it again to replace the current work.');
        await openDocument(result.file);
        detachFileHandle();
        fileHandleRef.current = result.handle;
      })
      .catch((cause: unknown) => {
        setDocumentError(`Could not open circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
      });
  };

  const importSpiceFile = useCallback(async (file: File) => {
    setDocumentError('');
    try {
      const report = await importSpiceDocument(file);
      detachFileHandle();
      showDocumentNotice(`Imported ${report.summary.components} components from ${file.name}`);
    } catch (cause: unknown) {
      setDocumentError(`SPICE import stopped: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }, [detachFileHandle, importSpiceDocument, showDocumentNotice]);

  const chooseSpiceDocument = () => {
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and import a SPICE netlist?')) return;
    setOpenMenu(null);
    spiceInputRef.current?.click();
  };

  const dropDocument = (event: DragEvent<HTMLElement>) => {
    const file = event.dataTransfer.files[0];
    if (!file) return;
    event.preventDefault();
    if (state.isDirty && !globalThis.confirm('Discard the current unsaved changes and open the dropped circuit?')) return;
    if (/\.kess$/i.test(file.name)) {
      setDocumentError('');
      void openDocument(file).then(detachFileHandle).catch((cause: unknown) => {
        setDocumentError(`Could not open circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
      });
    } else if (/\.(?:cir|sp|spice|net)$/i.test(file.name)) {
      void importSpiceFile(file);
    } else {
      setDocumentError('Drop a .kess circuit or a .cir, .sp, .spice or .net SPICE netlist.');
    }
  };

  const restorePreviousCircuit = async () => {
    if (state.isDirty && !globalThis.confirm('Replace the current unsaved circuit with the previous browser circuit? Download it first if you need both.')) return;
    try {
      const draft = decodeWorkspaceDraft(localStorage.getItem(PREVIOUS_CIRCUIT_STORAGE_KEY));
      if (!draft) return;
      await openDocument(new File([draft.source], `${draft.name ?? 'Previous circuit'}.kess`, { type: 'text/plain' }));
      detachFileHandle();
      if (draft.dirty) setCode(draft.source);
      localStorage.removeItem(PREVIOUS_CIRCUIT_STORAGE_KEY);
      setPreviousCircuitAvailable(false);
      setOpenMenu(null);
    } catch (error) { setDocumentError(error instanceof Error ? error.message : String(error)); }
  };
  const saveDocument = useCallback(async (forceSaveAs = false) => {
    if (!state.wasmLoaded) return;
    if (savePendingRef.current) return;
    savePendingRef.current = true;
    setDocumentSaving(true);
    setOpenMenu(null);
    setDocumentError('');
    const snapshot = captureDocumentRevision();
    const association = fileAssociationRef.current;
    const fileName = `${sanitizeFileStem(documentName)}.kess`;
    try {
      if (!nativeFileSaving) {
        if (forceSaveAs) {
          downloadTextFile(state.code, fileName);
          markSaved(snapshot);
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
        if (!isCurrentDocumentRevision(snapshot)) throw new Error('The circuit changed before it could be saved. Save the current version again.');
        saveBrowserDocument();
        detachFileHandle();
        showDocumentNotice('Saved in this browser');
      } else {
        if (isCurrentDocument(snapshot) && association === fileAssociationRef.current) {
          fileHandleRef.current = result.handle;
        }
        markSaved(snapshot);
        showDocumentNotice(isCurrentDocumentRevision(snapshot)
          ? `Saved to ${result.handle.name}`
          : isCurrentDocument(snapshot)
            ? `Saved an earlier version to ${result.handle.name}. Your latest changes still need saving.`
            : `Previous circuit saved to ${result.handle.name}. The current circuit was not saved.`);
      }
    } catch (cause: unknown) {
      setDocumentError(`Could not save circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      savePendingRef.current = false;
      setDocumentSaving(false);
    }
  }, [captureDocumentRevision, detachFileHandle, documentName, isCurrentDocument, isCurrentDocumentRevision, markSaved, nativeFileSaving, saveBrowserDocument, showDocumentNotice, state.code, state.wasmLoaded]);

  const renameCurrentDocument = useCallback((name: string) => {
    renameDocument(name);
    detachFileHandle();
  }, [detachFileHandle, renameDocument]);

  const shareCircuit = useCallback(async (name: string, signal: AbortSignal) => {
    const link = await share(name, signal);
    if (name.trim() !== documentName) detachFileHandle();
    return link;
  }, [detachFileHandle, documentName, share]);

  const acceptAgentProposal = useCallback((nextSource: string) => {
    detachFileHandle();
    setAcceptedProposal({ before: state.code, after: nextSource });
    setCode(nextSource);
    showDocumentNotice('Agent proposal accepted. Undo is available from Analyze.');
  }, [detachFileHandle, setCode, showDocumentNotice, state.code]);

  const undoAgentProposal = useCallback(() => {
    if (!acceptedProposal || state.code !== acceptedProposal.after) return;
    setCode(acceptedProposal.before);
    setAcceptedProposal(null);
    setOpenMenu(null);
    showDocumentNotice('Accepted proposal reverted.');
  }, [acceptedProposal, setCode, showDocumentNotice, state.code]);

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
    <main className="app-shell" onDragOver={(event) => {
      if (event.dataTransfer.types.includes('Files')) event.preventDefault();
    }} onDrop={dropDocument}>
      <header className="app-menubar">
        <a className="wordmark" href="./" aria-label="Kessetsu home"><BrandWordmark /></a>
        <nav className="application-menus" aria-label="Application menu" ref={menusRef}>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'file'} onClick={() => toggleMenu('file')}>File</button>
            {openMenu === 'file' && <div className="menu-popover file-menu" role="menu" aria-label="File menu">
              <button role="menuitem" onClick={startNewDocument}><span>New circuit</span></button>
              <button role="menuitem" onClick={chooseDocument}><span>Open .kess…</span></button>
              <button role="menuitem" onClick={chooseSpiceDocument}><span>Import SPICE netlist...</span></button>
              <button role="menuitem" disabled={documentSaving || !state.wasmLoaded} aria-label={nativeFileSaving ? 'Save' : 'Save in browser'} onClick={() => void saveDocument()}>
                <span>{nativeFileSaving ? 'Save' : 'Save in browser'}</span><kbd>Ctrl+S</kbd>
              </button>
              <button role="menuitem" disabled={documentSaving || !state.wasmLoaded} aria-label={nativeFileSaving ? 'Save As' : 'Download .kess'} onClick={() => void saveDocument(true)}>
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
                  <span className="menu-group-label">Parameter studies</span>
                  {(['study_filter', 'study_driver'] as const).map((id) => (
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
            <button aria-haspopup="menu" aria-expanded={openMenu === 'analyze'} onClick={() => toggleMenu('analyze')}>Analyze</button>
            {openMenu === 'analyze' && <div className="menu-popover analyze-menu" role="menu" aria-label="Analyze menu">
              <button role="menuitem" aria-label="Work with an AI agent…" disabled={!state.wasmLoaded} onClick={() => { setAgentProposalOpen(true); setOpenMenu(null); }}><strong>Work with an AI agent…</strong><small>Send a task, inspect the reply, and test it locally</small></button>
              {acceptedProposal && state.code === acceptedProposal.after && <button role="menuitem" onClick={undoAgentProposal}><strong>Undo AI agent change</strong><small>Restore the exact circuit from before the last accepted reply</small></button>}
              <div className="menu-separator" role="separator" />
              <button role="menuitem" aria-label="Test parameter variations…" disabled={!state.wasmLoaded || state.simulationState === 'running'} onClick={() => { setStudyOpen(true); setOpenMenu(null); }}><strong>Test parameter variations…</strong><small>Sweep values, tolerances, loads, and temperatures</small></button>
              <button role="menuitem" aria-label="Compare research data…" disabled={!state.wasmLoaded} onClick={() => { setResearchDataMounted(true); setResearchDataOpen(true); setOpenMenu(null); }}><strong>Compare research data…</strong><small>Compare CSV measurements, references, and simulation</small></button>
            </div>}
          </div>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'help'} onClick={() => toggleMenu('help')}>Help</button>
            {openMenu === 'help' && <div className="menu-popover" role="menu" aria-label="Help menu">
              <a role="menuitem" href={`${import.meta.env.BASE_URL}docs/guides/tutorial/`} target="_blank" rel="noreferrer">First circuit tutorial</a>
              <a role="menuitem" href={`${import.meta.env.BASE_URL}docs/`} target="_blank" rel="noreferrer">Documentation</a>
              <a role="menuitem" href={`${import.meta.env.BASE_URL}docs/guides/troubleshooting/`} target="_blank" rel="noreferrer">Troubleshooting</a>
              <a role="menuitem" href={`${import.meta.env.BASE_URL}changelog/`} target="_blank" rel="noreferrer">What’s new in {productVersion}</a>
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
        >{documentName}{documentSaving && <span className="document-save-progress" role="status">Saving…</span>}</div>
        <div className="global-actions">
          <span className={`compile-status compile-${state.compileState}`} role="status" aria-label={`Automatic source check: ${compileStatus}`} data-testid="compile-status" title={compileStatus}>
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
          const snapshot = fileInputRevisionRef.current;
          fileInputRevisionRef.current = null;
          if (snapshot && !isCurrentDocumentRevision(snapshot)) {
            setDocumentError('The circuit changed while choosing a file. Open it again to replace the current work.');
            return;
          }
          void openDocument(file).then(detachFileHandle).catch((cause: unknown) => {
            setDocumentError(`Could not open circuit: ${cause instanceof Error ? cause.message : String(cause)}`);
          });
        }}
      />
      <input
        ref={spiceInputRef}
        className="sr-only"
        type="file"
        accept=".cir,.sp,.spice,.net,text/plain"
        aria-label="Import SPICE netlist"
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) void importSpiceFile(file);
        }}
      />
      {state.draftRestored && <div className="draft-notice" role="status">Unsaved browser draft restored. Use File to save it here or download a portable .kess copy.</div>}
      {state.draftStorageError && <div className="draft-notice storage-warning" role="status">
        <span>{state.draftStorageError}</span>
        <button className="secondary-button" disabled={documentSaving} onClick={() => void saveDocument(true)}>Save a .kess copy</button>
      </div>}
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
        source={state.code}
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
      {state.wasmLoaded && <StudyDialog open={studyOpen} source={state.code} name={documentName} resources={modelResources}
        onClose={() => setStudyOpen(false)} onApplySource={setCode} />}
      {state.wasmLoaded && <AgentProposalDialog open={agentProposalOpen} source={state.code} name={documentName}
        productVersion={productVersion} resources={modelResources} onAccept={acceptAgentProposal}
        onClose={() => setAgentProposalOpen(false)} />}
      {state.wasmLoaded && researchDataMounted && <Suspense fallback={null}><ResearchDataDialog open={researchDataOpen} evaluation={state.evaluation} circuitName={documentName} onClose={() => setResearchDataOpen(false)} /></Suspense>}
      <WorkspaceLayout
        resetRequest={resetRequest}
        sourceRevealRequest={sourceRevealRequest}
        source={(panelControls) => <EditorPanel
          code={state.code}
          diagnostics={state.diagnostics}
          compileSucceeded={state.compileSucceeded}
          compileState={state.compileState}
          revealRequest={sourceRevealRequest}
          onCodeChange={setCode}
          panelControls={panelControls}
        />}
        schematic={(panelControls) => <SchematicPanel schematic={state.schematic} circuitIr={state.circuitIr} svg={state.schematicSvg} panelControls={panelControls} sourceFeedback={sourceFeedback} onResolveSource={resolveSource} />}
        results={(panelControls) => <ResultsPanel
          state={state.simulationState}
          hasSimulationAttempt={state.hasSimulationAttempt}
          message={state.simulationMessage}
          evaluation={state.evaluation}
          compileSucceeded={state.compileSucceeded}
          sourceFeedback={sourceFeedback}
          simulationConfigured={Boolean(state.circuitIr?.analyses.length)}
          onResolveSource={resolveSource}
          onRun={() => void run()}
          onCancel={cancel}
          panelControls={panelControls}
        />}
      />
    </main>
  );
}
