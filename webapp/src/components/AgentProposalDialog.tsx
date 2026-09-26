import { CheckCircle2, Clipboard, Download, FileUp, Play, Square, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  compile_kessetsu_with_resources,
  compile_schema_version,
  evaluate_browser_simulation_with_resources,
  prepare_browser_simulation_with_resources,
} from 'kessetsu-core';
import {
  createAgentTask,
  lineDiff,
  MAX_AGENT_PROPOSAL_BYTES,
  parseAgentProposal,
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
  if (!runner.current) runner.current = new BrowserSimulationRunner();
  const [requirements, setRequirements] = useState('');
  const [taskText, setTaskText] = useState('');
  const [proposalText, setProposalText] = useState('');
  const [proposal, setProposal] = useState<AgentProposalEnvelope | null>(null);
  const [compileReport, setCompileReport] = useState<CompileReport | null>(null);
  const [verification, setVerification] = useState<BrowserEvaluation | null>(null);
  const [verificationState, setVerificationState] = useState<VerificationState>('idle');
  const [status, setStatus] = useState('Nothing leaves this browser automatically.');
  const [error, setError] = useState('');

  const resetProposal = () => {
    runner.current?.cancel();
    setProposal(null);
    setCompileReport(null);
    setVerification(null);
    setVerificationState('idle');
    setError('');
    setStatus('Nothing leaves this browser automatically.');
  };

  useEffect(() => {
    if (open && sourceSnapshot.current !== source) {
      sourceSnapshot.current = source;
      setTaskText('');
      setProposalText('');
      resetProposal();
    }
    if (open && !dialog.current?.open) dialog.current?.showModal();
    if (!open && dialog.current?.open) dialog.current.close();
  }, [open, source]);
  useEffect(() => () => runner.current?.dispose(), []);

  const diff = useMemo(() => proposal ? lineDiff(source, proposal.proposed_source) : [], [proposal, source]);
  const changedLines = diff.filter((line) => line.kind !== 'same').length;
  const compileErrors = compileReport?.diagnostics.filter((item) => item.severity === 'error') ?? [];
  const compileWarnings = compileReport?.diagnostics.filter((item) => item.severity === 'warning') ?? [];
  const connectivityVerified = Boolean(compileReport?.schematic?.connectivity.verified && compileReport.schematic_svg);
  const assertionSummary = verification?.assertions.summary;
  const verificationPassed = verificationState === 'complete'
    && Boolean(assertionSummary)
    && assertionSummary!.failed === 0
    && assertionSummary!.errors === 0;

  const buildTask = async () => {
    setError('');
    try {
      const task = await createAgentTask(source, name, requirements, productVersion, compile_schema_version());
      const serialized = JSON.stringify(task, null, 2);
      setTaskText(serialized);
      setStatus('Agent task created from this exact source revision.');
      return serialized;
    } catch (cause) { setError(message(cause)); return null; }
  };

  const copyTask = async () => {
    const serialized = taskText || await buildTask();
    if (!serialized) return;
    try {
      await navigator.clipboard.writeText(serialized);
      setStatus('Task copied. Paste it into the AI agent you want to use.');
    } catch { setError('Clipboard access was blocked. Download the task JSON instead.'); }
  };

  const downloadTask = async () => {
    const serialized = taskText || await buildTask();
    if (!serialized) return;
    downloadTextFile(serialized, `${sanitizeFileStem(name)}.kessagent.json`);
    setStatus('Task downloaded. Give the JSON file to the AI agent you want to use.');
  };

  const reviewProposal = async (text = proposalText) => {
    resetProposal();
    try {
      const parsed = await parseAgentProposal(text, source);
      const report = compileCandidate(parsed.proposed_source, resources);
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
    } catch (cause) { setError(message(cause)); }
  };

  const runVerification = async () => {
    if (!proposal || !connectivityVerified || compileErrors.length) return;
    setError('');
    setVerification(null);
    setVerificationState('running');
    setStatus('Preparing proposal simulation…');
    try {
      const plan = prepare_browser_simulation_with_resources(proposal.proposed_source, resources) as BrowserSimulationPlan;
      if (plan.analyses.length === 0) throw new Error('The proposal has no simulation command. Ask the agent to include a verifiable analysis.');
      const simulation = await runner.current!.run(plan, {
        timeoutMs: 90_000,
        onProgress: (progress) => setStatus(progress.message),
      });
      const evaluation = evaluate_browser_simulation_with_resources(proposal.proposed_source, simulation, resources) as BrowserEvaluation;
      setVerification(evaluation);
      setVerificationState('complete');
      setStatus(`${evaluation.simulation.datasets.length} analyses completed · ${evaluation.assertions.summary.passed}/${evaluation.assertions.summary.total} assertions passed.`);
    } catch (cause) {
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
    runner.current?.cancel();
    setVerificationState('cancelled');
  };

  const close = () => {
    if (verificationState === 'running') cancelVerification();
    dialog.current?.close();
  };

  return <dialog ref={dialog} className="app-dialog agent-proposal-dialog" aria-labelledby="agent-proposal-title"
    onCancel={(event) => { event.preventDefault(); close(); }} onClose={onClose}>
    <header><div><h2 id="agent-proposal-title">Work with an AI agent</h2><p>Send the current circuit to an AI, then let Kessetsu test its returned design before you apply it.</p></div><button aria-label="Close AI agent workflow" onClick={close}><X size={18} /></button></header>

    <div className="agent-privacy-note">
      <strong>You stay in control.</strong>
      <span>Kessetsu never contacts an AI service or changes your circuit automatically. You choose the agent and move the task between apps.</span>
    </div>

    <section className="agent-step">
      <div className="agent-step-heading"><span>1</span><div><h3>Tell the agent what you need</h3><p>Write the electrical target and limits. Kessetsu includes your current circuit automatically.</p></div></div>
      <textarea rows={4} value={requirements} maxLength={12_000} placeholder="Example: Design for 2 W RMS into 8 ohm, gain near 20, and less than 2 W transistor dissipation. Include simulations and assertions for every target." onChange={(event) => { setRequirements(event.target.value); setTaskText(''); }} />
      <div className="agent-actions"><button className="agent-primary" onClick={() => void copyTask()}><Clipboard size={14} /> Copy task for AI</button><button onClick={() => void downloadTask()}><Download size={14} /> Download task file</button></div>
      {taskText && <div className="agent-next-step"><CheckCircle2 size={16} /><div><strong>Task ready</strong><span>Paste it into ChatGPT, Gemini, Claude, Codex, or another agent. Ask it to return only the requested JSON object.</span></div></div>}
      {taskText && <details><summary>Inspect the task JSON</summary><pre>{taskText}</pre></details>}
    </section>

    <section className="agent-step">
      <div className="agent-step-heading"><span>2</span><div><h3>Bring back the agent's reply</h3><p>Paste the complete JSON response, or open the JSON file the agent created.</p></div></div>
      <textarea rows={5} value={proposalText} placeholder={`Paste the returned ${'kessetsu.agent-proposal.v1'} JSON here…`} onChange={(event) => setProposalText(event.target.value)} />
      <input ref={fileInput} type="file" className="sr-only" accept=".json,application/json" aria-label="Open agent proposal JSON" onChange={(event) => {
        const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (!file) return;
        if (file.size > MAX_AGENT_PROPOSAL_BYTES) { setError('Proposal file is too large'); return; }
        void file.text().then((text) => { setProposalText(text); return reviewProposal(text); }).catch((cause) => setError(message(cause)));
      }} />
      <div className="agent-actions"><button onClick={() => fileInput.current?.click()}><FileUp size={14} /> Open reply file…</button><button className="agent-primary" disabled={!proposalText.trim()} onClick={() => void reviewProposal()}>Check returned circuit</button></div>
    </section>

    {proposal && <section className="agent-review" aria-label="Agent proposal review">
      <div className="agent-step-heading"><span>3</span><div><h3>Review and test the returned circuit</h3><p>The candidate remains separate from the editor until you explicitly apply it.</p></div></div>
      <div className="agent-check-grid">
        <div className="agent-check-card agent-check-pass"><CheckCircle2 size={16} /><div><strong>Correct circuit revision</strong><span>The reply matches the source you sent.</span></div></div>
        <div className={connectivityVerified && compileErrors.length === 0 ? 'agent-check-card agent-check-pass' : 'agent-check-card agent-check-fail'}>{connectivityVerified && compileErrors.length === 0 ? <CheckCircle2 size={16} /> : <X size={16} />}<div><strong>Core compile and connectivity</strong><span>{connectivityVerified && compileErrors.length === 0 ? 'Kessetsu accepted the source and verified its connections.' : 'Kessetsu rejected the returned circuit.'}</span></div></div>
        <div className={`agent-check-card ${verificationState === 'complete' ? (verificationPassed ? 'agent-check-pass' : 'agent-check-warning') : 'agent-check-pending'}`}>
          {verificationState === 'complete' ? (verificationPassed ? <CheckCircle2 size={16} /> : <X size={16} />) : <Play size={16} />}
          <div><strong>Local simulation</strong><span>{verificationState === 'complete'
            ? assertionSummary!.total === 0
              ? 'Simulation completed, but the proposal included no assertions.'
              : `${assertionSummary!.passed}/${assertionSummary!.total} assertions passed.`
            : 'Not run yet. Test the candidate before applying it.'}</span></div>
        </div>
      </div>
      {compileWarnings.length > 0 && <p className="agent-warning">{compileWarnings.length} compile warning{compileWarnings.length === 1 ? '' : 's'}: {compileWarnings.slice(0, 2).map((item) => item.message).join(' ')}</p>}
      <details className="agent-review-detail"><summary><span>Agent's explanation <em>unverified</em></span><small>Read</small></summary><p>{proposal.summary}</p></details>
      <details className="agent-review-detail"><summary><span>Source changes</span><small>{changedLines} changed lines</small></summary><div className="agent-diff" role="region" aria-label="Proposed source diff" tabIndex={0}>{diff.map((line, index) => <div className={`agent-diff-${line.kind}`} key={`${index}-${line.kind}`}><span>{line.oldLine ?? ''}</span><span>{line.newLine ?? ''}</span><b>{line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' '}</b><code>{line.text || ' '}</code></div>)}</div></details>
      {verification && <div className={`agent-verification ${verificationPassed ? '' : 'agent-verification-warning'}`} data-testid="agent-verification">{verificationPassed ? <CheckCircle2 size={16} /> : <X size={16} />}<div><strong>{verificationPassed ? 'Kessetsu verification passed' : 'Simulation completed with unchecked or failed requirements'}</strong><span>{verification.simulation.datasets.length} analyses · {assertionSummary?.passed}/{assertionSummary?.total} assertions passed · {assertionSummary?.failed} failed · {assertionSummary?.errors} errors</span></div></div>}
    </section>}

    {error && <p className="agent-error" role="alert">{error}</p>}
    <footer className="agent-footer"><p role="status">{status}</p><div>{verificationState === 'running' ? <button onClick={cancelVerification}><Square size={14} /> Stop test</button> : <button disabled={!proposal || !connectivityVerified || compileErrors.length > 0} onClick={() => void runVerification()}><Play size={14} /> {verificationState === 'complete' ? 'Test again' : 'Test proposed circuit'}</button>}<button className="agent-accept" disabled={!proposal || verificationState !== 'complete'} onClick={() => { onAccept(proposal!.proposed_source); close(); }}>Apply to editor</button></div></footer>
  </dialog>;
}
