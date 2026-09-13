import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spiceNumber, parseRows, cutoffFrequency } from './passive-filter.mjs';
import { fundamental } from './common-emitter.mjs';
import { digest, runBench, evaluatorMain } from './runtime.mjs';
import { verificationScope } from './verification-scope.mjs';

export const OFFICIAL_MODEL_SHA256 = 'fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5';
export const MODEL_PROVENANCE = {
  manufacturer: 'Texas Instruments', artifact: 'OPAx197 PSpice Model (Rev. D)', model_version: 'Final 1.3', model_date: '2022-06-23',
  product_url: 'https://www.ti.com/product/OPA197', download_url: 'https://www.ti.com/lit/zip/SBOMA34',
  file: 'OPAx197.LIB', file_sha256: OFFICIAL_MODEL_SHA256, redistribution_in_kessetsu: false,
  use_boundary: 'User-supplied local evaluation under TI terms; copyright/proprietary notices must be retained. Kessetsu does not redistribute the model.'
};
const normalize = (line) => line.trim().toLowerCase().replace(/\s+/g, ' ');

export function loadVerifiedModel(path, expectedHash = OFFICIAL_MODEL_SHA256) {
  if (!path) throw new Error('U6_MODEL_ACQUISITION_REQUIRED: set KESSETSU_U6_MODEL to the locally acquired official OPAx197.LIB');
  const model = readFileSync(resolve(path), 'utf8'), hash = digest(model);
  if (hash !== expectedHash) throw new Error(`U6_MODEL_INTEGRITY_ERROR: expected ${expectedHash}, got ${hash}`);
  if (!/^\s*\.subckt\s+opax197\s+in\+\s+in-\s+vcc\s+vee\s+out\s*$/im.test(model) || !/^\s*\.ends\s+opax197\s*$/im.test(model)) throw new Error('U6_MODEL_CAPABILITY_ERROR: expected OPAx197 five-pin subcircuit not found');
  return { text: model, sha256: hash };
}

export function inspectU6(netlist) {
  const devices = [];
  let control = false, ended = false;
  for (const original of netlist.replace(/^\uFEFF/, '').split(/\r?\n/).slice(1)) {
    const line = normalize(original);
    if (!line || line.startsWith('*')) continue;
    if (ended) throw new Error('Content after SPICE end');
    if (line === '.control') { if (control) throw new Error('Nested control'); control = true; continue; }
    if (line === '.endc') { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue;
    if (line === '.end') { ended = true; continue; }
    // Candidate analyses and presentation commands are inert: scoring always
    // uses the evaluator-owned OP/AC/transient testbench and exact local model.
    if (/^\.(op|ac|tran|meas|measure|four|print|plot|save)(\s|$)/.test(line)) continue;
    const fields = line.split(' '), kind = fields[0][0], count = kind === 'x' ? 5 : 2;
    if (!['r', 'v', 'x'].includes(kind)) throw new Error('Unsupported U6 device/directive; model inclusion belongs to the evaluator');
    const nodes = fields.slice(1, 1 + count);
    if (nodes.length !== count || nodes.some((node) => !/^[a-z0-9_]+$/.test(node))) throw new Error('Invalid node');
    if (devices.some((d) => d.name === fields[0])) throw new Error('Duplicate device');
    const d = { name: fields[0], kind, nodes };
    if (kind === 'x') {
      if (fields.length !== 7 || fields[6] !== 'opax197') throw new Error('Authentic OPAx197 instance required');
    } else if (kind === 'v') {
      const tail = fields.slice(3).join(' '), sine = /^sin(?:e)?\(\s*([^()]+)\)\s+ac\s+(\S+)$/.exec(tail);
      if (sine) {
        const values = sine[1].trim().split(/[\s,]+/).map(spiceNumber);
        if (values.length !== 3 || values[0] !== 0 || values[1] !== 0.1 || values[2] !== 1000 || spiceNumber(sine[2]) !== 1) throw new Error('Fixed U6 stimulus changed');
        d.signal = true;
      } else d.value = spiceNumber(tail.replace(/^dc\s+/, ''));
    } else {
      if (fields.length !== 4) throw new Error('Unsupported resistor parameters');
      d.value = spiceNumber(fields[3]); if (!(d.value > 0)) throw new Error('Invalid resistance');
    }
    devices.push(d);
  }
  if (control) throw new Error('Unterminated control');
  const sources = devices.filter((d) => d.kind === 'v'), resistors = devices.filter((d) => d.kind === 'r'), amps = devices.filter((d) => d.kind === 'x');
  if (sources.length !== 3 || resistors.length !== 3 || amps.length !== 1) throw new Error('U6 requires one non-inverting OPAx197 and three fixed/feedback resistors');
  const amp = amps[0], [plus, minus, vcc, vee, output] = amp.nodes;
  const signal = sources.find((d) => d.signal), positive = sources.find((d) => !d.signal && d.nodes[1] === '0' && d.value === 6),
    negative = sources.find((d) => !d.signal && d.nodes[0] === '0' && d.value === 6);
  if (!signal || !positive || !negative || signal.nodes[1] !== '0' || signal.nodes[0] !== plus || positive.nodes[0] !== vcc || negative.nodes[1] !== vee) throw new Error('Fixed U6 source/supplies changed');
  const connects = (d, a, b) => d.nodes.includes(a) && d.nodes.includes(b);
  const load = resistors.find((d) => connects(d, output, '0')), rf = resistors.find((d) => connects(d, output, minus)), rg = resistors.find((d) => connects(d, minus, '0'));
  if (!load || !rf || !rg || new Set([load, rf, rg]).size !== 3 || load.value !== 10000) throw new Error('Fixed load or non-inverting feedback topology changed');
  return { rf: rf.value, rg: rg.value };
}

export function evaluateU6(netlist, simulator, runSimulator, modelPath = process.env.KESSETSU_U6_MODEL, expectedHash = OFFICIAL_MODEL_SHA256) {
  const c = inspectU6(netlist), model = loadVerifiedModel(modelPath, expectedHash);
  const bench = `Evaluator-owned U6\n.param TEMP=27\nVIN in 0 SIN(0 0.1 1000) AC 1\nVP vcc 0 6\nVN 0 vee 6\nRF out feedback ${c.rf}\nRG feedback 0 ${c.rg}\nRL out 0 10000\nX1 in feedback vcc vee out OPAx197\n.include OPAx197.LIB\n.control\nset numdgt=15\nop\nwrdata op.data v(out)\nac dec 100 10 1000000\nwrdata ac.data v(out) v(in)\ntran 2u 10m 0 2u\nwrdata tran.data v(out)\nquit\n.endc\n.end\n`;
  const evidenceBench = `${bench}* External OPAx197 model omitted from record; sha256=${model.sha256}\n`;
  try { return runBench('U6', bench, { executable: simulator, args: ['-D', 'ngbehavior=ps'] }, ['op', 'ac', 'tran'], (raw) => {
    const op = parseRows(raw.op, 2), ac = parseRows(raw.ac, 6), transient = parseRows(raw.tran, 2);
    if (op.length !== 1 || ac.length !== 501) throw new Error('Incomplete U6 OP/AC data');
    const sample = ac.find((row) => Math.abs(row[0] - 1000) < 1e-6);
    if (!sample || sample[0] !== sample[3] || Math.abs(sample[4] - 1) > 1e-9 || Math.abs(sample[5]) > 1e-9) throw new Error('Missing/invalid 1 kHz AC sample');
    // Enforce the complete AC sweep and a conventional low-pass crossing when present.
    if (Math.abs(ac[0][0] - 10) > 1e-6 || Math.abs(ac.at(-1)[0] - 1e6) > 1) throw new Error('Incomplete U6 AC sweep');
    const gain = Math.hypot(sample[1], sample[2]), amplitude = fundamental(transient, 0.005, 0.01, 1000);
    const peak = transient.filter((row) => row[0] >= 0.005 && row[0] <= 0.01).reduce((max, row) => Math.max(max, Math.abs(row[1])), 0);
    let cutoff = null; try { cutoff = cutoffFrequency(ac, gain); } catch { /* Not an acceptance condition. */ }
    const checks = { fixed_requirements: true, gain_1khz: gain >= 4.75 && gain <= 5.25,
      settled_peak: peak >= 0.475 && peak <= 0.525, transient_fundamental: amplitude >= 0.475 && amplitude <= 0.525 };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks,
      measurements: { gain_1khz: gain, settled_peak_v: peak, fundamental_v: amplitude, dc_offset_v: op[0][1], cutoff_hz: cutoff },
      verification_scope: verificationScope('U6', checks),
      model_provenance: { ...MODEL_PROVENANCE, supplied_file_sha256: model.sha256 },
      model_adequacy: { authentic_manufacturer_model: model.sha256 === OFFICIAL_MODEL_SHA256 && expectedHash === OFFICIAL_MODEL_SHA256, hardware_validated: false,
        limitations: ['Manufacturer macromodel simulation does not replace tolerance, board, thermal, EMC, or bench validation.'] } };
  }, runSimulator, evidenceBench, { 'OPAx197.LIB': model.text }); } catch (error) {
    error.evidence.model_provenance = { ...MODEL_PROVENANCE, supplied_file_sha256: model.sha256 };
    if (model.sha256 === OFFICIAL_MODEL_SHA256 && /no such function 'if'|mif-error|fatal error/i.test(error.evidence?.simulator_stderr ?? '')) {
      error.message = 'U6_MODEL_CAPABILITY_ERROR: selected simulator lacks the PSpice/XSPICE support required by the official OPAx197 Rev. D model';
    }
    throw error;
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) evaluatorMain('U6', evaluateU6, 2);
