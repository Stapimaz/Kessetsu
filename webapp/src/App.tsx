import { useCallback, useEffect, useRef, useState } from 'react';
import Editor from '@monaco-editor/react';
import { CircuitBoard, Code2, Download, Play, Terminal as TerminalIcon } from 'lucide-react';
import init, {
  compile_netlang,
  compile_schema_version,
  evaluate_browser_simulation,
  prepare_browser_simulation,
} from 'netlang-core';
import defaultCircuit from '../../examples/demo_circuit.nl?raw';
import './monaco';
import { BrowserSimulationRunner, SimulationCancelledError } from './simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from './simulation/types';

interface CompileDiagnostic {
  code: string;
  severity: 'error' | 'warning' | 'info';
  stage:
    | 'parse'
    | 'flatten'
    | 'semantic'
    | 'erc'
    | 'io'
    | 'cli'
    | 'simulation'
    | 'assertion'
    | 'schematic';
  message: string;
}

interface SchematicSummary {
  schema_version: string;
  connectivity: { verified: boolean };
  quality: { passed: boolean; issues: string[] };
}

interface CompileReport {
  schema_version: string;
  diagnostics: CompileDiagnostic[];
  schematic: SchematicSummary | null;
  schematic_svg: string | null;
  kicad_sch: string | null;
  spice_netlist: string | null;
}

interface UiDiagnostic {
  message: string;
  type: CompileDiagnostic['severity'];
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function downloadText(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function App() {
  const [code, setCode] = useState(defaultCircuit);
  const [diagnostics, setDiagnostics] = useState<UiDiagnostic[]>([]);
  const [success, setSuccess] = useState(false);
  const [wasmError, setWasmError] = useState<string | null>(null);
  const [isWasmLoaded, setIsWasmLoaded] = useState(false);
  const [schematic, setSchematic] = useState<SchematicSummary | null>(null);
  const [schematicSvg, setSchematicSvg] = useState('');
  const [kicadSch, setKicadSch] = useState('');
  const [spiceNetlist, setSpiceNetlist] = useState('');
  const [simulationState, setSimulationState] = useState<'idle' | 'running' | 'succeeded' | 'failed' | 'cancelled'>('idle');
  const [simulationMessage, setSimulationMessage] = useState('Hazır');
  const [evaluation, setEvaluation] = useState<BrowserEvaluation | null>(null);
  const runnerRef = useRef<BrowserSimulationRunner | null>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);

  useEffect(() => {
    init()
      .then(() => setIsWasmLoaded(true))
      .catch((error: unknown) => setWasmError(errorMessage(error)));
  }, []);

  useEffect(() => {
    const runner = new BrowserSimulationRunner();
    runnerRef.current = runner;
    return () => runner.dispose();
  }, []);

  const clearArtifacts = useCallback(() => {
    setSuccess(false);
    setSchematic(null);
    setSchematicSvg('');
    setSpiceNetlist('');
    setKicadSch('');
  }, []);

  const compileCode = useCallback(() => {
    if (!isWasmLoaded) return;
    try {
      const result = compile_netlang(code) as CompileReport;
      const supportedSchema = compile_schema_version();
      if (result.schema_version !== supportedSchema) {
        throw new Error(`Unsupported compile report schema: ${result.schema_version}`);
      }

      const nextDiagnostics = result.diagnostics ?? [];
      setDiagnostics(
        nextDiagnostics.map((diagnostic) => ({
          message: `[${diagnostic.code}/${diagnostic.stage}] ${diagnostic.message}`,
          type: diagnostic.severity,
        })),
      );
      if (nextDiagnostics.some((diagnostic) => diagnostic.severity === 'error')) {
        clearArtifacts();
        return;
      }
      if (!result.schematic?.connectivity.verified || !result.schematic_svg) {
        throw new Error('Core did not return a connectivity-verified schematic artifact');
      }

      setSuccess(true);
      setSchematic(result.schematic);
      setSchematicSvg(result.schematic_svg);
      setSpiceNetlist(result.spice_netlist ?? '');
      setKicadSch(result.kicad_sch ?? '');
    } catch (error: unknown) {
      setDiagnostics([{ message: `WASM Execution Error: ${errorMessage(error)}`, type: 'error' }]);
      clearArtifacts();
    }
  }, [clearArtifacts, code, isWasmLoaded]);

  useEffect(() => compileCode(), [compileCode]);

  const runSimulation = useCallback(async () => {
    if (!isWasmLoaded || !success || !runnerRef.current) return;
    setEvaluation(null);
    setSimulationState('running');
    setSimulationMessage('Simulation hazırlanıyor');
    try {
      const plan = prepare_browser_simulation(code) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('Devrede çalıştırılacak simulate komutu yok');
      const simulation = await runnerRef.current.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => setSimulationMessage(progress.message),
      });
      const result = evaluate_browser_simulation(code, simulation) as BrowserEvaluation;
      setEvaluation(result);
      setSimulationState('succeeded');
      setSimulationMessage(`${result.simulation.datasets.length} analiz tamamlandı`);
    } catch (error: unknown) {
      if (error instanceof SimulationCancelledError) {
        setSimulationState('cancelled');
        setSimulationMessage('Simulation iptal edildi');
        return;
      }
      setSimulationState('failed');
      setSimulationMessage(errorMessage(error));
    }
  }, [code, isWasmLoaded, success]);

  const cancelSimulation = useCallback(() => {
    runnerRef.current?.cancel();
    setSimulationState('cancelled');
    setSimulationMessage('Simulation iptal edildi');
  }, []);

  return (
    <main className="app-container">
      <section className="panel left-panel" aria-label="NetLang source editor">
        <header className="panel-header">
          <Code2 size={18} color="var(--accent)" />
          <span>NetLang Editor</span>
          <button className="compile-btn" onClick={compileCode} disabled={!isWasmLoaded}>
            <Play size={14} aria-hidden="true" />
            {isWasmLoaded ? 'Derle & ERC' : 'WASM…'}
          </button>
        </header>
        <div className="editor-container">
          <Editor
            height="100%"
            defaultLanguage="plaintext"
            theme="vs-dark"
            value={code}
            onChange={(value) => setCode(value ?? '')}
            options={{ minimap: { enabled: false }, fontSize: 15 }}
          />
        </div>
      </section>

      <section className="panel right-panel" aria-label="Compile results">
        <header className="panel-header panel-header-spread">
          <div className="header-title">
            <CircuitBoard size={18} color="var(--success)" />
            <span>Canonical Schematic</span>
          </div>
          <div className="artifact-actions">
            {simulationState === 'running' ? (
              <button className="artifact-btn" onClick={cancelSimulation}>İptal</button>
            ) : (
              <button className="artifact-btn run-btn" onClick={() => void runSimulation()} disabled={!success}>
                <Play size={14} aria-hidden="true" /> Simüle Et
              </button>
            )}
            <button
              className="artifact-btn"
              disabled={!schematicSvg}
              onClick={() => downloadText('circuit.svg', schematicSvg, 'image/svg+xml')}
            >
              <Download size={14} aria-hidden="true" /> SVG
            </button>
            <button
              className="artifact-btn"
              disabled={!kicadSch}
              onClick={() => downloadText('circuit.kicad_sch', kicadSch, 'text/plain')}
            >
              <Download size={14} aria-hidden="true" /> KiCad
            </button>
            <button
              className="artifact-btn"
              disabled={!spiceNetlist}
              onClick={() => downloadText('circuit.spice', spiceNetlist, 'text/plain')}
            >
              <Download size={14} aria-hidden="true" /> SPICE
            </button>
          </div>
        </header>

        <div
          className={`schematic-surface${dragStart ? ' is-dragging' : ''}`}
          onWheel={(event) => {
            event.preventDefault();
            setZoom((current) => Math.max(0.2, Math.min(4, current - event.deltaY * 0.002)));
          }}
          onMouseDown={(event) => setDragStart({ x: event.clientX - pan.x, y: event.clientY - pan.y })}
          onMouseMove={(event) => {
            if (dragStart) setPan({ x: event.clientX - dragStart.x, y: event.clientY - dragStart.y });
          }}
          onMouseUp={() => setDragStart(null)}
          onMouseLeave={() => setDragStart(null)}
        >
          {schematicSvg ? (
            <div
              className="schematic-document"
              data-testid="canonical-schematic"
              data-quality={schematic?.quality.passed ? 'pass' : 'warning'}
              style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
              dangerouslySetInnerHTML={{ __html: schematicSvg }}
            />
          ) : (
            <div className="empty-state">Şema hesaplanıyor…</div>
          )}
        </div>

        <section className="simulation-summary" data-testid="simulation-summary" data-state={simulationState}>
          <strong>Browser Simulation</strong>
          <span>{simulationMessage}</span>
          {evaluation && (
            <>
              <span>{evaluation.simulation.simulator.version}</span>
              <div className="assertion-list">
                {evaluation.assertions.assertions.map((assertion) => (
                  <span key={assertion.code} className={`assertion assertion-${assertion.status.toLowerCase()}`} title={assertion.message}>
                    <span hidden data-assertion-code={assertion.code} data-actual={assertion.actual ?? ''} />
                    {assertion.status.toUpperCase()} · {assertion.metric}({assertion.signal})
                    {assertion.actual == null ? '' : ` = ${assertion.actual.toPrecision(5)}`}
                    {assertion.message ? ` — ${assertion.message}` : ''}
                  </span>
                ))}
              </div>
              <div className="dataset-kinds" aria-label="Analysis datasets">
                {evaluation.simulation.datasets.map((dataset) => (
                  <span key={dataset.index} data-testid="dataset-kind">{dataset.data.kind}</span>
                ))}
              </div>
            </>
          )}
        </section>

        <header className="panel-header subpanel-header">
          <Code2 size={18} color="#f59e0b" />
          <span>Generated SPICE Netlist</span>
        </header>
        <div className="terminal-container spice-output">
          {spiceNetlist ? <pre>{spiceNetlist}</pre> : <div className="empty-state">Geçerli devre bekleniyor…</div>}
        </div>

        <header className="panel-header subpanel-header">
          <TerminalIcon size={18} color="var(--text-muted)" />
          <span>Diagnostics</span>
        </header>
        <div className="terminal-container" aria-live="polite">
          {success && (
            <div className="success-msg" data-testid="compile-success">
              ERC ve şema connectivity kontrolü başarılı.
            </div>
          )}
          {wasmError && <div className="error-msg">WASM init failed: {wasmError}</div>}
          {diagnostics.map((diagnostic, index) => (
            <div key={`${diagnostic.message}-${index}`} className={diagnostic.type === 'error' ? 'error-msg' : ''}>
              [{diagnostic.type.toUpperCase()}] {diagnostic.message}
            </div>
          ))}
          {!success && !wasmError && diagnostics.length === 0 && <div className="empty-state">Derlemeye hazır…</div>}
        </div>
      </section>
    </main>
  );
}

export default App;
