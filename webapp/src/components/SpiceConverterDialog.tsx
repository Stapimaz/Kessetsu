import { ArrowRightLeft, Check, Clipboard, Copy, FileUp, Sparkles, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { compile_kessetsu, import_spice_netlist, spice_import_schema_version } from 'kessetsu-core';
import type { CompileReport, SpiceImportDiagnostic, SpiceImportReport } from '../domain';
import { assertValidSpiceReport, importDiagnosticLabel, mergeConvertedKessIntoSource, SpiceImportError } from '../spiceImport';

const SAMPLE_RC_SPICE = `* 1 kHz RC Low-Pass Filter
V1 in 0 AC 1
R1 in out 1k
C1 out 0 159.154943n
.ac dec 40 10 100k
.end`;

const SAMPLE_OPAMP_SPICE = `* Non-Inverting Op-Amp Stage (Gain = 2)
VCC vcc 0 15
VEE 0 vee 15
VIN in 0 SIN(0 0.5 1k) AC 1
R1 fb 0 10k
R2 out fb 10k
XU1 in fb vcc vee out OPAMP
.op
.ac dec 30 10 1Meg
.end`;

export type ApplyMode = 'replace' | 'insert';

interface Props {
  open: boolean;
  currentSource: string;
  onClose: () => void;
  onChooseSpiceFile: () => void;
  onApply: (nextSource: string, mode: ApplyMode, componentCount: number) => void;
}

export function SpiceConverterDialog({
  open,
  currentSource,
  onClose,
  onChooseSpiceFile,
  onApply,
}: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [spiceInput, setSpiceInput] = useState(SAMPLE_RC_SPICE);
  const [applyMode, setApplyMode] = useState<ApplyMode>('replace');
  const [copied, setCopied] = useState(false);
  const [clipboardError, setClipboardError] = useState('');

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      setCopied(false);
      setClipboardError('');
      dialog.showModal();
    }
    if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  const conversion = useMemo<{
    report: SpiceImportReport | null;
    error: string;
    diagnostics: SpiceImportDiagnostic[];
  }>(() => {
    const trimmed = spiceInput.trim();
    if (!open || !trimmed) {
      return { report: null, error: '', diagnostics: [] };
    }
    try {
      const rawReport = import_spice_netlist(spiceInput) as SpiceImportReport;
      const report = assertValidSpiceReport(rawReport, spice_import_schema_version());
      return {
        report,
        error: '',
        diagnostics: report.diagnostics,
      };
    } catch (cause: unknown) {
      if (cause instanceof SpiceImportError) {
        return {
          report: null,
          error: cause.message,
          diagnostics: cause.diagnostics,
        };
      }
      return {
        report: null,
        error: cause instanceof Error ? cause.message : String(cause),
        diagnostics: [],
      };
    }
  }, [open, spiceInput]);

  const previewSource = useMemo(() => {
    const kess = conversion.report?.kess_source;
    if (!kess) return '';
    return applyMode === 'insert'
      ? mergeConvertedKessIntoSource(currentSource, kess)
      : kess;
  }, [applyMode, conversion.report?.kess_source, currentSource]);

  const mergedValidation = useMemo(() => {
    if (applyMode !== 'insert' || !previewSource) return null;
    try {
      const report = compile_kessetsu(previewSource) as CompileReport;
      const errors = report.diagnostics.filter((d) => d.severity === 'error');
      return { valid: errors.length === 0, errors };
    } catch {
      return null;
    }
  }, [applyMode, previewSource]);

  const pasteFromClipboard = async () => {
    setClipboardError('');
    try {
      const text = await navigator.clipboard.readText();
      if (!text.trim()) {
        setClipboardError('Clipboard is empty. Copy a SPICE netlist first or paste it below.');
        return;
      }
      setSpiceInput(text);
    } catch {
      setClipboardError('Clipboard read was blocked by the browser. Focus the SPICE box and press Ctrl+V.');
    }
  };

  const copyPreview = async () => {
    if (!previewSource) return;
    try {
      await navigator.clipboard.writeText(previewSource);
      setCopied(true);
      globalThis.setTimeout(() => setCopied(false), 2000);
    } catch {
      setClipboardError('Could not copy automatically. Select the preview text and press Ctrl+C.');
    }
  };

  const applyToCircuit = () => {
    if (!previewSource || !conversion.report) return;
    onApply(previewSource, applyMode, conversion.report.summary.components);
    dialogRef.current?.close();
  };

  const canApply = Boolean(previewSource) && (applyMode === 'replace' || (mergedValidation?.valid ?? true));

  return (
    <dialog
      ref={dialogRef}
      className="app-dialog spice-converter-dialog"
      aria-labelledby="spice-converter-title"
      onClose={onClose}
    >
      <header>
        <div>
          <h2 id="spice-converter-title">SPICE → Kessetsu Converter</h2>
          <p>Convert an LTspice or Ngspice netlist into verified .kess source, inspect the live preview, and copy, insert, or replace your circuit.</p>
        </div>
        <button type="button" aria-label="Close SPICE converter" onClick={() => dialogRef.current?.close()}>
          <X size={18} />
        </button>
      </header>

      <div className="spice-dialog-grid">
        <section className="spice-dialog-pane" aria-label="SPICE netlist input pane">
          <div className="spice-pane-header">
            <strong>1. SPICE Netlist (.cir / .sp)</strong>
            <div className="spice-pane-actions">
              <button type="button" onClick={() => void pasteFromClipboard()}>
                <Clipboard size={12} /> Paste
              </button>
              <button type="button" onClick={() => { setSpiceInput(SAMPLE_RC_SPICE); setClipboardError(''); }}>
                RC example
              </button>
              <button type="button" onClick={() => { setSpiceInput(SAMPLE_OPAMP_SPICE); setClipboardError(''); }}>
                <Sparkles size={12} /> Op-Amp example
              </button>
              <button
                type="button"
                onClick={() => {
                  dialogRef.current?.close();
                  onChooseSpiceFile();
                }}
              >
                <FileUp size={12} /> Open file…
              </button>
            </div>
          </div>

          <textarea
            aria-label="SPICE netlist input"
            rows={12}
            value={spiceInput}
            placeholder={'V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 159.15n\n.ac dec 40 10 100k\n.end'}
            onChange={(event) => {
              setSpiceInput(event.target.value);
              setCopied(false);
              setClipboardError('');
            }}
          />

          {clipboardError && (
            <div className="spice-dialog-alert spice-dialog-warning" role="status">
              {clipboardError}
            </div>
          )}

          {conversion.error && (
            <div className="spice-dialog-alert spice-dialog-error" role="alert">
              <strong>Conversion paused — fix reported SPICE lines:</strong>
              <ul>
                {(conversion.diagnostics.length > 0 ? conversion.diagnostics : []).slice(0, 6).map((diag, index) => (
                  <li key={index}>{importDiagnosticLabel(diag)}</li>
                ))}
                {conversion.diagnostics.length === 0 && <li>{conversion.error}</li>}
              </ul>
            </div>
          )}

          {!conversion.error && conversion.diagnostics.length > 0 && (
            <div className="spice-dialog-alert spice-dialog-notes" role="status">
              <strong>Import notes ({conversion.diagnostics.length})</strong>
              <ul>
                {conversion.diagnostics.slice(0, 4).map((diag, index) => (
                  <li key={index}>{importDiagnosticLabel(diag)}</li>
                ))}
              </ul>
            </div>
          )}
        </section>

        <section className="spice-dialog-pane" aria-label="Converted Kessetsu preview pane">
          <div className="spice-pane-header">
            <strong>2. Verified .kess Preview</strong>
            <div className="spice-mode-switch" role="group" aria-label="Preview mode">
              <button
                type="button"
                aria-pressed={applyMode === 'replace'}
                onClick={() => setApplyMode('replace')}
              >
                Standalone / Replace
              </button>
              <button
                type="button"
                aria-pressed={applyMode === 'insert'}
                onClick={() => setApplyMode('insert')}
              >
                Merge into current circuit
              </button>
            </div>
          </div>

          {conversion.report ? (
            <>
              <div className="spice-preview-summary">
                <span className="spice-summary-badge">
                  <Check size={12} />
                  {conversion.report.summary.components} components · {conversion.report.summary.nets} nets · {conversion.report.summary.analyses} analyses
                </span>
                <button type="button" className="spice-copy-button" onClick={() => void copyPreview()}>
                  {copied ? <><Check size={12} /> Copied .kess</> : <><Copy size={12} /> Copy .kess</>}
                </button>
              </div>
              {applyMode === 'insert' && mergedValidation && !mergedValidation.valid && (
                <div className="spice-dialog-alert spice-dialog-error" role="alert">
                  <strong>Merge conflict with current circuit:</strong>
                  <ul>
                    {mergedValidation.errors.slice(0, 3).map((err, idx) => (
                      <li key={idx}>[{err.code}] {err.message}</li>
                    ))}
                  </ul>
                  <small>Switch to Standalone / Replace or rename conflicting components in your SPICE input.</small>
                </div>
              )}
              <pre className="spice-preview-code" data-testid="spice-preview-code">{previewSource}</pre>
              {conversion.report.names.length > 0 && (
                <details className="spice-name-mappings">
                  <summary>Identifier mappings ({conversion.report.names.length})</summary>
                  <div className="spice-mapping-chips">
                    {conversion.report.names.map((item, idx) => (
                      <span key={idx}>
                        <code>{item.original}</code> → <code>{item.kessetsu}</code> <small>({item.kind})</small>
                      </span>
                    ))}
                  </div>
                </details>
              )}
            </>
          ) : (
            <div className="spice-preview-empty">
              <ArrowRightLeft size={24} />
              <p>
                {spiceInput.trim()
                  ? 'Resolve the SPICE diagnostics on the left to preview the verified .kess circuit.'
                  : 'Paste a SPICE netlist on the left or load an example to preview its .kess translation.'}
              </p>
            </div>
          )}
        </section>
      </div>

      <div className="dialog-footer">
        <span className="dialog-status" role="status">
          {conversion.report
            ? applyMode === 'insert'
              ? mergedValidation?.valid
                ? 'Merged preview passes Kessetsu compiler & ERC checks.'
                : 'Merged preview has naming conflicts with the current circuit.'
              : 'Converted .kess source passes compiler & ERC checks.'
            : 'Waiting for a valid SPICE netlist…'}
        </span>
        <button type="button" className="dialog-secondary" onClick={() => dialogRef.current?.close()}>
          Cancel
        </button>
        <button
          type="button"
          className="dialog-secondary"
          disabled={!previewSource}
          onClick={() => void copyPreview()}
        >
          <Copy size={14} /> {copied ? 'Copied' : 'Copy .kess'}
        </button>
        <button
          type="button"
          className="dialog-primary"
          disabled={!canApply}
          onClick={applyToCircuit}
        >
          <ArrowRightLeft size={14} />
          {applyMode === 'insert' ? 'Insert into circuit' : 'Replace current circuit'}
        </button>
      </div>
    </dialog>
  );
}
