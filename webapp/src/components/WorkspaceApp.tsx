import { Moon, Share2, Sun, Play, Square, Zap } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useKessetsuWorkspace, examples, type ExampleId } from '../hooks/useKessetsuWorkspace';
import { ArtifactBar } from './ArtifactBar';
import { EditorPanel } from './EditorPanel';
import { ResultsPanel } from './ResultsPanel';
import { SchematicPanel } from './SchematicPanel';
import { WorkspaceLayout } from './WorkspaceLayout';

export function WorkspaceApp() {
  const { state, setCode, loadExample, compile, run, cancel, createExport, share } = useKessetsuWorkspace();
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const selectedExample = Object.entries(examples).find(([, example]) => example.source === state.code)?.[0] ?? 'custom';

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.title = 'Kessetsu Web Hub';
  }, [theme]);

  return (
    <main className="app-shell">
      <header className="product-header">
        <div>
          <a className="wordmark" href="./" aria-label="Kessetsu home">KESSETSU</a>
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
      <div className="command-toolbar" aria-label="Circuit actions">
        <select aria-label="Example circuit" value={selectedExample}
          onChange={(event) => event.target.value !== 'custom' && loadExample(event.target.value as ExampleId)}>
          {selectedExample === 'custom' && <option value="custom">Shared / custom circuit</option>}
          {Object.entries(examples).map(([id, example]) => <option key={id} value={id}>{example.label}</option>)}
        </select>
        <button className="secondary-button" onClick={compile} disabled={!state.wasmLoaded}>
          <Play size={14} /> {state.wasmLoaded ? 'Check' : 'Core…'}
        </button>
        {state.simulationState === 'running'
          ? <button className="run-button cancel-button" onClick={cancel}><Square size={13} /> Cancel</button>
          : <button className="run-button" onClick={() => void run()} disabled={!state.compileSucceeded}><Zap size={15} /> Run</button>}
        <ArtifactBar spice={state.spiceNetlist} models={state.modelManifest} enabled={state.compileSucceeded}
          capabilities={state.exportCapabilities} message={state.exportMessage} onExport={createExport} />
      </div>
      <WorkspaceLayout source={<EditorPanel
          code={state.code}
          diagnostics={state.diagnostics}
          compileSucceeded={state.compileSucceeded}
          onCodeChange={setCode}
        />}
        schematic={<SchematicPanel schematic={state.schematic} svg={state.schematicSvg} />}
        results={<ResultsPanel
          state={state.simulationState}
          message={state.simulationMessage}
          evaluation={state.evaluation}
        />}
      />
    </main>
  );
}
