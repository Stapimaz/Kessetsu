import { AlertTriangle, CheckCircle2, Clipboard, Download, FileUp, LoaderCircle, Play, Square, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  compile_kessetsu_with_resources,
  compile_schema_version,
  evaluate_browser_simulation_with_resources,
  prepare_browser_simulation_with_resources,
} from 'kessetsu-core';
import {
  createAgentTask,
  assertionOutcome,
  lineDiff,
  MAX_AGENT_PROPOSAL_BYTES,
  parseAgentProposal,
  summarizeProposalChanges,
  type AgentProposalEnvelope,
} from '../agentProposal';
import { downloadTextFile, sanitizeFileStem } from '../document';
import type { CompileReport } from '../domain';
import { BrowserSimulationRunner, SimulationCancelledError } from '../simulation/browserRunner';
import type { BrowserEvaluation, BrowserSimulationPlan } from '../simulation/types';
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

type VerificationState = 'idle' | 'running' | 'complete' | 'failed' | 'cancelled';
const message = (error: unknown) => error instanceof Error ? error.message : String(error);

function compileCandidate(source: string, resources: Record<string, number[]>): CompileReport {
  const report = compile_kessetsu_with_resources(source, resources) as CompileReport;
  if (report.schema_version !== compile_schema_version()) throw new Error(`Unsupported compile report: ${report.schema_version}`);
  return report;
}

export function AgentProposalDialog({ open, source, name, productVersion, resources, onAccept, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const runner = useRef<BrowserSimulationRunner | null>(null);
  const sourceSnapshot = useRef(source);
  // Workspace model bindings are replaced, never mutated in place.
  const resourceSnapshot = useRef(resources);
  const latestInputs = useRef({ open, source, resources });
  latestInputs.current = { open, source, resources };
  const operationId = useRef(0);
  const taskOperationId = useRef(0);
  if (!runner.current) runner.current = new BrowserSimulationRunner();
  const [requirements, setRequirements] = useState('');
  const [taskText, setTaskText] = useState('');
  const [proposalText, setProposalText] = useState('');
  const [proposal, setProposal] = useState<AgentProposalEnvelope | null>(null);
  const [compileReport, setCompileReport] = useState<CompileReport | null>(null);
  const [verification, setVerification] = useState<BrowserEvaluation | null>(null);
  const [verificationState, setVerificationState] = useState<VerificationState>('idle');
  const [confirmApply, setConfirmApply] = useState(false);
  const [status, setStatus] = useState('Nothing leaves this browser automatically.');
  const [error, setError] = useState('');
  const latestReview = useRef({ verificationState, proposal, proposalText });
  latestReview.current = { verificationState, proposal, proposalText };

  const resetProposal = (nextStatus = 'Nothing leaves this browser automatically.') => {
    const nextOperation = ++operationId.current;
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

  const operationIsCurrent = (requestId: number) => requestId === operationId.current
    && latestInputs.current.open && latestInputs.current.source === source
    && latestInputs.current.resources === resources;
  const taskIsCurrent = (requestId: number) => requestId === taskOperationId.current
    && latestInputs.current.open && latestInputs.current.source === source
    && latestInputs.current.resources === resources;

  const invalidatePending = () => {
    operationId.current++;
    taskOperationId.current++;
    runner.current?.cancel();
    const { verificationState, proposal, proposalText } = latestReview.current;
    if (verificationState === 'running') {
      setVerificationState('cancelled');
      setVerification(null);
      setConfirmApply(false);
      setStatus('Proposal simulation cancelled. Test again before applying.');
    } else if (!proposal) {
      setStatus(proposalText.trim() ? 'Check the returned circuit before testing or applying it.'
        : 'Nothing leaves this browser automatically.');
    }
  };

  useEffect(() => {
    const sourceChanged = sourceSnapshot.current !== source;
    const resourcesChanged = resourceSnapshot.current !== resources;
    if (sourceChanged || resourcesChanged) {
      sourceSnapshot.current = source;
      resourceSnapshot.current = resources;
      taskOperationId.current++;
      setTaskText('');
      if (sourceChanged) setProposalText('');
      resetProposal(sourceChanged
        ? 'The circuit changed. Create a new task and check a reply for this revision.'
        : 'Local model files changed. Check the returned circuit and test it again before applying.');
    }
    if (open && !dialog.current?.open) dialog.current?.showModal();
    if (!open) {
      invalidatePending();
      if (dialog.current?.open) dialog.current.close();
    }
  }, [open, source, resources]);
  useEffect(() => () => {
    operationId.current++;
    taskOperationId.current++;
    runner.current?.dispose();
  }, []);

  const diff = useMemo(() => proposal ? lineDiff(source, proposal.proposed_source) : [], [proposal, source]);
  const changeSummary = useMemo(() => summarizeProposalChanges(diff), [diff]);
  const changedLines = diff.filter((line) => line.kind !== 'same').length;
  const compileErrors = compileReport?.diagnostics.filter((item) => item.severity === 'error') ?? [];
  const compileWarnings = compileReport?.diagnostics.filter((item) => item.severity === 'warning') ?? [];
  const connectivityVerified = Boolean(compileReport?.schematic?.connectivity.verified && compileReport.schematic_svg);
  const assertionSummary = verification?.assertions.summary;
  const checkedAssertions = assertionOutcome(assertionSummary);
  const inputsMatch = sourceSnapshot.current === source && resourceSnapshot.current === resources;
  const canApply = open && inputsMatch && Boolean(proposal) && verificationState === 'complete';
  const verificationPassed = canApply && checkedAssertions === 'passed';
  const simulationCard = verificationState === 'running'
    ? { tone: 'agent-check-pending', text: 'Running the candidate locally…', icon: 'loading' as const }
    : verificationState === 'failed'
      ? { tone: 'agent-check-fail', text: 'Simulation failed. Read the error and test again.', icon: 'error' as const }
      : verificationState === 'cancelled'
        ? { tone: 'agent-check-warning', text: 'Simulation was cancelled. Test again before applying.', icon: 'warning' as const }
        : verificationState === 'complete' && checkedAssertions === 'passed'
          ? { tone: 'agent-check-pass', text: `${assertionSummary!.passed}/${assertionSummary!.total} encoded assertions passed.`, icon: 'pass' as const }
          : verificationState === 'complete' && checkedAssertions === 'unchecked'
            ? { tone: 'agent-check-warning', text: 'Simulation completed, but no assertions checked the requested outcome.', icon: 'warning' as const }
            : verificationState === 'complete'
              ? { tone: 'agent-check-warning', text: `${assertionSummary!.passed}/${assertionSummary!.total} encoded assertions passed; failures, errors, or skipped checks remain.`, icon: 'warning' as const }
              : { tone: 'agent-check-pending', text: 'Not run yet. Test the candidate before applying it.', icon: 'idle' as const };

  const buildTask = async () => {
    setError('');
    const requestId = ++taskOperationId.current;
    try {
      const task = await createAgentTask(source, name, requirements, productVersion, compile_schema_version());
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

  const copyTask = async () => {
    const serialized = taskText || await buildTask();
    const requestId = taskOperationId.current;
    if (!serialized || !latestInputs.current.open) return;
    try {
      await navigator.clipboard.writeText(serialized);
      if (taskIsCurrent(requestId)) setStatus('Task copied. Paste it into the AI agent you want to use.');
    } catch {
      if (taskIsCurrent(requestId)) setError('Clipboard access was blocked. Download the task JSON instead.');
    }
  };

  const downloadTask = async () => {
    const serialized = taskText || await buildTask();
    if (!serialized) return;
    downloadTextFile(serialized, `${sanitizeFileStem(name)}.kessagent.json`);
    setStatus('Task downloaded. Give the JSON file to the AI agent you want to use.');
  };

  const reviewProposal = async (text = proposalText) => {
    const requestId = resetProposal('Checking the returned circuit…');
    try {
      const parsed = await parseAgentProposal(text, source);
      if (!operationIsCurrent(requestId)) return;
      const report = compileCandidate(parsed.proposed_source, resources);
      if (!operationIsCurrent(requestId)) return;
      setProposal(parsed);
      setCompileReport(report);
      const errors = report.diagnostics.filter((item) => item.severity === 'error');
      const verified = Boolean(report.schematic?.connectivity.verified && report.schematic_svg);
      if (errors.length || !verified) {
        setError(errors.slice(0, 4).map((item) => `${item.code}${item.line ? ` line ${item.line}` : ''}: ${item.message}`).join(' ') || 'Schematic connectivity was not verified.');
        setStatus('Proposal loaded, but Core rejected it. The editor source is unchanged.');
      } else {
        setStatus('The returned circuit is valid and still separate from your editor. Test it before applying.');
      }
    } catch (cause) {
      if (operationIsCurrent(requestId)) {
        setError(message(cause));
        setStatus('The reply could not be checked. The editor source is unchanged.');
      }
    }
  };

  const runVerification = async () => {
    if (!open || !inputsMatch || !proposal || !connectivityVerified || compileErrors.length) return;
    const candidate = proposal;
    const requestId = ++operationId.current;
    setError('');
    setVerification(null);
    setVerificationState('running');
    setConfirmApply(false);
    setStatus('Preparing proposal simulation…');
    try {
      const plan = prepare_browser_simulation_with_resources(candidate.proposed_source, resources) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('The proposal has no simulation command. Ask the agent to include a verifiable analysis.');
      const simulation = await runner.current!.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => {
          if (operationIsCurrent(requestId)) setStatus(progress.message);
        },
      });
      if (!operationIsCurrent(requestId)) return;
      const evaluation = evaluate_browser_simulation_with_resources(candidate.proposed_source, simulation, resources) as BrowserEvaluation;
      if (!operationIsCurrent(requestId)) return;
      setVerification(evaluation);
      setVerificationState('complete');
      const outcome = assertionOutcome(evaluation.assertions.summary);
      setStatus(outcome === 'passed'
        ? `${evaluation.simulation.datasets.length} analyses completed · all ${evaluation.assertions.summary.total} encoded assertions passed.`
        : outcome === 'unchecked'
          ? `${evaluation.simulation.datasets.length} analyses completed · no assertions were checked.`
          : `${evaluation.simulation.datasets.length} analyses completed · some encoded assertions were not satisfied.`);
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

  const close = () => {
    invalidatePending();
    dialog.current?.close();
  };

  return <dialog ref={dialog} className="app-dialog agent-proposal-dialog" aria-labelledby="agent-proposal-title"
    onCancel={(event) => { event.preventDefault(); close(); }} onClose={() => { invalidatePending(); onClose(); }}>
    <header><div><h2 id="agent-proposal-title">Work with an AI agent</h2><p>Send the current circuit to an AI, then let Kessetsu test its returned design before you apply it.</p></div><button aria-label="Close AI agent workflow" onClick={close}><X size={18} /></button></header>

    <div className="agent-privacy-note">
      <strong>You stay in control.</strong>
      <span>Kessetsu never contacts an AI service or changes your circuit automatically. You choose the agent and move the task between apps.</span>
    </div>

    <section className="agent-step">
      <div className="agent-step-heading"><span>1</span><div><h3>Tell the agent what you need</h3><p>Write the electrical target and limits. Kessetsu includes your current circuit automatically.</p></div></div>
      <textarea rows={4} value={requirements} maxLength={12_000} placeholder="Example: Design for 2 W RMS into 8 ohm, gain near 20, and less than 2 W transistor dissipation. Include simulations and assertions for every target." onChange={(event) => {
        taskOperationId.current++;
        setRequirements(event.target.value);
        setTaskText('');
        setProposalText('');
        resetProposal(proposal || proposalText ? 'Requirements changed. Create a new task and bring back a new reply.' : undefined);
      }} />
      <div className="agent-actions"><button className="agent-primary" onClick={() => void copyTask()}><Clipboard size={14} /> Copy task for AI</button><button onClick={() => void downloadTask()}><Download size={14} /> Download task file</button></div>
      {taskText && <div className="agent-next-step"><CheckCircle2 size={16} /><div><strong>Task ready</strong><span>Paste it into ChatGPT, Gemini, Claude, Codex, or another agent. Ask it to return only the requested JSON object.</span></div></div>}
      {taskText && <details><summary>Inspect the task JSON</summary><pre>{taskText}</pre></details>}
    </section>

    <section className="agent-step">
      <div className="agent-step-heading"><span>2</span><div><h3>Bring back the agent's reply</h3><p>Paste the complete JSON response, or open the JSON file the agent created.</p></div></div>
      <textarea rows={5} value={proposalText} placeholder={`Paste the returned ${'kessetsu.agent-proposal.v1'} JSON here…`} onChange={(event) => {
        const text = event.target.value;
        setProposalText(text);
        resetProposal(text.trim() ? 'Reply changed. Check this version before testing or applying it.' : undefined);
      }} />
      <input ref={fileInput} type="file" className="sr-only" accept=".json,application/json" aria-label="Open agent proposal JSON" onChange={(event) => {
        const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (!file) return;
        const fileRequestId = resetProposal('Reading the reply file…');
        setProposalText('');
        if (file.size > MAX_AGENT_PROPOSAL_BYTES) { setError('Proposal file is too large'); return; }
        void file.text().then((text) => {
          if (!operationIsCurrent(fileRequestId)) return;
          setProposalText(text);
          return reviewProposal(text);
        }).catch((cause) => {
          if (operationIsCurrent(fileRequestId)) setError(message(cause));
        });
      }} />
      <div className="agent-actions"><button onClick={() => fileInput.current?.click()}><FileUp size={14} /> Open reply file…</button><button className="agent-primary" disabled={!proposalText.trim()} onClick={() => void reviewProposal()}>Check returned circuit</button></div>
    </section>

    {proposal && <section className="agent-review" aria-label="Agent proposal review">
      <div className="agent-step-heading"><span>3</span><div><h3>Review and test the returned circuit</h3><p>The candidate remains separate from the editor until you explicitly apply it.</p></div></div>
      <div className="agent-check-grid">
        <div className="agent-check-card agent-check-pass"><CheckCircle2 size={16} /><div><strong>Correct circuit revision</strong><span>The reply matches the source you sent.</span></div></div>
        <div className={connectivityVerified && compileErrors.length === 0 ? 'agent-check-card agent-check-pass' : 'agent-check-card agent-check-fail'}>{connectivityVerified && compileErrors.length === 0 ? <CheckCircle2 size={16} /> : <X size={16} />}<div><strong>Core compile and connectivity</strong><span>{connectivityVerified && compileErrors.length === 0 ? 'Kessetsu accepted the source and verified its connections.' : 'Kessetsu rejected the returned circuit.'}</span></div></div>
        <div className={`agent-check-card ${simulationCard.tone}`}>
          {simulationCard.icon === 'pass' ? <CheckCircle2 size={16} />
            : simulationCard.icon === 'loading' ? <LoaderCircle className="agent-spin" size={16} />
              : simulationCard.icon === 'idle' ? <Play size={16} />
                : simulationCard.icon === 'error' ? <X size={16} /> : <AlertTriangle size={16} />}
          <div><strong>Local simulation and assertions</strong><span>{simulationCard.text}</span></div>
        </div>
      </div>
      {compileWarnings.length > 0 && <p className="agent-warning">{compileWarnings.length} compile warning{compileWarnings.length === 1 ? '' : 's'}: {compileWarnings.slice(0, 2).map((item) => item.message).join(' ')}</p>}
      <div className="agent-human-requirements">
        <strong>Your requested outcome</strong>
        <p>{requirements.trim() || 'No human requirements were recorded in this dialog. Review the source changes and encoded assertions carefully.'}</p>
        <span>Kessetsu can evaluate encoded assertions; it cannot infer that they cover every sentence above.</span>
      </div>
      <details className="agent-review-detail"><summary><span>Agent's explanation <em>unverified</em></span><small>Read</small></summary><p>{proposal.summary}</p></details>
      <details className="agent-review-detail"><summary><span>Source changes</span><small>{changedLines} changed lines</small></summary>
        <div className="agent-change-summary" aria-label="High-impact source changes">
          <span>Assertions <b>+{changeSummary.assertions.added} / −{changeSummary.assertions.removed}</b></span>
          <span>Analyses <b>+{changeSummary.analyses.added} / −{changeSummary.analyses.removed}</b></span>
          <span>Sources <b>+{changeSummary.sources.added} / −{changeSummary.sources.removed}</b></span>
          <span>Components <b>+{changeSummary.components.added} / −{changeSummary.components.removed}</b></span>
          <span>Connections <b>+{changeSummary.connections.added} / −{changeSummary.connections.removed}</b></span>
        </div>
        {(changeSummary.assertions.removed > 0 || changeSummary.sources.added + changeSummary.sources.removed > 0) && <p className="agent-impact-warning"><AlertTriangle size={14} /> Review removed assertions and source or supply changes before applying.</p>}
        <div className="agent-diff" role="region" aria-label="Proposed source diff" tabIndex={0}>{diff.map((line, index) => <div className={`agent-diff-${line.kind}`} key={`${index}-${line.kind}`}><span>{line.oldLine ?? ''}</span><span>{line.newLine ?? ''}</span><b>{line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' '}</b><code>{line.text || ' '}</code></div>)}</div>
      </details>
      {verification && <div className={`agent-verification ${verificationPassed ? '' : 'agent-verification-warning'}`} data-testid="agent-verification">{verificationPassed ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}<div><strong>{verificationPassed ? 'Simulation completed and all encoded assertions passed' : checkedAssertions === 'unchecked' ? 'Simulation completed without requirement checks' : 'Simulation completed with unresolved assertion results'}</strong><span>{verification.simulation.datasets.length} analyses · {assertionSummary?.passed}/{assertionSummary?.total} passed · {assertionSummary?.failed} failed · {assertionSummary?.errors} errors · {assertionSummary?.skipped} skipped. Only encoded assertions were checked.</span></div></div>}
      {confirmApply && canApply && !verificationPassed && <div className="agent-apply-confirm" role="alert">
        <AlertTriangle size={17} />
        <div><strong>Apply an unverified outcome?</strong><span>The simulation ran, but the requested outcome was not fully checked or satisfied. Review the diff before changing the editor.</span></div>
        <button onClick={() => setConfirmApply(false)}>Cancel</button>
        <button className="agent-confirm-apply" onClick={() => { if (!canApply) return; onAccept(proposal.proposed_source); close(); }}>Apply unchecked changes</button>
      </div>}
    </section>}

    {error && <p className="agent-error" role="alert">{error}</p>}
    <footer className="agent-footer"><p role="status">{status}</p><div>{verificationState === 'running' ? <button onClick={cancelVerification}><Square size={14} /> Stop test</button> : <button disabled={!inputsMatch || !proposal || !connectivityVerified || compileErrors.length > 0} onClick={() => void runVerification()}><Play size={14} /> {verificationState === 'complete' || verificationState === 'failed' || verificationState === 'cancelled' ? 'Test again' : 'Test proposed circuit'}</button>}<button className="agent-accept" disabled={!canApply} onClick={() => {
      if (!proposal || !canApply) return;
      if (verificationPassed) { onAccept(proposal.proposed_source); close(); }
      else setConfirmApply(true);
    }}>{verificationPassed ? 'Apply tested proposal' : verificationState === 'complete' ? 'Apply anyway…' : 'Apply to editor'}</button></div></footer>
  </dialog>;
}
