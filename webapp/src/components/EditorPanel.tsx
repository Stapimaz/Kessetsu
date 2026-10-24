import Editor, { type OnMount } from '@monaco-editor/react';
import { Code2 } from 'lucide-react';
import { useCallback, useEffect, useRef } from 'react';
import type { editor } from 'monaco-editor';
import type { CompileDiagnostic, CompileState } from '../domain';
import { monaco } from '../monaco';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

export interface SymbolRevealRequest {
  id: number;
  symbol: string;
}

interface Props {
  code: string;
  diagnostics: CompileDiagnostic[];
  compileSucceeded: boolean;
  compileState: CompileState;
  revealRequest: number;
  symbolRevealRequest?: SymbolRevealRequest | null;
  onCodeChange(code: string): void;
  panelControls: PanelWindowControls;
}

function findSymbolLine(code: string, symbol: string): number | null {
  const candidates = symbol.includes('_') ? [symbol, symbol.split('_')[0], symbol.split('_').slice(1).join('_')] : [symbol];
  const lines = code.split(/\r?\n/);
  for (const candidate of candidates) {
    if (!candidate) continue;
    const escaped = candidate.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const declPattern = new RegExp(
      `^\\s*(?:net|source|current_source|resistor|capacitor|inductor|diode|bjt|transistor|mosfet|opamp|vcvs|vccs|ccvs|cccs|subcircuit_instance|module)\\s+${escaped}\\b|\\buse\\s+\\S+\\s+(?:as\\s+)?${escaped}\\b`,
      'i',
    );
    for (let i = 0; i < lines.length; i += 1) {
      if (declPattern.test(lines[i])) return i + 1;
    }
  }
  for (const candidate of candidates) {
    if (!candidate) continue;
    const escaped = candidate.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const fallbackPattern = new RegExp(`\\b${escaped}\\b`, 'i');
    for (let i = 0; i < lines.length; i += 1) {
      if (fallbackPattern.test(lines[i])) return i + 1;
    }
  }
  return null;
}

export function EditorPanel({
  code,
  diagnostics,
  compileSucceeded,
  compileState,
  revealRequest,
  symbolRevealRequest = null,
  onCodeChange,
  panelControls,
}: Props) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<editor.ITextModel | null>(null);
  const previousRevealRequest = useRef(revealRequest);
  const previousSymbolRevealId = useRef<number | null>(symbolRevealRequest?.id ?? null);

  const applyMarkers = useCallback(() => {
    const model = modelRef.current;
    if (!model) return;
    monaco.editor.setModelMarkers(model, 'kessetsu-core', diagnostics.map((diagnostic) => ({
      severity: diagnostic.severity === 'error'
        ? monaco.MarkerSeverity.Error
        : diagnostic.severity === 'warning'
          ? monaco.MarkerSeverity.Warning
          : monaco.MarkerSeverity.Info,
      message: `[${diagnostic.code}/${diagnostic.stage}] ${diagnostic.message}`,
      startLineNumber: diagnostic.line ?? 1,
      startColumn: diagnostic.column ?? 1,
      endLineNumber: diagnostic.line ?? 1,
      endColumn: (diagnostic.column ?? 1) + 1,
      code: diagnostic.code,
    })));
  }, [diagnostics]);

  useEffect(applyMarkers, [applyMarkers]);

  useEffect(() => {
    if (!panelControls.visible || previousRevealRequest.current === revealRequest) return;
    previousRevealRequest.current = revealRequest;
    const diagnostic = diagnostics.find((item) => item.severity === 'error' && item.line);
    // A recovery request must wait until the layout has actually restored Source.
    editorRef.current?.layout();
    if (diagnostic?.line) {
      editorRef.current?.revealLineInCenter(diagnostic.line);
      editorRef.current?.setPosition({ lineNumber: diagnostic.line, column: diagnostic.column ?? 1 });
    }
    editorRef.current?.focus();
  }, [diagnostics, revealRequest, panelControls.visible]);

  useEffect(() => {
    if (!panelControls.visible || !symbolRevealRequest || previousSymbolRevealId.current === symbolRevealRequest.id) return;
    previousSymbolRevealId.current = symbolRevealRequest.id;
    const lineNumber = findSymbolLine(code, symbolRevealRequest.symbol);
    const instance = editorRef.current;
    const model = modelRef.current;
    if (!instance || !lineNumber) return;
    instance.layout();
    instance.revealLineInCenter(lineNumber);
    const maxCol = model ? model.getLineMaxColumn(lineNumber) : 1;
    instance.setSelection({
      startLineNumber: lineNumber,
      startColumn: 1,
      endLineNumber: lineNumber,
      endColumn: maxCol,
    });
    instance.focus();
  }, [code, panelControls.visible, symbolRevealRequest]);

  const onMount: OnMount = (instance) => {
    editorRef.current = instance;
    modelRef.current = instance.getModel();
    applyMarkers();
  };

  const reveal = (diagnostic: CompileDiagnostic) => {
    if (!diagnostic.line) return;
    editorRef.current?.revealLineInCenter(diagnostic.line);
    editorRef.current?.setPosition({ lineNumber: diagnostic.line, column: diagnostic.column ?? 1 });
    editorRef.current?.focus();
  };

  return (
    <section className="workspace-panel editor-panel" aria-label="Kessetsu source editor">
      <PanelHeader controls={panelControls} icon={<Code2 size={15} />} title="Source" />
      <div className="editor-container">
        <Editor
          height="100%"
          language="kessetsu"
          theme="kessetsu-dark"
          value={code}
          onMount={onMount}
          onChange={(value) => onCodeChange(value ?? '')}
          options={{ minimap: { enabled: false }, fontSize: 14, lineHeight: 22, padding: { top: 12 }, automaticLayout: true }}
        />
      </div>
      <div className="inline-diagnostics" aria-label="Diagnostics" aria-live="polite">
        {compileSucceeded && diagnostics.length === 0 ? (
          <span className="diagnostic-ok" data-testid="compile-success">Source valid · ERC and schematic connectivity passed</span>
        ) : diagnostics.length > 0 ? diagnostics.map((diagnostic, index) => (
          <button
            key={`${diagnostic.code}-${index}`}
            className={`diagnostic-row diagnostic-${diagnostic.severity}`}
            onClick={() => reveal(diagnostic)}
            disabled={!diagnostic.line}
          >
            <span>{diagnostic.code}</span>
            <span>{diagnostic.message}</span>
            {diagnostic.line && <span className="diagnostic-location">L{diagnostic.line}:{diagnostic.column ?? 1}</span>}
          </button>
        )) : <span className="diagnostic-pending">{compileState === 'loading' ? 'Loading circuit checks…' : compileState === 'checking' ? 'Checking your changes…' : 'Source needs attention. See the editor status above.'}</span>}
      </div>
    </section>
  );
}
