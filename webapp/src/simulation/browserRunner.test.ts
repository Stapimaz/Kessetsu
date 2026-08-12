import { describe, expect, it, vi } from 'vitest';
import {
  BrowserSimulationRunner,
  SimulationCancelledError,
  SimulationTimeoutError,
} from './browserRunner';
import type { BrowserSimulationPlan, SimulationResult, WorkerRequest, WorkerResponse } from './types';

const plan: BrowserSimulationPlan = {
  schema_version: 'netlang.simulation.v1',
  simulator_adapter: 'eecircuit-engine@1.7.0',
  analyses: [],
};

const result: SimulationResult = {
  schema_version: 'netlang.simulation.v1',
  status: 'succeeded',
  analyses: [],
  simulator: { executable: 'test', version: 'test' },
  process: { exit_code: 0, success: true },
  measurements: {},
  datasets: [],
  diagnostics: [],
  warnings: [],
  errors: [],
  raw_log: { stdout: '', stderr: '' },
  artifacts: [],
};

class FakeWorker {
  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  terminated = false;
  request: WorkerRequest | null = null;

  postMessage(request: WorkerRequest) {
    this.request = request;
  }

  terminate() {
    this.terminated = true;
  }

  respond(message: WorkerResponse) {
    this.onmessage?.({ data: message } as MessageEvent<WorkerResponse>);
  }

  crash(message: string) {
    this.onerror?.({ message } as ErrorEvent);
  }
}

function runnerWith(...workers: FakeWorker[]) {
  return new BrowserSimulationRunner(() => {
    const worker = workers.shift();
    if (!worker) throw new Error('test worker pool exhausted');
    return worker as unknown as Worker;
  });
}

describe('BrowserSimulationRunner lifecycle', () => {
  it('hard-cancels the worker and rejects the active promise', async () => {
    const worker = new FakeWorker();
    const runner = runnerWith(worker);
    const pending = runner.run(plan);
    runner.cancel();
    await expect(pending).rejects.toBeInstanceOf(SimulationCancelledError);
    expect(worker.terminated).toBe(true);
  });

  it('times out, terminates and returns a typed timeout error', async () => {
    vi.useFakeTimers();
    const worker = new FakeWorker();
    const pending = runnerWith(worker).run(plan, { timeoutMs: 25 });
    const rejection = expect(pending).rejects.toBeInstanceOf(SimulationTimeoutError);
    await vi.advanceTimersByTimeAsync(25);
    await rejection;
    expect(worker.terminated).toBe(true);
    vi.useRealTimers();
  });

  it('recovers with a fresh worker after a crash', async () => {
    const crashed = new FakeWorker();
    const recovered = new FakeWorker();
    const runner = runnerWith(crashed, recovered);
    const first = runner.run(plan);
    crashed.crash('synthetic crash');
    await expect(first).rejects.toThrow('synthetic crash');

    const second = runner.run(plan);
    const id = recovered.request?.id ?? -1;
    recovered.respond({ type: 'result', id, result });
    await expect(second).resolves.toEqual(result);
  });

  it('suppresses a terminated worker stale result', async () => {
    const stale = new FakeWorker();
    const current = new FakeWorker();
    const runner = runnerWith(stale, current);
    const first = runner.run(plan);
    const firstId = stale.request?.id ?? -1;
    const firstRejection = expect(first).rejects.toBeInstanceOf(SimulationCancelledError);
    const second = runner.run(plan);
    await firstRejection;

    stale.respond({ type: 'result', id: firstId, result });
    const secondId = current.request?.id ?? -1;
    current.respond({ type: 'result', id: secondId, result });
    await expect(second).resolves.toEqual(result);
  });
});
