import {
  AlertTriangle,
  CheckCircle2,
  ClipboardPaste,
  Copy,
  Download,
  LoaderCircle,
  Play,
  Upload,
  X,
} from 'lucide-react';
import type { ReactNode } from 'react';
import {
  describeProposalDiagnostic,
  lineDiff,
  summarizeProposalChanges,
  type AgentProposalEnvelope,
} from '../agentProposal';
import type { CompileDiagnostic } from '../domain';
import type { BrowserEvaluation } from '../simulation/types';

export type AgentStep = 'task' | 'proposal' | 'review';

export interface AgentRecoverySectionProps {
  inReview: boolean;
  compileErrors: CompileDiagnostic[];
  proposal: AgentProposalEnvelope | null;
  error: string;
  checkedAssertions: string;
  correctionPending: boolean;
  correctionText: string;
  onSendCorrection: (download: boolean) => void;
  onSetStep: (step: AgentStep) => void;
}

export function AgentRecoverySection({
  inReview,
  compileErrors,
  proposal,
  error,
  checkedAssertions,
  correctionPending,
  correctionText,
  onSendCorrection,
  onSetStep,
}: AgentRecoverySectionProps) {
  return (
    <section className="agent-recovery" aria-label="Correct the agent reply" role="alert">
      <h3>The returned circuit needs a correction</h3>
      <p>Your editor circuit is unchanged. Send this diagnostic feedback back to your agent:</p>
      {compileErrors.length > 0 ? (
        <ul>
          {compileErrors.map((diagnostic, index) => (
            <li key={index}>
              <span>{describeProposalDiagnostic(diagnostic)}</span>
              {diagnostic.line && proposal && (
                <code>{proposal.proposed_source.split(/\r?\n/)[diagnostic.line - 1]?.slice(0, 240)}</code>
              )}
            </li>
          ))}
        </ul>
      ) : (
        <p>
          {error ||
            (checkedAssertions === 'unchecked'
              ? 'No assertions checked your requested outcome. Ask the agent to encode the measurable targets.'
              : 'Some assertions failed, errored or were skipped. The correction task includes the actual results.')}
        </p>
      )}
      {compileErrors.length > 0 && (
        <details>
          <summary>Technical diagnostics</summary>
          <pre>{compileErrors.map((item) => `${item.code}: ${item.message}`).join('\n\n')}</pre>
        </details>
      )}
      <div className="agent-actions">
        <button
          type="button"
          className="agent-primary"
          disabled={correctionPending}
          onClick={() => onSendCorrection(false)}
        >
          <Copy size={14} /> {correctionPending ? 'Preparing feedback…' : 'Copy correction task'}
        </button>
        <button
          type="button"
          disabled={correctionPending}
          onClick={() => onSendCorrection(true)}
        >
          <Download size={14} /> Download correction file
        </button>
        {inReview && (
          <button type="button" onClick={() => onSetStep('proposal')}>
            Edit reply JSON
          </button>
        )}
      </div>
      {inReview && <small className="agent-recovery-note">Failure details included</small>}
      {correctionText && (
        <details>
          <summary>Copy correction text manually</summary>
          <textarea
            aria-label="Correction task JSON"
            readOnly
            rows={5}
            value={correctionText}
            onFocus={(event) => event.currentTarget.select()}
          />
        </details>
      )}
    </section>
  );
}

export interface AgentTaskStepProps {
  sourceHash: string;
  sourceClean: boolean;
  requirements: string;
  taskText: string;
  onRequirementsChange: (value: string) => void;
}

export function AgentTaskStep({
  sourceHash,
  sourceClean,
  requirements,
  taskText,
  onRequirementsChange,
}: AgentTaskStepProps) {
  return (
    <div className="workflow-step-pane">
      <div className="agent-context-strip">
        <div className="agent-context-item">
          <span className="agent-context-label">Circuit revision</span>
          <code>sha256:{sourceHash ? `${sourceHash.slice(0, 12)}…` : '…'}</code>
        </div>
        <div className="agent-context-item">
          <span className="agent-context-label">Compile status</span>
          <span>{sourceClean ? 'Compiled cleanly' : 'Diagnostics present'}</span>
        </div>
        <div className="agent-context-item">
          <span className="agent-context-label">Simulation context</span>
          <span>Ready to evaluate locally</span>
        </div>
      </div>

      <label className="agent-label">
        <span>Circuit requirements</span>
        <textarea
          rows={5}
          value={requirements}
          onChange={(event) => onRequirementsChange(event.target.value)}
          placeholder="Describe the circuit change or target outcome (optional — leave blank to use assertions already in the circuit)…"
        />
      </label>

      <details className="agent-advanced">
        <summary>Inspect task JSON</summary>
        <p>Includes current source, SHA-256 revision hash, diagnostics, simulation summary, and reply schema.</p>
        <pre>{taskText}</pre>
      </details>
    </div>
  );
}

export interface AgentProposalStepProps {
  proposalText: string;
  needsCorrection: boolean;
  proposal: AgentProposalEnvelope | null;
  error: string;
  onPasteAndReview: () => void;
  onOpenFileClick: () => void;
  onProposalTextChange: (text: string) => void;
  recoverySection?: ReactNode;
}

export function AgentProposalStep({
  proposalText,
  needsCorrection,
  proposal,
  error,
  onPasteAndReview,
  onOpenFileClick,
  onProposalTextChange,
  recoverySection,
}: AgentProposalStepProps) {
  return (
    <div className="workflow-step-pane">
      <div className="agent-step-toolbar">
        <button type="button" onClick={onPasteAndReview}>
          <ClipboardPaste size={14} /> Paste &amp; check
        </button>
        <button type="button" onClick={onOpenFileClick}>
          <Upload size={14} /> Open reply file…
        </button>
      </div>

      <label className="agent-label">
        <span>Agent reply JSON</span>
        <textarea
          rows={8}
          value={proposalText}
          onChange={(event) => onProposalTextChange(event.target.value)}
          placeholder="Paste the JSON reply from your AI agent…"
        />
      </label>

      {needsCorrection && !proposal && recoverySection}
      {error && !needsCorrection && (
        <p className="agent-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

export interface AgentReviewStepProps {
  proposal: AgentProposalEnvelope;
  connectivityVerified: boolean;
  compileErrors: CompileDiagnostic[];
  simulationCard: {
    tone: string;
    text: string;
    icon: 'loading' | 'error' | 'warning' | 'pass' | 'idle';
  };
  compileWarnings: CompileDiagnostic[];
  changedLines: number;
  changeSummary: ReturnType<typeof summarizeProposalChanges>;
  diff: ReturnType<typeof lineDiff>;
  verification: BrowserEvaluation | null;
  verificationPassed: boolean;
  checkedAssertions: string;
  assertionSummary: BrowserEvaluation['assertions']['summary'] | undefined;
  confirmApply: boolean;
  canApply: boolean;
  onCancelConfirmApply: () => void;
  onApply: () => void;
  recoverySection?: ReactNode;
}

export function AgentReviewStep({
  proposal,
  connectivityVerified,
  compileErrors,
  simulationCard,
  compileWarnings,
  changedLines,
  changeSummary,
  diff,
  verification,
  verificationPassed,
  checkedAssertions,
  assertionSummary,
  confirmApply,
  canApply,
  onCancelConfirmApply,
  onApply,
  recoverySection,
}: AgentReviewStepProps) {
  return (
    <div className="workflow-step-pane is-wide">
      <div className="agent-check-grid">
        <div className="agent-check-card agent-check-pass">
          <CheckCircle2 size={16} />
          <div>
            <strong>Correct circuit revision</strong>
            <span>Matches the source you sent.</span>
          </div>
        </div>
        <div
          className={
            connectivityVerified && compileErrors.length === 0
              ? 'agent-check-card agent-check-pass'
              : 'agent-check-card agent-check-fail'
          }
        >
          {connectivityVerified && compileErrors.length === 0 ? <CheckCircle2 size={16} /> : <X size={16} />}
          <div>
            <strong>Core compile and connectivity</strong>
            <span>
              {connectivityVerified && compileErrors.length === 0
                ? 'Compiles cleanly and schematic connects.'
                : 'Rejected by Core compiler or ERC.'}
            </span>
          </div>
        </div>
        <div className={`agent-check-card ${simulationCard.tone}`}>
          {simulationCard.icon === 'pass' ? (
            <CheckCircle2 size={16} />
          ) : simulationCard.icon === 'loading' ? (
            <LoaderCircle className="agent-spin" size={16} />
          ) : simulationCard.icon === 'idle' ? (
            <Play size={16} />
          ) : simulationCard.icon === 'error' ? (
            <X size={16} />
          ) : (
            <AlertTriangle size={16} />
          )}
          <div>
            <strong>Local simulation and assertions</strong>
            <span>{simulationCard.text}</span>
          </div>
        </div>
      </div>

      {recoverySection}

      {compileWarnings.length > 0 && (
        <p className="agent-warning">
          {compileWarnings.length} compile warning{compileWarnings.length === 1 ? '' : 's'}:{' '}
          {compileWarnings.slice(0, 2).map((item) => item.message).join(' ')}
        </p>
      )}
      {proposal.summary && (
        <p className="agent-summary-line">
          <strong>
            Agent note <em>(unverified)</em>:
          </strong>{' '}
          {proposal.summary}
        </p>
      )}

      <details className="agent-review-detail" open>
        <summary>
          <span>Source changes</span>
          <small>{changedLines} changed lines</small>
        </summary>
        <div className="agent-change-summary" aria-label="High-impact source changes">
          <span>
            Assertions{' '}
            <b>
              +{changeSummary.assertions.added} / −{changeSummary.assertions.removed}
            </b>
          </span>
          <span>
            Analyses{' '}
            <b>
              +{changeSummary.analyses.added} / −{changeSummary.analyses.removed}
            </b>
          </span>
          <span>
            Sources{' '}
            <b>
              +{changeSummary.sources.added} / −{changeSummary.sources.removed}
            </b>
          </span>
          <span>
            Components{' '}
            <b>
              +{changeSummary.components.added} / −{changeSummary.components.removed}
            </b>
          </span>
          <span>
            Connections{' '}
            <b>
              +{changeSummary.connections.added} / −{changeSummary.connections.removed}
            </b>
          </span>
        </div>
        {(changeSummary.assertions.removed > 0 ||
          changeSummary.sources.added + changeSummary.sources.removed > 0) && (
          <p className="agent-impact-warning">
            <AlertTriangle size={14} /> Review removed assertions and source or supply changes before applying.
          </p>
        )}
        <div className="agent-diff" role="region" aria-label="Proposed source diff" tabIndex={0}>
          {diff.map((line, index) => (
            <div className={`agent-diff-${line.kind}`} key={`${index}-${line.kind}`}>
              <span>{line.oldLine ?? ''}</span>
              <span>{line.newLine ?? ''}</span>
              <b>{line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' '}</b>
              <code>{line.text || ' '}</code>
            </div>
          ))}
        </div>
      </details>

      {verification && (
        <div
          className={`agent-verification ${verificationPassed ? '' : 'agent-verification-warning'}`}
          data-testid="agent-verification"
        >
          {verificationPassed ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
          <div>
            <strong>
              {verificationPassed
                ? 'Simulation completed and all encoded assertions passed'
                : checkedAssertions === 'unchecked'
                  ? 'Simulation completed without requirement checks'
                  : 'Simulation completed with unresolved assertion results'}
            </strong>
            <span>
              {verification.simulation.datasets.length} analyses · {assertionSummary?.passed}/
              {assertionSummary?.total} passed · {assertionSummary?.failed} failed · {assertionSummary?.errors}{' '}
              errors · {assertionSummary?.skipped} skipped. Only encoded assertions were checked.
            </span>
          </div>
        </div>
      )}

      {confirmApply && canApply && !verificationPassed && (
        <div className="agent-apply-confirm" role="alert">
          <AlertTriangle size={17} />
          <div>
            <strong>Apply an unverified outcome?</strong>
            <span>
              The simulation ran, but the requested outcome was not fully checked or satisfied. Review the diff before
              changing the editor.
            </span>
          </div>
          <button type="button" onClick={onCancelConfirmApply}>
            Cancel
          </button>
          <button
            type="button"
            className="agent-confirm-apply"
            onClick={onApply}
          >
            Apply unchecked changes
          </button>
        </div>
      )}
    </div>
  );
}
