import { X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { import_spice_netlist, spice_import_schema_version } from 'kessetsu-core';
import type { SpiceImportDiagnostic, SpiceImportReport } from '../domain';
import { assertValidSpiceReport, importDiagnosticLabel, SpiceImportError } from '../spiceImport';

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

interface Props {
  open: boolean;
  onClose: () => void;
  onChooseSpiceFile: () => void;
  onApply: (nextSource: string, componentCount: number) => void;
}

export function SpiceConverterDialog({
  open,
  onClose,
  onChooseSpiceFile,
  onApply,
}: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [spiceInput, setSpiceInput] = useState(SAMPLE_RC_SPICE);
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

  const previewSource = conversion.report?.kess_source ?? '';

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
      setClipboardError('Could not copy automatically. Select the output text and press Ctrl+C.');
    }
  };

  const openInEditor = () => {
    if (!previewSource || !conversion.report) return;
    onApply(previewSource, conversion.report.summary.components);
    dialogRef.current?.close();
  };

  return (
    <dialog
      ref={dialogRef}
      className="app-dialog spice-converter-dialog"
      aria-labelledby="spice-converter-title"
      onClose={onClose}
    >
      <header>
        <div>
          <h2 id="spice-converter-title">Convert SPICE Netlist</h2>
          <p>Paste an LTspice or Ngspice netlist on the left to generate verified Kessetsu (.kess) source on the right.</p>
        </div>
        <button type="button" aria-label="Close SPICE converter" onClick={() => dialogRef.current?.close()}>
          <X size={18} />
        </button>
      </header>

      <div className="spice-dialog-grid">
        <section className="spice-dialog-pane" aria-label="SPICE netlist input pane">
          <div className="spice-pane-header">
            <strong>SPICE Netlist</strong>
            <div className="spice-pane-actions">
              <button type="button" onClick={() => void pasteFromClipboard()}>
                Paste
              </button>
              <button type="button" onClick={() => { setSpiceInput(SAMPLE_RC_SPICE); setClipboardError(''); }}>
                RC example
              </button>
              <button type="button" onClick={() => { setSpiceInput(SAMPLE_OPAMP_SPICE); setClipboardError(''); }}>
                Op-Amp example
              </button>
              <button
                type="button"
                onClick={() => {
                  dialogRef.current?.close();
                  onChooseSpiceFile();
                }}
              >
                Open file…
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
              <strong>Fix reported SPICE lines:</strong>
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
            <strong>Kessetsu (.kess) Output</strong>
            {conversion.report && (
              <span className="spice-summary-badge">
                {conversion.report.summary.components} components · {conversion.report.summary.nets} nets · {conversion.report.summary.analyses} analyses
              </span>
            )}
          </div>

          {conversion.report ? (
            <pre className="spice-preview-code" data-testid="spice-preview-code">{previewSource}</pre>
          ) : (
            <div className="spice-preview-empty">
              <p>
                {spiceInput.trim()
                  ? 'Resolve the SPICE diagnostics on the left to generate the .kess output.'
                  : 'Paste a SPICE netlist on the left or choose an example.'}
              </p>
            </div>
          )}
        </section>
      </div>

      <div className="dialog-footer">
        <span className="dialog-status" role="status">
          {conversion.report
            ? 'Converted .kess source passes compiler & ERC checks.'
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
          {copied ? 'Copied' : 'Copy .kess'}
        </button>
        <button
          type="button"
          className="dialog-primary"
          disabled={!previewSource}
          onClick={openInEditor}
        >
          Open in Editor
        </button>
      </div>
    </dialog>
  );
}

