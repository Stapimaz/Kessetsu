import {
  AlertTriangle,
  CheckCircle2,
  ClipboardPaste,
  Copy,
  Download,
  LoaderCircle,
  Play,
  RotateCcw,
  Square,
  Upload,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import {
  compile_kessetsu_with_resources,
  compile_schema_version,
  evaluate_browser_simulation_with_resources,
  import_spice_netlist,
  prepare_browser_simulation_with_resources,
  spice_import_schema_version,
} from 'kessetsu-core';
import {
  assertionOutcome,
  createAgentCorrectionTask,
  createAgentTask,
  describeProposalDiagnostic,
  lineDiff,
  MAX_AGENT_PROPOSAL_BYTES,
  parseAgentProposal,
  sha256Text,
  summarizeProposalChanges,
  type AgentProposalEnvelope,
} from '../agentProposal';
import { downloadTextFile, sanitizeFileStem } from '../document';
import type { CompileReport, SpiceImportReport } from '../domain';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';
import { assertValidSpiceReport, looksLikeSpiceNetlist } from '../spiceImport';
import { WorkflowDialog, type WorkflowStep } from './WorkflowDialog';
import './AgentProposalDialog.css';

interface Props {
  open: boolean;
  source: string;
  name: string;
  productVersion: string;
  resources: Record<string, number[]>;
  onAccept(source: string): void;
  onClose(): void;
}

export type AgentStep = 'task' | 'proposal' | 'review';
type VerificationState = 'idle' | 'running' | 'complete' | 'failed' | 'cancelled';
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const docsHref = (path: string) => `${import.meta.env.BASE_URL}docs/${path}`;

function compileCandidate(source: string, resources: Record<string, number[]>): CompileReport {
  const report = compile_kessetsu_with_resources(source, resources) as CompileReport;
  if (report.schema_version !== compile_schema_version()) {
    throw new Error(`Unsupported compile report: ${report.schema_version}`);
  }
  return report;
}

export function AgentProposalDialog({
  open,
  source,
  name,
  productVersion,
  resources,
  onAccept,
  onClose,
}: Props) {
  const fileInput = useRef<HTMLInputElement>(null);
  const runner = useRef<BrowserSimulationRunner | null>(null);
  const sourceSnapshot = useRef(source);
  // Workspace model bindings are replaced, never mutated in place.
  const resourceSnapshot = useRef(resources);
  const latestInputs = useRef({ open, source, resources });
  latestInputs.current = { open, source, resources };
  const operationId = useRef(0);
  const taskOperationId = useRef(0);
  const correctionOperationId = useRef(0);
  const correctionPendingRef = useRef(false);
  if (!runner.current) runner.current = new BrowserSimulationRunner();

  const [step, setStep] = useState<AgentStep>('task');
  const [requirements, setRequirements] = useState('');
  const [taskCopied, setTaskCopied] = useState(false);
  const [taskText, setTaskText] = useState('');
  const [sourceHash, setSourceHash] = useState('');
  const [proposalText, setProposalText] = useState('');
  const [proposal, setProposal] = useState<AgentProposalEnvelope | null>(null);
  const [compileReport, setCompileReport] = useState<CompileReport | null>(null);
  const [verification, setVerification] = useState<BrowserEvaluation | null>(null);
  const [verificationState, setVerificationState] = useState<VerificationState>('idle');
  const [confirmApply, setConfirmApply] = useState(false);
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [correctionText, setCorrectionText] = useState('');
  const [correctionPending, setCorrectionPending] = useState(false);
  const latestReview = useRef({ verificationState, proposal, proposalText });
  latestReview.current = { verificationState, proposal, proposalText };

  const resetProposal = (nextStatus = '') => {
    const nextOperation = ++operationId.current;
    correctionOperationId.current++;
    correctionPendingRef.current = false;
    setCorrectionText('');
    setCorrectionPending(false);
    runner.current?.cancel();
    setProposal(null);
    setCompileReport(null);
    setVerification(null);
    setVerificationState('idle');
    setConfirmApply(false);
    setError('');
    setStatus(nextStatus);
    return nextOperation;
  };

  const handleReset = () => {
    resetProposal('');
    setProposalText('');
    setError('');
    setStep('task');
  };

  const operationIsCurrent = (requestId: number) =>
    requestId === operationId.current &&
    latestInputs.current.open &&
    latestInputs.current.source === source &&
    latestInputs.current.resources === resources;

  const taskIsCurrent = useCallback(
    (requestId: number) =>
      requestId === taskOperationId.current &&
      latestInputs.current.open &&
      latestInputs.current.source === source &&
      latestInputs.current.resources === resources,
    [source, resources],
  );

  const invalidatePending = () => {
    operationId.current++;
    taskOperationId.current++;
    correctionOperationId.current++;
    correctionPendingRef.current = false;
    setCorrectionText('');
    setCorrectionPending(false);
    runner.current?.cancel();
    const { verificationState, proposal, proposalText } = latestReview.current;
    if (verificationState === 'running') {
      setVerificationState('cancelled');
      setVerification(null);
      setConfirmApply(false);
      setStatus('Proposal simulation cancelled. Test again before applying.');
    } else if (!proposal) {
      setStatus(proposalText.trim() ? 'Check the returned circuit before testing or applying it.' : '');
    }
  };

  useEffect(() => {
    let active = true;
    void sha256Text(source).then((hash) => {
      if (active) setSourceHash(hash);
    });
    return () => {
      active = false;
    };
  }, [source]);

  useEffect(() => {
    const sourceChanged = sourceSnapshot.current !== source;
    const resourcesChanged = resourceSnapshot.current !== resources;
    if (sourceChanged || resourcesChanged) {
      sourceSnapshot.current = source;
      resourceSnapshot.current = resources;
      taskOperationId.current++;
      setTaskText('');
      setTaskCopied(false);
      if (sourceChanged) {
        setProposalText('');
        setStep('task');
      }
      resetProposal(
        sourceChanged
          ? 'The circuit changed. Create a new task and check a reply for this revision.'
          : 'Local model files changed. Check the returned circuit and test it again before applying.',
      );
    }
    if (!open) {
      invalidatePending();
      setStep('task');
    }
  }, [open, source, resources]);

  useEffect(() => () => {
    operationId.current++;
    taskOperationId.current++;
    runner.current?.dispose();
  }, []);

  // Compute live task text preview on task step when requirements or source change
  useEffect(() => {
    let active = true;
    const requestId = ++taskOperationId.current;
    const run = async () => {
      try {
        const effectiveReq =
          requirements.trim() || 'Preserve existing behavior and satisfy all encoded circuit assertions.';
        const task = await createAgentTask(source, name, effectiveReq, productVersion, compile_schema_version());
        if (active && taskIsCurrent(requestId)) {
          setTaskText(JSON.stringify(task, null, 2));
        }
      } catch {
        // Ignored for background preview
      }
    };
    void run();
    return () => {
      active = false;
    };
  }, [source, name, requirements, productVersion, open, taskIsCurrent]);

  const sourceReport = useMemo(() => {
    try {
      return compileCandidate(source, resources);
    } catch {
      return null;
    }
  }, [source, resources]);
  const sourceClean = Boolean(sourceReport && sourceReport.diagnostics.every((d) => d.severity !== 'error'));

  const diff = useMemo(() => (proposal ? lineDiff(source, proposal.proposed_source) : []), [proposal, source]);
  const changeSummary = useMemo(() => summarizeProposalChanges(diff), [diff]);
  const changedLines = diff.filter((line) => line.kind !== 'same').length;
  const compileErrors = compileReport?.diagnostics.filter((item) => item.severity === 'error') ?? [];
  const compileWarnings = compileReport?.diagnostics.filter((item) => item.severity === 'warning') ?? [];
  const connectivityVerified = Boolean(compileReport?.schematic?.connectivity.verified && compileReport.schematic_svg);
  const assertionSummary = verification?.assertions.summary;
  const checkedAssertions = assertionOutcome(assertionSummary);
  const needsCorrection =
    Boolean(error && proposalText.trim()) ||
    Boolean(proposal && (compileErrors.length > 0 || !connectivityVerified)) ||
    verificationState === 'failed' ||
    (verificationState === 'complete' && checkedAssertions !== 'passed');
  const inputsMatch = sourceSnapshot.current === source && resourceSnapshot.current === resources;
  const canApply = open && inputsMatch && Boolean(proposal) && verificationState === 'complete';
  const verificationPassed = canApply && checkedAssertions === 'passed';
  const simulationCard =
    verificationState === 'running'
      ? { tone: 'agent-check-pending', text: 'Running the candidate locally…', icon: 'loading' as const }
      : verificationState === 'failed'
        ? { tone: 'agent-check-fail', text: 'Simulation failed. Read the error and test again.', icon: 'error' as const }
        : verificationState === 'cancelled'
          ? {
              tone: 'agent-check-warning',
              text: 'Simulation was cancelled. Test again before applying.',
              icon: 'warning' as const,
            }
          : verificationState === 'complete' && checkedAssertions === 'passed'
            ? {
                tone: 'agent-check-pass',
                text: `${assertionSummary!.passed}/${assertionSummary!.total} encoded assertions passed.`,
                icon: 'pass' as const,
              }
            : verificationState === 'complete' && checkedAssertions === 'unchecked'
              ? {
                  tone: 'agent-check-warning',
                  text: 'Simulation completed, but no assertions checked the requested outcome.',
                  icon: 'warning' as const,
                }
              : verificationState === 'complete'
                ? {
                    tone: 'agent-check-warning',
                    text: `${assertionSummary!.passed}/${assertionSummary!.total} encoded assertions passed; failures, errors, or skipped checks remain.`,
                    icon: 'warning' as const,
                  }
                : {
                    tone: 'agent-check-pending',
                    text: 'Not run yet. Test the candidate before applying it.',
                    icon: 'idle' as const,
                  };

  const buildTask = async () => {
    setError('');
    const requestId = ++taskOperationId.current;
    try {
      const effectiveReq =
        requirements.trim() || 'Preserve existing behavior and satisfy all encoded circuit assertions.';
      const task = await createAgentTask(source, name, effectiveReq, productVersion, compile_schema_version());
      if (!taskIsCurrent(requestId)) return null;
      const serialized = JSON.stringify(task, null, 2);
      setTaskText(serialized);
      setStatus('Agent task created from this exact source revision.');
      return serialized;
    } catch (cause) {
      if (taskIsCurrent(requestId)) setError(message(cause));
      return null;
    }
  };

  const copyPrompt = async () => {
    const serialized = taskText || (await buildTask());
    const requestId = taskOperationId.current;
    if (!serialized || !latestInputs.current.open) return;
    try {
      await navigator.clipboard.writeText(serialized);
      if (taskIsCurrent(requestId)) {
        setTaskCopied(true);
        setStatus('Task copied. Paste it into the AI agent you want to use.');
      }
    } catch {
      if (taskIsCurrent(requestId)) setError('Clipboard access was blocked. Download the task JSON instead.');
    }
  };

  const downloadTask = async () => {
    const serialized = taskText || (await buildTask());
    if (!serialized) return;
    downloadTextFile(serialized, `${sanitizeFileStem(name)}.kessagent.json`);
    setStatus('Task downloaded. Give the JSON file to the AI agent you want to use.');
  };

  const sendCorrection = async (download: boolean) => {
    if (!open || !needsCorrection || !inputsMatch || !requirements.trim() || correctionPendingRef.current) return;
    correctionPendingRef.current = true;
    setCorrectionPending(true);
    const correctionId = ++correctionOperationId.current;
    const reviewId = operationId.current;
    const isCurrent = () => correctionId === correctionOperationId.current && operationIsCurrent(reviewId);
    try {
      const task = await createAgentCorrectionTask(
        source,
        name,
        requirements,
        productVersion,
        compile_schema_version(),
        proposalText,
        error || 'Local simulation did not satisfy all encoded requirements.',
        compileReport?.diagnostics ?? [],
        verification?.assertions ?? null,
      );
      if (!isCurrent()) return;
      const serialized = JSON.stringify(task, null, 2);
      setCorrectionText(serialized);
      if (download) {
        downloadTextFile(serialized, `${sanitizeFileStem(name)}.correction.kessagent.json`);
        setStatus('Correction task downloaded. Send it to your agent, then paste its new complete reply in Step 2.');
      } else {
        try {
          await navigator.clipboard.writeText(serialized);
          if (isCurrent()) {
            setStatus('Correction task copied. Send it to your agent, then paste its new complete reply in Step 2.');
          }
        } catch {
          if (isCurrent()) {
            setStatus('Clipboard access was blocked. Copy the correction text below or download its file.');
          }
        }
      }
    } catch (cause) {
      if (isCurrent()) setStatus(`Could not prepare correction task: ${message(cause)}`);
    } finally {
      if (isCurrent()) {
        correctionPendingRef.current = false;
        setCorrectionPending(false);
      }
    }
  };

  const reviewProposal = async (text = proposalText) => {
    const requestId = resetProposal('Checking the returned circuit…');
    try {
      const parsed = await parseAgentProposal(text, source);
      if (!operationIsCurrent(requestId)) return;
      let candidateProposal = parsed;
      let report = compileCandidate(candidateProposal.proposed_source, resources);
      if (
        report.diagnostics.some((item) => item.code === 'KES-P001' && item.severity === 'error') &&
        looksLikeSpiceNetlist(candidateProposal.proposed_source)
      ) {
        try {
          const rawReport = import_spice_netlist(candidateProposal.proposed_source) as SpiceImportReport;
          const imported = assertValidSpiceReport(rawReport, spice_import_schema_version());
          if (imported.kess_source) {
            candidateProposal = { ...parsed, proposed_source: imported.kess_source };
            report = compileCandidate(candidateProposal.proposed_source, resources);
          }
        } catch {
          // Keep original compile report if SPICE import fails.
        }
      }
      if (!operationIsCurrent(requestId)) return;
      setProposal(candidateProposal);
      setCompileReport(report);
      setStep('review');
      const errors = report.diagnostics.filter((item) => item.severity === 'error');
      const verified = Boolean(report.schematic?.connectivity.verified && report.schematic_svg);
      if (errors.length || !verified) {
        setError(
          errors
            .slice(0, 4)
            .map((item) => `${item.code}${item.line ? ` line ${item.line}` : ''}: ${item.message}`)
            .join(' ') || 'Schematic connectivity was not verified.',
        );
        setStatus('Proposal loaded, but Core rejected it. The editor source is unchanged.');
      } else {
        setStatus('The returned circuit is valid and still separate from your editor. Test it before applying.');
      }
    } catch (cause) {
      if (operationIsCurrent(requestId)) {
        setError(message(cause));
        setStatus('The reply could not be checked. The editor source is unchanged.');
        setStep('proposal');
      }
    }
  };

  const runVerification = async () => {
    if (!open || !inputsMatch || !proposal || !connectivityVerified || compileErrors.length) return;
    const candidate = proposal;
    const requestId = ++operationId.current;
    correctionOperationId.current++;
    correctionPendingRef.current = false;
    setCorrectionPending(false);
    setCorrectionText('');
    setError('');
    setVerification(null);
    setVerificationState('running');
    setConfirmApply(false);
    setStatus('Preparing proposal simulation…');
    try {
      const plan = prepare_browser_simulation_with_resources(
        candidate.proposed_source,
        resources,
      ) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) {
        throw new Error('The proposal has no simulation command. Ask the agent to include a verifiable analysis.');
      }
      const simulation = await runner.current!.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => {
          if (operationIsCurrent(requestId)) setStatus(progress.message);
        },
      });
      if (!operationIsCurrent(requestId)) return;
      const evaluation = evaluate_browser_simulation_with_resources(
        candidate.proposed_source,
        simulation,
        resources,
      ) as BrowserEvaluation;
      if (!operationIsCurrent(requestId)) return;
      setVerification(evaluation);
      setVerificationState('complete');
      const outcome = assertionOutcome(evaluation.assertions.summary);
      setStatus(
        outcome === 'passed'
          ? `${evaluation.simulation.datasets.length} analyses completed · all ${evaluation.assertions.summary.total} encoded assertions passed.`
          : outcome === 'unchecked'
            ? `${evaluation.simulation.datasets.length} analyses completed · no assertions were checked.`
            : `${evaluation.simulation.datasets.length} analyses completed · some encoded assertions were not satisfied.`,
      );
    } catch (cause) {
      if (!operationIsCurrent(requestId)) return;
      if (cause instanceof SimulationCancelledError) {
        setVerificationState('cancelled');
        setStatus('Proposal simulation cancelled. The editor source is unchanged.');
      } else {
        setVerificationState('failed');
        setError(message(cause));
        setStatus('Proposal simulation failed. The editor source is unchanged.');
      }
    }
  };

  const cancelVerification = () => {
    operationId.current++;
    runner.current?.cancel();
    setVerificationState('cancelled');
    setVerification(null);
    setConfirmApply(false);
    setStatus('Proposal simulation cancelled. The editor source is unchanged.');
  };

  const pasteAndReview = async () => {
    try {
      const clip = await navigator.clipboard.readText();
      if (!clip.trim()) {
        setStatus('Clipboard is empty. Copy the JSON reply from your AI agent first.');
        return;
      }
      setProposalText(clip);
      await reviewProposal(clip);
    } catch {
      setError('Clipboard access was blocked. Paste the reply below with Ctrl+V, then select Check returned circuit.');
    }
  };

  const close = () => {
    invalidatePending();
    setStep('task');
    onClose();
  };

  const steps: WorkflowStep<AgentStep>[] = [
    {
      id: 'task',
      number: 1,
      label: 'Task',
      done: taskCopied || Boolean(requirements.trim()),
    },
    {
      id: 'proposal',
      number: 2,
      label: 'Proposal',
      done: Boolean(proposal) && !needsCorrection,
    },
    {
      id: 'review',
      number: 3,
      label: 'Review',
      disabled: !proposal,
      done: verificationPassed,
    },
  ];

  const renderRecovery = (inReview: boolean) => (
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
          disabled={!requirements.trim() || correctionPending}
          onClick={() => void sendCorrection(false)}
        >
          <Copy size={14} /> {correctionPending ? 'Preparing feedback…' : 'Copy correction task'}
        </button>
        <button
          type="button"
          disabled={!requirements.trim() || correctionPending}
          onClick={() => void sendCorrection(true)}
        >
          <Download size={14} /> Download correction file
        </button>
        {inReview && (
          <button type="button" onClick={() => setStep('proposal')}>
            Edit reply JSON
          </button>
        )}
      </div>
      {inReview && <small className="agent-recovery-note">Failure details included</small>}
      {!requirements.trim() && (
        <p>Add your electrical target above and send a task first so Kessetsu can package the correction context.</p>
      )}
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

  return (
    <WorkflowDialog<AgentStep>
      open={open}
      onClose={close}
      ariaLabel="Work with an AI agent"
      closeLabel="Close AI agent workflow"
      className="agent-workflow-dialog"
      title="Work with an AI agent"
      subtitle="Copy a structured circuit task for your AI tool, paste back its JSON reply, and verify locally before applying. Kessetsu never contacts an AI service."
      headerExtras={
        <p className="dialog-help-link">
          <a
            href={docsHref('guides/web-editor/#review-a-proposal-from-an-external-agent')}
            target="_blank"
            rel="noreferrer"
          >
            How the agent handoff works
          </a>
          {' · '}
          <a href={docsHref('guides/agent-workflow/')} target="_blank" rel="noreferrer">
            Local CLI / MCP setup (<code>kess mcp</code>)
          </a>
        </p>
      }
      stepperLabel="AI agent workflow steps"
      steps={steps}
      activeStep={step}
      onStepSelect={(nextStep) => setStep(nextStep)}
      stepperActions={
        proposal || proposalText.trim() || error ? (
          <button type="button" onClick={handleReset}>
            <RotateCcw size={12} />
            <span>Start over</span>
          </button>
        ) : undefined
      }
      status={status}
      footerActions={
        step === 'task' ? (
          <>
            <button type="button" onClick={downloadTask}>
              <Download size={14} /> Download task file
            </button>
            <button type="button" onClick={() => setStep('proposal')}>
              Next: Paste reply
            </button>
            <button type="button" className="primary" onClick={() => void copyPrompt()}>
              <Copy size={14} /> Copy task for AI
            </button>
          </>
        ) : step === 'proposal' ? (
          <>
            <button type="button" onClick={() => setStep('task')}>
              Back
            </button>
            <button
              type="button"
              className="primary"
              disabled={!proposalText.trim()}
              onClick={() => void reviewProposal()}
            >
              Check returned circuit
            </button>
          </>
        ) : proposal ? (
          <>
            <button type="button" onClick={() => setStep('proposal')}>
              Back
            </button>
            {verificationState === 'running' ? (
              <button type="button" onClick={cancelVerification}>
                <Square size={14} /> Stop test
              </button>
            ) : (
              <button
                type="button"
                className={!connectivityVerified || compileErrors.length > 0 ? '' : !verificationPassed ? 'primary' : ''}
                disabled={!inputsMatch || !connectivityVerified || compileErrors.length > 0}
                onClick={() => void runVerification()}
              >
                <Play size={14} />{' '}
                {verificationState === 'complete' ||
                verificationState === 'failed' ||
                verificationState === 'cancelled'
                  ? 'Test again'
                  : 'Test proposed circuit'}
              </button>
            )}
            <button
              type="button"
              className={`agent-accept ${verificationPassed ? 'primary' : ''}`.trim()}
              disabled={!canApply}
              onClick={() => {
                if (!proposal || !canApply) return;
                if (verificationPassed) {
                  onAccept(proposal.proposed_source);
                  close();
                } else {
                  setConfirmApply(true);
                }
              }}
            >
              {verificationPassed
                ? 'Apply tested proposal'
                : verificationState === 'complete'
                  ? 'Apply anyway…'
                  : 'Apply to editor'}
            </button>
          </>
        ) : undefined
      }
    >
      <input
        ref={fileInput}
        type="file"
        className="sr-only"
        accept=".json,application/json"
        aria-label="Open agent proposal JSON"
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (!file) return;
          const fileRequestId = resetProposal('Reading the reply file…');
          setProposalText('');
          if (file.size > MAX_AGENT_PROPOSAL_BYTES) {
            setError('Proposal file is too large');
            return;
          }
          void file
            .text()
            .then((text) => {
              if (!operationIsCurrent(fileRequestId)) return;
              setProposalText(text);
              return reviewProposal(text);
            })
            .catch((cause) => {
              if (operationIsCurrent(fileRequestId)) setError(message(cause));
            });
        }}
      />

      {step === 'task' && (
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
              onChange={(event) => {
                taskOperationId.current++;
                setRequirements(event.target.value);
                setTaskCopied(false);
                setTaskText('');
              }}
              placeholder="Describe the circuit change or target outcome (optional — leave blank to use assertions already in the circuit)…"
            />
          </label>

          <details className="agent-advanced">
            <summary>Inspect task JSON</summary>
            <p>Includes current source, SHA-256 revision hash, diagnostics, simulation summary, and reply schema.</p>
            <pre>{taskText}</pre>
          </details>
        </div>
      )}

      {step === 'proposal' && (
        <div className="workflow-step-pane">
          <div className="agent-step-toolbar">
            <button type="button" onClick={() => void pasteAndReview()}>
              <ClipboardPaste size={14} /> Paste &amp; check
            </button>
            <button type="button" onClick={() => fileInput.current?.click()}>
              <Upload size={14} /> Open reply file…
            </button>
          </div>

          <label className="agent-label">
            <span>Agent reply JSON</span>
            <textarea
              rows={8}
              value={proposalText}
              onChange={(event) => {
                const text = event.target.value;
                setProposalText(text);
                setConfirmApply(false);
                resetProposal(text.trim() ? 'Reply changed. Check this version before testing or applying it.' : '');
              }}
              placeholder="Paste the JSON reply from your AI agent…"
            />
          </label>

          {needsCorrection && !proposal && renderRecovery(false)}
          {error && !needsCorrection && (
            <p className="agent-error" role="alert">
              {error}
            </p>
          )}
        </div>
      )}

      {step === 'review' && proposal && (
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

          {needsCorrection && renderRecovery(true)}

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
              <button type="button" onClick={() => setConfirmApply(false)}>
                Cancel
              </button>
              <button
                type="button"
                className="agent-confirm-apply"
                onClick={() => {
                  if (!canApply) return;
                  onAccept(proposal.proposed_source);
                  close();
                }}
              >
                Apply unchecked changes
              </button>
            </div>
          )}
        </div>
      )}
    </WorkflowDialog>
  );
}
