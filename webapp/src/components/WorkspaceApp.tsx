import { CheckCircle2, Share2, Square, Zap } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useKessetsuWorkspace, examples, type ExampleId } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { EditorPanel } from './EditorPanel';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';
import { WorkspaceLayout } from './WorkspaceLayout';

type MenuId = 'file' | 'simulation' | 'view' | 'help';

export function WorkspaceApp() {
  const { state, setCode, loadExample, compile, run, cancel, createExport, share } = useKessetsuWorkspace();
  const [openMenu, setOpenMenu] = useState<MenuId | null>(null);
  const [resetRequest, setResetRequest] = useState(0);
  const menusRef = useRef<HTMLElement>(null);
  const selectedExample = Object.entries(examples).find(([, example]) => example.source === state.code)?.[0] as ExampleId | undefined;
  const documentName = selectedExample ? examples[selectedExample].label : 'Untitled circuit';

  useEffect(() => {
    document.documentElement.dataset.theme = 'dark';
    document.title = `${documentName} — Kessetsu`;
  }, [documentName]);

  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!menusRef.current?.contains(event.target as Node)) setOpenMenu(null);
    };
    const closeWithEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpenMenu(null);
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
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key === 'Enter' && state.compileSucceeded && state.simulationState !== 'running') {
        event.preventDefault();
        void run();
      } else if (event.shiftKey && event.key.toLowerCase() === 'b' && state.wasmLoaded) {
        event.preventDefault();
        compile();
      }
    };
    window.addEventListener('keydown', runShortcut);
    return () => window.removeEventListener('keydown', runShortcut);
  }, [compile, run, state.compileSucceeded, state.simulationState, state.wasmLoaded]);

  const toggleMenu = (menu: MenuId) => setOpenMenu((current) => current === menu ? null : menu);
  const selectExample = (id: ExampleId) => {
    loadExample(id);
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
              <span className="menu-heading">Open example</span>
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
          <div className="application-menu">
            <button aria-haspopup="menu" aria-expanded={openMenu === 'simulation'} onClick={() => toggleMenu('simulation')}>Simulation</button>
            {openMenu === 'simulation' && <div className="menu-popover" role="menu" aria-label="Simulation menu">
              <button role="menuitem" disabled={!state.wasmLoaded} onClick={() => { compile(); setOpenMenu(null); }}>
                <span>Check circuit</span><kbd>Ctrl+Shift+B</kbd>
              </button>
              {state.simulationState === 'running'
                ? <button role="menuitem" onClick={() => { cancel(); setOpenMenu(null); }}><span>Cancel simulation</span></button>
                : <button role="menuitem" disabled={!state.compileSucceeded} onClick={() => { void run(); setOpenMenu(null); }}>
                  <span>Run simulation</span><kbd>Ctrl+Enter</kbd>
                </button>}
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
          <button className="secondary-button check-button" aria-label="Check" onClick={compile} disabled={!state.wasmLoaded}>
            <CheckCircle2 size={14} /> <span>{state.wasmLoaded ? 'Check' : 'Core…'}</span>
          </button>
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
