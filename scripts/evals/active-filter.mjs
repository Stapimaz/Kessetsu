// Bounded U2 evaluator, independent of Core measurement/assertion code.
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { spiceNumber, parseRows, cutoffFrequency } from './passive-filter.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const digest = (value) => createHash('sha256').update(value).digest('hex');
export const MODEL = `.subckt KESSETSU_OPAMP_V1 in_p in_n vcc vee out
EGAIN n_int 0 in_p in_n 200000
RPOLE n_int out 1
CPOLE out 0 0.03183098861837907
RLOAD out 0 1e9
.ends KESSETSU_OPAMP_V1`;
const limitations = ['Generic linear model only; no manufacturer-part validation.',
  'Supply pins do not impose rail saturation, current limits, or supply-current consumption.',
  'Evaluator currently supports input RC followed by a resistive-feedback non-inverting stage; other topologies/models are unsupported, not electrical failures.'];
const normalize = (text) => text.trim().toLowerCase().split(/\s+/);
const sameModel = (text) => {
  const actual = normalize(text), expected = normalize(MODEL);
  return actual.length === expected.length && actual.every((value, index) => {
    if (value === expected[index]) return true;
    try { return Math.abs(spiceNumber(value) / spiceNumber(expected[index]) - 1) < 1e-14; } catch { return false; }
  });
};

export function inspectU2(netlist) {
  const devices = [], model = [];
  let control = false, inModel = false, modelDone = false, ended = false;
  for (const original of netlist.replace(/^\uFEFF/, '').split(/\r?\n/).slice(1)) {
    const line = original.trim();
    if (!line || line.startsWith('*')) continue;
    if (ended) throw new Error('Content after SPICE end directive');
    if (/^\.control$/i.test(line)) { if (control || inModel) throw new Error('Invalid control block'); control = true; continue; }
    if (/^\.endc$/i.test(line)) { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue; // Never execute candidate commands.
    if (/^\.subckt\b/i.test(line)) { if (inModel || modelDone) throw new Error('Unsupported extra model'); inModel = true; }
    if (inModel) {
      model.push(line);
      if (/^\.ends\b/i.test(line)) { inModel = false; modelDone = true; }
      continue;
    }
    if (/^\.end$/i.test(line)) { ended = true; continue; }
    if (/^\.(op|ac|tran|meas|measure)(\s|$)/i.test(line)) continue;
    const fields = normalize(line), kind = fields[0][0];
    if (!['r', 'c', 'v', 'x'].includes(kind)) throw new Error(`Unsupported U2 device/directive: ${line}`);
    const nodes = fields.slice(1, kind === 'x' ? 6 : 3);
    if (nodes.length !== (kind === 'x' ? 5 : 2) || nodes.some((node) => !/^[a-z0-9_]+$/.test(node))) throw new Error('Invalid device nodes');
    if (devices.some((device) => device.name === fields[0])) throw new Error('Duplicate device');
    const device = { kind, name: fields[0], nodes };
    if (kind === 'x') {
      if (fields.length !== 7 || fields[6] !== 'kessetsu_opamp_v1') throw new Error('Unsupported op-amp model');
    } else if (kind === 'v') {
      const tail = fields.slice(3).join(' ');
      const sine = /^(?:dc\s+0\s+)?sin(?:e)?\(\s*([^()]+)\)\s+ac\s+(\S+)$/.exec(tail);
      if (sine) {
        const values = sine[1].trim().split(/[\s,]+/).map(spiceNumber);
        if (values.length !== 3 || values[0] !== 0 || values[1] !== 0.05 || values[2] !== 100 || spiceNumber(sine[2]) !== 1) throw new Error('Fixed U2 stimulus changed');
        device.signal = true;
      } else {
        device.value = spiceNumber(tail.replace(/^dc\s+/, ''));
      }
    } else {
      if (fields.length !== 4) throw new Error('Unsupported passive parameters');
      device.value = spiceNumber(fields[3]);
      if (!(device.value > 0)) throw new Error('Passive values must be positive');
    }
    devices.push(device);
  }
  if (control || inModel || !sameModel(model.join('\n'))) throw new Error('Missing, altered, or unsupported generic model');
  const ofKind = (kind) => devices.filter((device) => device.kind === kind);
  const [resistors, capacitors, sources, amps] = ['r', 'c', 'v', 'x'].map(ofKind);
  if (resistors.length !== 4 || capacitors.length !== 1 || sources.length !== 3 || amps.length !== 1) throw new Error('Unsupported U2 topology: expected input RC, feedback divider, and load');
  const [positive, negative, vcc, vee, output] = amps[0].nodes;
  const signal = sources.find((device) => device.signal);
  if (!signal || sources.filter((device) => device.signal).length !== 1 || signal.nodes[1] !== '0') throw new Error('Fixed input source missing');
  const input = signal.nodes[0];
  if (new Set(['0', positive, negative, vcc, vee, output, input]).size !== 7) throw new Error('Shorted stage or supply pins');
  const connects = (device, a, b) => device.nodes.includes(a) && device.nodes.includes(b);
  const rail = (node, voltage) => sources.some((device) => !device.signal &&
    ((device.nodes[0] === node && device.nodes[1] === '0' && device.value === voltage) ||
     (device.nodes[1] === node && device.nodes[0] === '0' && device.value === -voltage)));
  if (!rail(vcc, 6) || !rail(vee, -6)) throw new Error('Fixed +/-6 V rails changed or disconnected');
  const series = resistors.find((device) => connects(device, input, positive));
  const feedback = resistors.find((device) => connects(device, output, negative));
  const ground = resistors.find((device) => connects(device, negative, '0'));
  const load = resistors.find((device) => connects(device, output, '0'));
  if (!series || !feedback || !ground || !load || !connects(capacitors[0], positive, '0')) throw new Error('Unsupported non-inverting RC topology');
  if (load.value !== 10000) throw new Error('Fixed 10 kohm load changed');
  return { resistance: series.value, capacitance: capacitors[0].value, feedback: feedback.value, ground: ground.value, model: model.join('\n') };
}

// Closed-form transfer includes finite open-loop gain, output pole, and feedback/load loading.
export function referenceU2(circuit, frequency) {
  const beta = circuit.ground / (circuit.feedback + circuit.ground);
  const real = 1 + 1 / 10000 + 1e-9 + 1 / (circuit.feedback + circuit.ground) + 200000 * beta;
  const a = 2 * Math.PI * frequency * circuit.resistance * circuit.capacitance;
  const b = 2 * Math.PI * frequency * (200000 / (2 * Math.PI * 1e6));
  const denominatorReal = real - a * b, denominatorImaginary = b + a * real;
  const denominator = denominatorReal ** 2 + denominatorImaginary ** 2;
  return [200000 * denominatorReal / denominator, -200000 * denominatorImaginary / denominator];
}

export function settledPeak(rows) {
  if (rows.length < 2 || rows[0][0] > 0.05 || rows.at(-1)[0] < 0.1 - 1e-12) throw new Error('Incomplete transient window');
  let peak = 0, count = 0;
  for (let index = 0; index < rows.length; index++) {
    const [time, value] = rows[index];
    if (!Number.isFinite(time) || !Number.isFinite(value) || time < 0 || (index && (time <= rows[index - 1][0] || time - rows[index - 1][0] > 2.001e-6))) throw new Error('Invalid transient data or maximum timestep');
    if (time >= 0.05 && time <= 0.1) { peak = Math.max(peak, Math.abs(value)); count++; }
  }
  if (count < 25000) throw new Error('Insufficient settled samples');
  return peak;
}

export function evaluateU2(netlist, simulator) {
  const circuit = inspectU2(netlist);
  const bench = `Evaluator-owned U2\nVIN in 0 SIN(0 0.05 100) AC 1\nVP vcc 0 6\nVN vee 0 -6\nRS in filtered ${circuit.resistance}\nC1 filtered 0 ${circuit.capacitance}\nRF out feedback ${circuit.feedback}\nRG feedback 0 ${circuit.ground}\nRL out 0 10000\nX1 filtered feedback vcc vee out KESSETSU_OPAMP_V1\n${MODEL}\n.control\nset numdgt=15\nop\nwrdata op.data v(out)\nac dec 100 10 1000000\nwrdata ac.data v(out) v(in)\ntran 2u 100m 0 2u\nwrdata tran.data v(out)\nquit\n.endc\n.end\n`;
  const directory = mkdtempSync(join(tmpdir(), 'kessetsu-u2-'));
  try {
    writeFileSync(join(directory, 'bench.spice'), bench);
    const run = spawnSync(simulator, ['-n', '-b', 'bench.spice'], { cwd: directory, encoding: 'utf8', timeout: 30000, maxBuffer: 2 * 1024 * 1024, windowsHide: true });
    if (run.error || run.status !== 0) throw new Error(`Ngspice failed: ${run.error?.message ?? run.stderr}`);
    const raw = Object.fromEntries(['op', 'ac', 'tran'].map((name) => [name, readFileSync(join(directory, `${name}.data`), 'utf8')]));
    const op = parseRows(raw.op, 2), ac = parseRows(raw.ac, 6);
    if (op.length !== 1 || ac.length !== 501 || Math.abs(ac[0][0] - 10) > 1e-6 || Math.abs(ac.at(-1)[0] - 1e6) > 1) throw new Error('Incomplete OP/AC analyses');
    const at100 = ac.find((row) => Math.abs(row[0] - 100) < 1e-6);
    if (!at100) throw new Error('Missing 100 Hz sample');
    const gain = Math.hypot(at100[1], at100[2]) / Math.hypot(at100[4], at100[5]);
    const cutoff = cutoffFrequency(ac, gain);
    const peak = settledPeak(parseRows(raw.tran, 2));
    const formulaAgreement = ac.every((row) => {
      const expected = referenceU2(circuit, row[0]);
      return Math.abs(row[4] - 1) < 1e-9 && Math.abs(row[5]) < 1e-9 &&
        Math.hypot(row[1] - expected[0], row[2] - expected[1]) < 1e-7 * Math.max(1, Math.hypot(...expected));
    });
    const checks = { fixed_requirements: true, gain_100hz: gain >= 2.85 && gain <= 3.15,
      relative_cutoff: cutoff >= 1800 && cutoff <= 2200, dc_offset: Math.abs(op[0][1]) < 0.02,
      settled_peak: peak < 0.2, independent_formula_agreement: formulaAgreement,
      ac_transient_agreement: Math.abs(peak - 0.05 * gain) < 1e-5 };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks,
      measurements: { gain_100hz: gain, cutoff_hz: cutoff, dc_offset_v: op[0][1], settled_peak_v: peak },
      model_adequacy: { scope: 'nominal_generic_linear_simulation', hardware_validated: false, limitations, model_sha256: digest(MODEL) },
      evidence: { testbench: bench, op_data: raw.op, ac_data: raw.ac, transient_data: raw.tran, simulator_stdout: run.stdout, simulator_stderr: run.stderr } };
  } finally { rmSync(directory, { recursive: true, force: true }); } // Only this invocation's mkdtemp directory.
}

function main() {
  const [arm, candidatePath] = process.argv.slice(2);
  const record = { schema_version: 'kessetsu.u2-evaluation.v1', task: 'U2', arm, spec_sha256: digest(readFileSync(join(root, 'docs/evals/unseen-design-v1.md'))) };
  try {
    if (!['kessetsu', 'direct'].includes(arm) || !candidatePath) throw new Error('Usage: node scripts/evals/active-filter.mjs <kessetsu|direct> <candidate-file>');
    const candidate = readFileSync(resolve(candidatePath), 'utf8');
    Object.assign(record, { candidate_source: candidate, candidate_sha256: digest(candidate) });
    let netlist = candidate;
    if (arm === 'kessetsu') {
      const binary = process.env.KESSETSU_BINARY ?? join(root, 'core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
      const run = spawnSync(binary, ['compile', '-', '--format', 'json', '--include', 'spice'], { input: candidate, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
      record.compiler_stdout = run.stdout; record.compiler_stderr = run.stderr;
      if (run.error || run.status !== 0) throw new Error(`Compilation failed: ${run.error?.message ?? run.status}`);
      netlist = JSON.parse(run.stdout).debug?.spice_netlist;
      if (typeof netlist !== 'string') throw new Error('Compile output missing SPICE');
    }
    Object.assign(record, { compiled_netlist: netlist, netlist_sha256: digest(netlist) });
    const simulator = process.env.KESSETSU_NGSPICE ?? (process.platform === 'win32' ? join(root, 'core/tools/ngspice/bin/ngspice_con.exe') : 'ngspice');
    Object.assign(record, evaluateU2(netlist, simulator));
    process.exitCode = record.status === 'PASS' ? 0 : 1;
  } catch (error) { Object.assign(record, { status: 'ERROR', message: error.message }); process.exitCode = 2; }
  console.log(JSON.stringify(record));
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
