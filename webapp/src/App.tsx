import { useCallback, useEffect, useState } from 'react';
import Editor from '@monaco-editor/react';
import { CircuitBoard, Code2, Download, Play, Terminal as TerminalIcon } from 'lucide-react';
import init, { compile_netlang, compile_schema_version } from 'netlang-core';
import defaultCircuit from '../../examples/demo_circuit.nl?raw';
import './monaco';

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
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);

  useEffect(() => {
    init()
      .then(() => setIsWasmLoaded(true))
      .catch((error: unknown) => setWasmError(errorMessage(error)));
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
