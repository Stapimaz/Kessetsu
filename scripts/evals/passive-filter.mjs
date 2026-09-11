// Independent U1 evaluator: never execute the candidate's SPICE control block.
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { verificationScope } from './verification-scope.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const digest = (text) => createHash('sha256').update(text).digest('hex');

export function spiceNumber(text) {
  const match = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(meg|[tgkmunpf])?$/i.exec(text);
  if (!match) throw new Error(`Unsupported SPICE number: ${text}`);
  const scales = { t: 1e12, g: 1e9, meg: 1e6, k: 1e3, m: 1e-3, u: 1e-6, n: 1e-9, p: 1e-12, f: 1e-15 };
  const value = Number(match[1]) * (scales[match[2]?.toLowerCase()] ?? 1);
  if (!Number.isFinite(value)) throw new Error('Non-finite component value');
  return value;
}

export function inspectU1(netlist) {
  const devices = [];
  let control = false, ended = false;
  const lines = netlist.replace(/^\uFEFF/, '').split(/\r?\n/);
  // A SPICE file's first line is its title, not an executable/device line.
  for (const original of lines.slice(1)) {
    const line = original.trim();
    if (!line || line.startsWith('*')) continue;
    if (ended) throw new Error('Content after SPICE end directive');
    if (/^\.control$/i.test(line)) { if (control) throw new Error('Nested control block'); control = true; continue; }
    if (/^\.endc$/i.test(line)) { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue;
    if (/^\.end$/i.test(line)) { ended = true; continue; }
    if (/^\.(op|ac|tran|meas|measure)(\s|$)/i.test(line)) continue;
    const fields = line.split(/\s+/);
    const kind = fields[0][0].toUpperCase();
    if (!['R', 'C', 'V'].includes(kind) || fields.length < 4) throw new Error(`Unsupported U1 device/directive: ${line}`);
    if (devices.some((device) => device.name.toLowerCase() === fields[0].toLowerCase())) throw new Error('Duplicate device');
    const nodes = fields.slice(1, 3).map((node) => node.toLowerCase());
    if (nodes.some((node) => !/^[a-z0-9_]+$/.test(node)) || nodes[0] === nodes[1]) throw new Error('Invalid or shorted device nodes');
    if (kind === 'V') {
      const tail = fields.slice(3).join(' ');
      const ac = /^(?:(?:DC\s+)?([+\-\d.e]+)\s+)?AC\s+([^\s]+)(?:\s+(0))?$/i.exec(tail);
      if (!ac || spiceNumber(ac[2]) !== 1) throw new Error('U1 source must have fixed AC amplitude 1 V and phase 0');
      if (ac[1] && !Number.isFinite(Number(ac[1]))) throw new Error('Invalid source DC value');
      devices.push({ kind, name: fields[0], nodes, value: 1 });
    } else {
      if (fields.length !== 4) throw new Error('Extra device parameters are not supported in U1');
      const value = spiceNumber(fields[3]);
      if (value <= 0) throw new Error('Passive values must be positive');
      devices.push({ kind, name: fields[0], nodes, value });
    }
  }
  if (control) throw new Error('Unterminated control block');
  const sources = devices.filter((device) => device.kind === 'V');
  const resistors = devices.filter((device) => device.kind === 'R');
  const capacitors = devices.filter((device) => device.kind === 'C');
  if (sources.length !== 1 || resistors.length !== 2 || capacitors.length !== 1) throw new Error('U1 requires exactly one source, two resistors, and one capacitor');
  const source = sources[0];
  if (source.nodes[1] !== '0') throw new Error('U1 source negative terminal must be ground');
  const input = source.nodes[0];
  const capacitor = capacitors[0];
  if (!capacitor.nodes.includes('0')) throw new Error('U1 capacitor must be shunt-connected to ground');
  const output = capacitor.nodes.find((node) => node !== '0');
  if (output === input) throw new Error('U1 needs a distinct output node');
  const connects = (device, a, b) => device.nodes.includes(a) && device.nodes.includes(b);
  const series = resistors.find((device) => connects(device, input, output));
  const load = resistors.find((device) => connects(device, output, '0'));
  if (!series || !load) throw new Error('U1 requires series R followed by parallel load and C');
  if (load.value !== 100000) throw new Error('Fixed U1 load changed: expected 100 kohm');
  if (series.value < 1000 || series.value > 10000) throw new Error('Series R must be between 1 and 10 kohm');
  return { resistance: series.value, capacitance: capacitor.value, load: load.value };
}

export function parseRows(text, columns) {
  const rows = text.trim().split(/\r?\n/).map((line) => line.trim().split(/\s+/).map(Number));
  if (!rows.length || rows.some((row) => row.length !== columns || row.some((value) => !Number.isFinite(value)))) throw new Error('Malformed simulator data');
  return rows;
}

export function cutoffFrequency(rows, dcGain) {
  if (!(dcGain > 0) || rows.length < 2) throw new Error('Insufficient AC data');
  const target = dcGain / Math.sqrt(2);
  const points = rows.map((row, index) => {
    if (row[0] <= 0 || row[0] !== row[3] || (index && row[0] <= rows[index - 1][0])) throw new Error('Invalid AC frequency axis');
    const input = Math.hypot(row[4], row[5]);
    if (!(input > 0)) throw new Error('Zero AC input');
    return [row[0], Math.hypot(row[1], row[2]) / input];
  });
  const crossings = [];
  for (let index = 1; index < points.length; index++) {
    const [f0, g0] = points[index - 1];
    const [f1, g1] = points[index];
    if (g0 >= target && g1 < target) {
      const fraction = (Math.log(target) - Math.log(g0)) / (Math.log(g1) - Math.log(g0));
      crossings.push(Math.exp(Math.log(f0) + fraction * (Math.log(f1) - Math.log(f0))));
    }
  }
  if (crossings.length !== 1 || points[0][1] < target || points.at(-1)[1] >= target) throw new Error('Expected one low-pass cutoff crossing');
  return crossings[0];
}

export function evaluateU1(netlist, simulator, runSimulator = spawnSync) {
  const circuit = inspectU1(netlist);
  const expectedGain = circuit.load / (circuit.resistance + circuit.load);
  const parallel = circuit.resistance * circuit.load / (circuit.resistance + circuit.load);
  const expectedCutoff = 1 / (2 * Math.PI * parallel * circuit.capacitance);
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u1-'));
  // Only validated positive finite component values cross into the testbench.
  const bench = `Evaluator-owned U1\nVTEST in 0 DC 1 AC 1\nRS in out ${circuit.resistance}\nRL out 0 100000\nC1 out 0 ${circuit.capacitance}\n.control\nop\nwrdata op.data v(out)\nac dec 100 10 1000000\nwrdata ac.data v(out) v(in)\nquit\n.endc\n.end\n`;
  const evidence = { testbench: bench };
  try {
    writeFileSync(join(directory, 'bench.spice'), bench);
    const run = runSimulator(simulator, ['-n', '-b', 'bench.spice'], { cwd: directory, encoding: 'utf8', timeout: 30000, maxBuffer: 2 * 1024 * 1024, windowsHide: true });
    Object.assign(evidence, { simulator_stdout: run.stdout, simulator_stderr: run.stderr,
      simulator_exit_code: run.status, simulator_signal: run.signal, simulator_error: run.error?.message ?? null });
    for (const name of ['op', 'ac']) {
      try { evidence[`${name}_data`] = readFileSync(join(directory, `${name}.data`), 'utf8'); }
      catch (error) { evidence[`${name}_data_error`] = error.code ?? error.message; }
    }
    if (run.error || run.status !== 0) throw new Error(`Ngspice failed: ${run.error?.message ?? run.stderr}`);
    const opText = evidence.op_data;
    const acText = evidence.ac_data;
    if (opText === undefined || acText === undefined) throw new Error('Missing simulator datasets');
    const op = parseRows(opText, 2);
    if (op.length !== 1) throw new Error('Expected one operating-point sample');
    const dcGain = op[0][1];
    const cutoff = cutoffFrequency(parseRows(acText, 6), dcGain);
    const formulaAgreement = Math.abs(dcGain - expectedGain) < 1e-6 && Math.abs(cutoff / expectedCutoff - 1) < 0.001;
    const checks = { fixed_requirements: true, dc_gain: dcGain >= 0.95, cutoff: cutoff >= 1450 && cutoff <= 1750, independent_formula_agreement: formulaAgreement };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks,
      measurements: { dc_gain: dcGain, cutoff_hz: cutoff }, analytic_reference: { dc_gain: expectedGain, cutoff_hz: expectedCutoff },
      verification_scope: verificationScope('U1', checks),
      evidence };
  } catch (error) { error.evidence = evidence; throw error; } finally {
    // This path is created by mkdtemp for this evaluation, never caller-supplied.
    rmSync(directory, { recursive: true, force: true });
  }
}

function main() {
  const [arm, candidatePath] = process.argv.slice(2);
  const record = { schema_version: 'kessetsu.u1-evaluation.v1', task: 'U1', arm };
  try {
  record.spec_sha256 = digest(readFileSync(join(root, 'docs/evals/unseen-design-v1.md')));
  if (!['kessetsu', 'direct'].includes(arm) || !candidatePath) throw new Error('Usage: node scripts/evals/passive-filter.mjs <kessetsu|direct> <candidate-file>');
  const candidate = readFileSync(resolve(candidatePath), 'utf8');
  Object.assign(record, { candidate_source: candidate, candidate_sha256: digest(candidate) });
  let netlist = candidate;
  if (arm === 'kessetsu') {
    const binary = process.env.KESSETSU_BINARY ?? join(root, 'core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
    const compile = spawnSync(binary, ['compile', '-', '--format', 'json', '--include', 'spice'], { input: candidate, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
    Object.assign(record, { compiler_stdout: compile.stdout, compiler_stderr: compile.stderr });
    if (compile.error || compile.status !== 0) throw new Error(`Candidate compilation failed: ${compile.error?.message ?? compile.stdout}`);
    const report = JSON.parse(compile.stdout);
    netlist = report.debug?.spice_netlist;
    if (typeof netlist !== 'string') throw new Error('Compile output did not contain SPICE');
  }
  const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32' ? join(root, 'core/tools/ngspice/bin/ngspice_con.exe') : 'ngspice');
  Object.assign(record, { netlist_sha256: digest(netlist), compiled_netlist: netlist });
  const result = evaluateU1(netlist, simulator);
  Object.assign(record, result);
  process.exitCode = result.status === 'PASS' ? 0 : 1;
  } catch (error) {
    Object.assign(record, { status: 'ERROR', message: error.message, ...(error.evidence ? { evidence: error.evidence } : {}) });
    process.exitCode = 2;
  }
  console.log(JSON.stringify(record));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
