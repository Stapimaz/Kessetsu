// Evaluator-owned testbench execution; never a raw candidate execution API.
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
export const root = fileURLToPath(new URL('../../', import.meta.url));
export const digest = (value) => createHash('sha256').update(value).digest('hex');

export function runBench(task, bench, simulator, names, measure, runSimulator = spawnSync, evidenceBench = bench, supportFiles = {}) {
  const directory = mkdtempSync(join(tmpdir(), `kessetsu-${task.toLowerCase()}-`));
  const evidence = { testbench: evidenceBench };
  try {
    writeFileSync(join(directory, 'bench.spice'), bench);
    for (const [name, contents] of Object.entries(supportFiles)) {
      if (!/^[A-Za-z0-9._-]+$/.test(name) || typeof contents !== 'string') throw new Error('Invalid evaluator support file');
      writeFileSync(join(directory, name), contents);
    }
    const executable = typeof simulator === 'string' ? simulator : simulator.executable;
    const prefixArgs = typeof simulator === 'string' ? [] : simulator.args;
    const run = runSimulator(executable, [...prefixArgs, '-n', '-b', 'bench.spice'], { cwd: directory, encoding: 'utf8', timeout: 30000, maxBuffer: 2 * 1024 * 1024, windowsHide: true });
    Object.assign(evidence, { simulator_stdout: run.stdout, simulator_stderr: run.stderr, simulator_exit_code: run.status,
      simulator_signal: run.signal, simulator_error: run.error?.message ?? null });
    const raw = {};
    for (const name of names) {
      try { raw[name] = readFileSync(join(directory, `${name}.data`), 'utf8'); evidence[`${name}_data`] = raw[name]; }
      catch (error) { evidence[`${name}_data_error`] = error.code ?? error.message; }
    }
    if (run.error || run.status !== 0) throw new Error('Simulator process failed');
    if (Object.keys(raw).length !== names.length) throw new Error('Missing simulator datasets');
    return { ...measure(raw), evidence };
  } catch (error) { error.evidence = evidence; throw error; }
  finally { rmSync(directory, { recursive: true, force: true }); } // Only this invocation's mkdtemp path.
}

export function evaluatorMain(task, evaluate) {
  const [arm, path] = process.argv.slice(2);
  const record = { schema_version: `kessetsu.${task.toLowerCase()}-evaluation.v1`, task, arm };
  try {
    record.spec_sha256 = digest(readFileSync(join(root, 'docs/evals/unseen-design-v1.md')));
    if (!['direct', 'kessetsu'].includes(arm) || !path) throw new Error('Expected <kessetsu|direct> <candidate-file>');
    const source = readFileSync(resolve(path), 'utf8');
    Object.assign(record, { candidate_source: source, candidate_sha256: digest(source) });
    let netlist = source;
    if (arm === 'kessetsu') {
      const binary = process.env.KESSETSU_BINARY ?? join(root, 'core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
      const run = spawnSync(binary, ['compile', '-', '--format', 'json', '--include', 'spice'], { input: source, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
      Object.assign(record, { compiler_stdout: run.stdout, compiler_stderr: run.stderr });
      if (run.error || run.status !== 0) throw new Error(`Compilation failed: ${run.error?.message ?? run.status}`);
      netlist = JSON.parse(run.stdout).debug?.spice_netlist;
      if (typeof netlist !== 'string') throw new Error('Missing compiled netlist');
    }
    Object.assign(record, { compiled_netlist: netlist, netlist_sha256: digest(netlist) });
    const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32' ? join(root, 'core/tools/ngspice/bin/ngspice_con.exe') : 'ngspice');
    Object.assign(record, evaluate(netlist, simulator));
    process.exitCode = record.status === 'PASS' ? 0 : 1;
  } catch (error) {
    Object.assign(record, { status: 'ERROR', message: error.message, ...(error.evidence ? { evidence: error.evidence } : {}) });
    process.exitCode = 2;
  }
  console.log(JSON.stringify(record));
}
