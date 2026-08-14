import { Moon, Share2, Sun } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useKessetsuWorkspace } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { EditorPanel } from './EditorPanel';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';

export function WorkspaceApp() {
  const { state, setCode, loadExample, compile, run, cancel, createExport, share } = useKessetsuWorkspace();
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.title = 'Kessetsu Web Hub';
  }, [theme]);

  return (
    <main className="app-shell">
      <header className="product-header">
        <div>
          <a className="wordmark" href="./" aria-label="Kessetsu home">KESSETSU</a>
          <span className="tagline">Circuit engineering, executable.</span>
        </div>
        <div className="product-actions">
          <span className="share-status" role="status">{state.shareMessage}</span>
          <a
            className="license-link"
            href="https://github.com/Stapimaz/Kessetsu"
            target="_blank"
            rel="noreferrer"
            aria-label="Kessetsu source code and AGPL license; provided without warranty"
          >
            Source · AGPLv3
          </a>
          <button className="share-button" onClick={() => void share()} disabled={!state.compileSucceeded} aria-label="Share circuit">
            <Share2 size={15} /> Share
          </button>
          <button className="theme-button" onClick={() => setTheme((current) => current === 'dark' ? 'light' : 'dark')} aria-label="Change theme">
            {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
          </button>
        </div>
      </header>
      {state.wasmError && <div className="global-error" role="alert">Core failed to initialize: {state.wasmError}</div>}
      <div className="workspace-grid">
        <EditorPanel
          code={state.code}
          diagnostics={state.diagnostics}
          wasmLoaded={state.wasmLoaded}
          compileSucceeded={state.compileSucceeded}
          onCodeChange={setCode}
          onCompile={compile}
          onExample={loadExample}
        />
        <SchematicPanel schematic={state.schematic} svg={state.schematicSvg} />
        <ResultsPanel
          state={state.simulationState}
          message={state.simulationMessage}
          evaluation={state.evaluation}
          canRun={state.compileSucceeded}
          onRun={() => void run()}
          onCancel={cancel}
        />
      </div>
      <ArtifactBar
        spice={state.spiceNetlist}
        models={state.modelManifest}
        enabled={state.compileSucceeded}
        capabilities={state.exportCapabilities}
        message={state.exportMessage}
        onExport={createExport}
      />
    </main>
  );
}
