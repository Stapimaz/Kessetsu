import Editor, { type OnMount } from '@monaco-editor/react';
import { Code2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { editor } from 'monaco-editor';
import type { CompileDiagnostic, CompileState } from '../domain';
import { monaco } from '../monaco';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

interface Props {
  code: string;
  diagnostics: CompileDiagnostic[];
  compileSucceeded: boolean;
  compileState: CompileState;
  revealRequest: number;
  onCodeChange(code: string): void;
  panelControls: PanelWindowControls;
}

function findAnalysisLine(code: string, kind: 'op' | 'ac' | 'tran'): number | null {
  const pattern = new RegExp(`^\\s*simulate\\s+${kind}\\b`, 'i');
  const lines = code.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    if (pattern.test(lines[index])) return index + 1;
  }
  return null;
}

export function EditorPanel({
  code,
  diagnostics,
  compileSucceeded,
  compileState,
  revealRequest,
  onCodeChange,
  panelControls,
}: Props) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<editor.ITextModel | null>(null);
  const previousRevealRequest = useRef(revealRequest);

  const existingAnalyses = useMemo(() => ({
    op: findAnalysisLine(code, 'op'),
    ac: findAnalysisLine(code, 'ac'),
    tran: findAnalysisLine(code, 'tran'),
  }), [code]);

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

  const onMount: OnMount = (instance) => {
    editorRef.current = instance;
    modelRef.current = instance.getModel();
    applyMarkers();
  };

  const focusLine = (lineNumber: number) => {
    const instance = editorRef.current;
    const model = modelRef.current;
    if (!instance) return;
    instance.revealLineInCenter(lineNumber);
    const maxCol = model ? model.getLineMaxColumn(lineNumber) : 1;
    instance.setSelection({
      startLineNumber: lineNumber,
      startColumn: 1,
      endLineNumber: lineNumber,
      endColumn: maxCol,
    });
    instance.focus();
  };

  const reveal = (diagnostic: CompileDiagnostic) => {
    if (!diagnostic.line) return;
    editorRef.current?.revealLineInCenter(diagnostic.line);
    editorRef.current?.setPosition({ lineNumber: diagnostic.line, column: diagnostic.column ?? 1 });
    editorRef.current?.focus();
  };

  const insertOrFocusAnalysis = (kind: 'op' | 'ac' | 'tran', defaultSnippet: string) => {
    const existingLine = existingAnalyses[kind];
    if (existingLine !== null) {
      focusLine(existingLine);
      return;
    }
    const trimmed = code.replace(/\s+$/, '');
    const nextCode = trimmed ? `${trimmed}\n${defaultSnippet}\n` : `${defaultSnippet}\n`;
    onCodeChange(nextCode);
    editorRef.current?.focus();
  };

  const insertSnippet = (snippet: string, placement: 'top' | 'bottom' = 'bottom') => {
    const lines = code.split(/\r?\n/);
    const existingIdx = lines.findIndex((line) => line.trim() === snippet.trim());
    if (existingIdx >= 0) {
      focusLine(existingIdx + 1);
      return;
    }
    const trimmed = code.replace(/\s+$/, '');
    let nextCode: string;
    if (placement === 'top') {
      const nextLines = trimmed ? trimmed.split(/\r?\n/) : [];
      let insertIndex = 0;
      while (insertIndex < nextLines.length && /^\s*(?:\/\/|net\b|param\b)/.test(nextLines[insertIndex])) {
        insertIndex += 1;
      }
      nextLines.splice(insertIndex, 0, snippet);
      nextCode = `${nextLines.join('\n')}\n`;
    } else {
      nextCode = trimmed ? `${trimmed}\n${snippet}\n` : `${snippet}\n`;
    }
    onCodeChange(nextCode);
    editorRef.current?.focus();
  };

  return (
    <section className="workspace-panel editor-panel" aria-label="Kessetsu source editor">
      <PanelHeader controls={panelControls} icon={<Code2 size={15} />} title="Source">
        <div className="editor-quick-snippets" aria-label="Quick insert statements">
          <button
            type="button"
            className={existingAnalyses.op ? 'snippet-present' : undefined}
            title={existingAnalyses.op ? `Focus existing .op analysis on line ${existingAnalyses.op}` : 'Insert DC operating point analysis'}
            onClick={() => insertOrFocusAnalysis('op', 'simulate op')}
          >
            {existingAnalyses.op ? '.op' : '+ .op'}
          </button>
          <button
            type="button"
            className={existingAnalyses.ac ? 'snippet-present' : undefined}
            title={existingAnalyses.ac ? `Focus existing .ac analysis on line ${existingAnalyses.ac}` : 'Insert AC frequency sweep analysis'}
            onClick={() => insertOrFocusAnalysis('ac', 'simulate ac dec 40 10Hz 100kHz')}
          >
            {existingAnalyses.ac ? '.ac' : '+ .ac'}
          </button>
          <button
            type="button"
            className={existingAnalyses.tran ? 'snippet-present' : undefined}
            title={existingAnalyses.tran ? `Focus existing .tran analysis on line ${existingAnalyses.tran}` : 'Insert transient time-domain analysis'}
            onClick={() => insertOrFocusAnalysis('tran', 'simulate tran 10us 5ms')}
          >
            {existingAnalyses.tran ? '.tran' : '+ .tran'}
          </button>
          <button
            type="button"
            title="Insert sweepable circuit parameter for Parameter Study"
            onClick={() => insertSnippet('param R_load: Ohm = 10kOhm', 'top')}
          >
            + param
          </button>
          <button
            type="button"
            title="Insert verifiable requirement assertion"
            onClick={() => insertSnippet('assert value(V(OUT)) >= 0.5V')}
          >
            + assert
          </button>
        </div>
      </PanelHeader>
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
