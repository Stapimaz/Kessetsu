import SimulationWorker from './simulation.worker?worker';
import type { BrowserSimulationPlan, SimulationResult, WorkerRequest, WorkerResponse } from './types';

export type RunnerProgress = { completed: number; total: number; message: string };

export class SimulationCancelledError extends Error {
  constructor() {
    super('Simulation cancelled');
    this.name = 'SimulationCancelledError';
  }
}

export class SimulationTimeoutError extends Error {
  constructor(timeoutMs: number) {
    super(`Simulation timed out after ${timeoutMs} ms`);
    this.name = 'SimulationTimeoutError';
  }
}

export class BrowserSimulationRunner {
  private worker: Worker | null = null;
  private nextId = 1;
  private activeId: number | null = null;
  private rejectActive: ((reason: Error) => void) | null = null;
  private activeTimeout: ReturnType<typeof setTimeout> | null = null;
  private readonly workerFactory: () => Worker;

  constructor(workerFactory: () => Worker = () => new SimulationWorker()) {
    this.workerFactory = workerFactory;
  }

  private createWorker() {
    this.worker?.terminate();
    this.worker = this.workerFactory();
    return this.worker;
  }

  run(
    plan: BrowserSimulationPlan,
    options: {
      timeoutMs?: number;
      includeRawLog?: boolean;
      onProgress?: (progress: RunnerProgress) => void;
    } = {},
  ): Promise<SimulationResult> {
    this.cancel();
    const id = this.nextId++;
    this.activeId = id;
    const worker = this.createWorker();
    const timeoutMs = Math.max(1, options.timeoutMs ?? 30_000);

    return new Promise((resolve, reject) => {
      const timeout = globalThis.setTimeout(() => {
        if (this.activeId !== id) return;
        finish();
        reject(new SimulationTimeoutError(timeoutMs));
      }, timeoutMs);
      this.activeTimeout = timeout;
      this.rejectActive = reject;

      const finish = () => {
        globalThis.clearTimeout(timeout);
        worker.terminate();
        if (this.worker === worker) this.worker = null;
        if (this.activeId === id) this.activeId = null;
        if (this.activeTimeout === timeout) this.activeTimeout = null;
        if (this.rejectActive === reject) this.rejectActive = null;
      };

      worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const message = event.data;
        if (message.id !== id || this.activeId !== id) return;
        if (message.type === 'progress') {
          options.onProgress?.(message);
          return;
        }
        finish();
        if (message.type === 'result') resolve(message.result);
        else reject(new Error(message.message));
      };
      worker.onerror = (event) => {
        if (this.activeId !== id) return;
        finish();
        reject(new Error(event.message || 'Simulation worker crashed'));
      };
      const request: WorkerRequest = {
        type: 'run',
        id,
        plan,
        includeRawLog: options.includeRawLog ?? false,
      };
      worker.postMessage(request);
    });
  }

  cancel() {
    const reject = this.rejectActive;
    this.activeId = null;
    if (this.activeTimeout !== null) globalThis.clearTimeout(this.activeTimeout);
    this.activeTimeout = null;
    this.rejectActive = null;
    this.worker?.terminate();
    this.worker = null;
    reject?.(new SimulationCancelledError());
  }

  dispose() {
    this.cancel();
  }
}
