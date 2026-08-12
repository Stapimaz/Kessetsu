import Editor, { type OnMount } from '@monaco-editor/react';
import { Code2, Play } from 'lucide-react';
import { useCallback, useEffect, useRef } from 'react';
import type { editor } from 'monaco-editor';
import type { CompileDiagnostic } from '../domain';
import { type ExampleId, examples } from '../hooks/useNetlangWorkspace';
import { monaco } from '../monaco';

interface Props {
  code: string;
  diagnostics: CompileDiagnostic[];
  wasmLoaded: boolean;
  compileSucceeded: boolean;
  onCodeChange(code: string): void;
  onCompile(): void;
  onExample(id: ExampleId): void;
}

export function EditorPanel({ code, diagnostics, wasmLoaded, compileSucceeded, onCodeChange, onCompile, onExample }: Props) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<editor.ITextModel | null>(null);

  const applyMarkers = useCallback(() => {
    const model = modelRef.current;
    if (!model) return;
    monaco.editor.setModelMarkers(model, 'netlang-core', diagnostics.map((diagnostic) => ({
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
    <section className="workspace-panel editor-panel" aria-label="NetLang source editor">
      <header className="workspace-header">
        <div className="header-title"><Code2 size={18} /><strong>NetLang</strong></div>
        <label className="example-picker">
          <span className="sr-only">Örnek devre</span>
          <select aria-label="Örnek devre" defaultValue="rc" onChange={(event) => onExample(event.target.value as ExampleId)}>
            {Object.entries(examples).map(([id, example]) => <option key={id} value={id}>{example.label}</option>)}
          </select>
        </label>
        <button className="secondary-button" onClick={onCompile} disabled={!wasmLoaded}>
          <Play size={14} /> {wasmLoaded ? 'Check' : 'Core…'}
        </button>
      </header>
      <div className="editor-container">
        <Editor
          height="100%"
          language="netlang"
          theme="netlang-dark"
          value={code}
          onMount={onMount}
          onChange={(value) => onCodeChange(value ?? '')}
          options={{ minimap: { enabled: false }, fontSize: 14, lineHeight: 22, padding: { top: 12 }, automaticLayout: true }}
        />
      </div>
      <div className="inline-diagnostics" aria-label="Diagnostics" aria-live="polite">
        {compileSucceeded && diagnostics.length === 0 ? (
          <span className="diagnostic-ok" data-testid="compile-success">ERC + schematic connectivity verified</span>
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
        )) : <span className="diagnostic-pending">Core hazırlanıyor…</span>}
      </div>
    </section>
  );
}
