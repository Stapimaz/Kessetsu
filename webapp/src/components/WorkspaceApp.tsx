import { CheckCircle2, ChevronRight, CircleAlert, LoaderCircle, Share2, Square, Zap } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useKessetsuWorkspace, examples, type ExampleId } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { EditorPanel } from './EditorPanel';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';
import { WorkspaceLayout } from './WorkspaceLayout';

type MenuId = 'file' | 'view' | 'help';

export function WorkspaceApp() {
  const { state, setCode, loadExample, run, cancel, createExport, share } = useKessetsuWorkspace();
  const [openMenu, setOpenMenu] = useState<MenuId | null>(null);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [resetRequest, setResetRequest] = useState(0);
  const menusRef = useRef<HTMLElement>(null);
  const selectedExample = Object.entries(examples).find(([, example]) => example.source === state.code)?.[0] as ExampleId | undefined;
  const documentName = selectedExample ? examples[selectedExample].label : 'Untitled circuit';
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
    document.title = `${documentName} — Kessetsu`;
  }, [documentName]);

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
    loadExample(id);
    setExamplesOpen(false);
    setOpenMenu(null);
  };

  return (
    <main className="app-shell">
      <header className="app-menubar">
        <a className="wordmark" href="./" aria-label="Kessetsu home">KESSETSU</a>
        <nav className="application-menus" aria-label="Application menu" ref={menusRef}>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'file'} onClick={() => toggleMenu('file')}>File</button>
            {openMenu === 'file' && <div className="menu-popover file-menu" role="menu" aria-label="File menu">
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
                </div>}
              </div>
            </div>}
          </div>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'view'} onClick={() => toggleMenu('view')}>View</button>
            {openMenu === 'view' && <div className="menu-popover" role="menu" aria-label="View menu">
              <button role="menuitem" onClick={() => { setResetRequest((current) => current + 1); setOpenMenu(null); }}>
                <span>Reset panel layout</span>
              </button>
            </div>}
          </div>
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'help'} onClick={() => toggleMenu('help')}>Help</button>
            {openMenu === 'help' && <div className="menu-popover" role="menu" aria-label="Help menu">
              <a role="menuitem" href="https://github.com/Stapimaz/Kessetsu/blob/main/docs/README.md" target="_blank" rel="noreferrer">Documentation</a>
              <a role="menuitem" href="https://github.com/Stapimaz/Kessetsu" target="_blank" rel="noreferrer" aria-label="Kessetsu source code and AGPL license">Source and license</a>
            </div>}
          </div>
        </nav>
        <div className="document-title" title={documentName}>{documentName}</div>
        <div className="global-actions">
          <span className="share-status" role="status">{state.shareMessage}</span>
          <span className={`compile-status compile-${state.compileState}`} role="status" aria-label={`Automatic circuit check: ${compileStatus}`} data-testid="compile-status" title={compileStatus}>
            <CompileStatusIcon size={14} /><span>{compileStatus}</span>
          </span>
          {state.simulationState === 'running'
            ? <button className="run-button cancel-button" aria-label="Cancel" onClick={cancel}><Square size={13} /> <span>Cancel</span></button>
            : <button className="run-button" aria-label="Run" onClick={() => void run()} disabled={!state.compileSucceeded}><Zap size={15} /> <span>Run</span></button>}
          <ArtifactBar spice={state.spiceNetlist} models={state.modelManifest} enabled={state.compileSucceeded}
            capabilities={state.exportCapabilities} message={state.exportMessage} onExport={createExport} />
          <button className="share-button" onClick={() => void share()} disabled={!state.compileSucceeded} aria-label="Share circuit">
            <Share2 size={15} /> <span>Share</span>
          </button>
        </div>
      </header>
      {state.wasmError && <div className="global-error" role="alert">Core failed to initialize: {state.wasmError}</div>}
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
          panelControls={panelControls}
        />}
      />
    </main>
  );
}
