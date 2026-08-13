import { Moon, Share2, Sun } from 'lucide-react';
import { useEffect, useState } from 'react';
import { ArtifactBar } from './components/ArtifactBar';
import { EditorPanel } from './components/EditorPanel';
import { ResultsPanel } from './components/ResultsPanel';
import { SchematicPanel } from './components/SchematicPanel';
import { useNetlangWorkspace } from './hooks/useNetlangWorkspace';

function App() {
  const { state, setCode, loadExample, compile, run, cancel, createExport, share } = useNetlangWorkspace();
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  return (
    <main className="app-shell">
      <header className="product-header">
        <div><span className="wordmark">NETLANG</span><span className="tagline">Circuit engineering, executable.</span></div>
        <div className="product-actions">
          <span className="share-status" role="status">{state.shareMessage}</span>
          <button className="share-button" onClick={() => void share()} disabled={!state.compileSucceeded} aria-label="Share circuit">
            <Share2 size={15} /> Share
          </button>
          <button className="theme-button" onClick={() => setTheme((current) => current === 'dark' ? 'light' : 'dark')} aria-label="Tema değiştir">
            {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
          </button>
        </div>
      </header>
      {state.wasmError && <div className="global-error" role="alert">Core başlatılamadı: {state.wasmError}</div>}
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

export default App;
