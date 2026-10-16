import Editor, { type OnMount } from '@monaco-editor/react';
import { ArrowRightLeft, Clipboard, Code2, FileUp, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { import_spice_netlist, spice_import_schema_version } from 'kessetsu-core';
import type { editor } from 'monaco-editor';
import type { CompileDiagnostic, CompileState, SpiceImportDiagnostic, SpiceImportReport } from '../domain';
import { monaco } from '../monaco';
import { assertValidSpiceReport, importDiagnosticLabel, SpiceImportError } from '../spiceImport';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

const SAMPLE_SPICE_NETLIST = `* 1 kHz RC Low-Pass Filter
V1 in 0 AC 1
R1 in out 1k
C1 out 0 159.154943n
.ac dec 40 10 100k
.end`;

interface Props {
  code: string;
  diagnostics: CompileDiagnostic[];
  compileSucceeded: boolean;
  compileState: CompileState;
  wasmLoaded?: boolean;
  revealRequest: number;
  onCodeChange(code: string): void;
  onChooseSpiceFile?(): void;
  onSpiceConverted?(kessSource: string, componentCount: number): void;
  panelControls: PanelWindowControls;
}

export function EditorPanel({
  code,
  diagnostics,
  compileSucceeded,
  compileState,
  wasmLoaded = true,
  revealRequest,
  onCodeChange,
  onChooseSpiceFile,
  onSpiceConverted,
  panelControls,
}: Props) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<editor.ITextModel | null>(null);
  const previousRevealRequest = useRef(revealRequest);
  const [converterOpen, setConverterOpen] = useState(false);
  const [spiceInput, setSpiceInput] = useState('');
  const [converterError, setConverterError] = useState('');
  const [converterDiagnostics, setConverterDiagnostics] = useState<SpiceImportDiagnostic[]>([]);

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

  const reveal = (diagnostic: CompileDiagnostic) => {
    if (!diagnostic.line) return;
    editorRef.current?.revealLineInCenter(diagnostic.line);
    editorRef.current?.setPosition({ lineNumber: diagnostic.line, column: diagnostic.column ?? 1 });
    editorRef.current?.focus();
  };

  const insertSnippet = (snippet: string, placement: 'top' | 'bottom' = 'bottom') => {
    const trimmed = code.replace(/\s+$/, '');
    let nextCode: string;
    if (placement === 'top') {
      const lines = trimmed ? trimmed.split(/\r?\n/) : [];
      let insertIndex = 0;
      while (insertIndex < lines.length && /^\s*(?:\/\/|net\b|param\b)/.test(lines[insertIndex])) {
        insertIndex += 1;
      }
      lines.splice(insertIndex, 0, snippet);
      nextCode = `${lines.join('\n')}\n`;
    } else {
      nextCode = trimmed ? `${trimmed}\n${snippet}\n` : `${snippet}\n`;
    }
    onCodeChange(nextCode);
    editorRef.current?.focus();
  };

  const runSpiceConversion = (rawSpice: string) => {
    setConverterError('');
    setConverterDiagnostics([]);
    try {
      const rawReport = import_spice_netlist(rawSpice) as SpiceImportReport;
      const report = assertValidSpiceReport(rawReport, spice_import_schema_version());
      if (!report.kess_source) return;
      if (onSpiceConverted) {
        onSpiceConverted(report.kess_source, report.summary.components);
      } else {
        onCodeChange(report.kess_source);
      }
      setConverterOpen(false);
      setSpiceInput('');
      editorRef.current?.focus();
    } catch (cause: unknown) {
      if (cause instanceof SpiceImportError) {
        setConverterError(cause.message);
        setConverterDiagnostics(cause.diagnostics.filter((item) => item.severity === 'error'));
      } else {
        setConverterError(cause instanceof Error ? cause.message : String(cause));
      }
      setConverterOpen(true);
    }
  };

  const pasteAndConvertSpice = async () => {
    try {
      const clip = await navigator.clipboard.readText();
      if (!clip.trim()) {
        setConverterError('Clipboard is empty. Copy a SPICE netlist first or paste it below.');
        return;
      }
      setSpiceInput(clip);
      runSpiceConversion(clip);
    } catch {
      setConverterError('Clipboard read was blocked by the browser. Paste the SPICE netlist into the box below with Ctrl+V.');
    }
  };

  return (
    <section className="workspace-panel editor-panel" aria-label="Kessetsu source editor">
      <PanelHeader controls={panelControls} icon={<Code2 size={15} />} title="Source">
        <div className="editor-quick-snippets" aria-label="Quick insert statements">
          <button
            type="button"
            className="spice-converter-trigger"
            aria-expanded={converterOpen}
            disabled={!wasmLoaded}
            title="Paste or convert a SPICE netlist (.cir / .sp) into Kessetsu source"
            onClick={() => {
              setConverterOpen((open) => !open);
              setConverterError('');
              setConverterDiagnostics([]);
            }}
          >
            <ArrowRightLeft size={11} /> SPICE → .kess
          </button>
          <span className="editor-snippet-divider" aria-hidden="true" />
          <button type="button" title="Insert DC operating point analysis" onClick={() => insertSnippet('simulate op')}>+ .op</button>
          <button type="button" title="Insert AC frequency sweep analysis" onClick={() => insertSnippet('simulate ac dec 40 10Hz 100kHz')}>+ .ac</button>
          <button type="button" title="Insert transient time-domain analysis" onClick={() => insertSnippet('simulate tran 10us 5ms')}>+ .tran</button>
          <button type="button" title="Insert sweepable circuit parameter for Parameter Study" onClick={() => insertSnippet('param R_load: Ohm = 10kOhm', 'top')}>+ param</button>
          <button type="button" title="Insert verifiable requirement assertion" onClick={() => insertSnippet('assert value(V(OUT)) >= 0.5V')}>+ assert</button>
        </div>
      </PanelHeader>
      {converterOpen && (
        <div className="spice-converter-drawer" role="region" aria-label="SPICE to Kessetsu converter">
          <div className="spice-converter-header">
            <div>
              <strong>SPICE → Kessetsu Converter</strong>
              <span>Paste an LTspice / Ngspice netlist below (or open a .cir/.sp file) to convert it into verified .kess source.</span>
            </div>
            <button type="button" aria-label="Close SPICE converter" onClick={() => setConverterOpen(false)}><X size={14} /></button>
          </div>
          <div className="spice-converter-toolbar">
            <button type="button" onClick={() => void pasteAndConvertSpice()}><Clipboard size={12} /> Paste &amp; convert</button>
            <button type="button" onClick={() => { setSpiceInput(SAMPLE_SPICE_NETLIST); setConverterError(''); setConverterDiagnostics([]); }}>Load example SPICE</button>
            {onChooseSpiceFile && <button type="button" onClick={() => { setConverterOpen(false); onChooseSpiceFile(); }}><FileUp size={12} /> Open .cir/.sp file…</button>}
          </div>
          <textarea
            aria-label="SPICE netlist input"
            rows={5}
            value={spiceInput}
            placeholder={'V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 159.15n\n.ac dec 40 10 100k\n.end'}
            onChange={(event) => {
              setSpiceInput(event.target.value);
              setConverterError('');
              setConverterDiagnostics([]);
            }}
          />
          {converterError && (
            <div className="spice-converter-error" role="alert">
              <strong>{converterError}</strong>
              {converterDiagnostics.length > 1 && (
                <ul>
                  {converterDiagnostics.slice(0, 4).map((diag, idx) => (
                    <li key={idx}>{importDiagnosticLabel(diag)}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
          <div className="spice-converter-actions">
            <button type="button" className="spice-convert-primary" disabled={!spiceInput.trim()} onClick={() => runSpiceConversion(spiceInput)}>
              <ArrowRightLeft size={12} /> Convert to .kess
            </button>
            <button type="button" onClick={() => setConverterOpen(false)}>Cancel</button>
          </div>
        </div>
      )}
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
