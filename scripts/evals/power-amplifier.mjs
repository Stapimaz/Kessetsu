import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spiceNumber, parseRows } from './passive-filter.mjs';
import { fundamental } from './common-emitter.mjs';
import { MODEL as OPAMP_MODEL, inspectModel } from './active-filter.mjs';
import { digest, runBench, evaluatorMain } from './runtime.mjs';
import { verificationScope } from './verification-scope.mjs';

export const NPN_MODEL = '.model KESSETSU_POWER_NPN_V1 NPN (Is=1e-12 Bf=80 Vaf=60 Cje=300p Cjc=150p Tf=1u Tr=5u)';
export const PNP_MODEL = '.model KESSETSU_POWER_PNP_V1 PNP (Is=1e-12 Bf=80 Vaf=60 Cje=300p Cjc=150p Tf=1u Tr=5u)';
const normalize = (line) => line.trim().toLowerCase().replace(/\s+/g, ' ');

export function inspectU5(netlist) {
  const devices = [], subcircuit = [], modelLines = new Set();
  let control = false, inSub = false, ended = false;
  for (const original of netlist.replace(/^\uFEFF/, '').split(/\r?\n/).slice(1)) {
    const line = normalize(original);
    if (!line || line.startsWith('*')) continue;
    if (ended) throw new Error('Content after SPICE end');
    if (line === '.control') { if (control || inSub) throw new Error('Invalid control'); control = true; continue; }
    if (line === '.endc') { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue;
    if (line.startsWith('.subckt')) { if (inSub || subcircuit.length) throw new Error('Unsupported extra subcircuit'); inSub = true; }
    if (inSub) { subcircuit.push(line); if (line.startsWith('.ends')) inSub = false; continue; }
    if (line === '.end') { ended = true; continue; }
    if (/^\.(op|ac|tran|meas|measure)(\s|$)/.test(line)) continue;
    if (line.startsWith('.model')) { modelLines.add(line); continue; }
    const fields = line.split(' '), kind = fields[0][0], count = kind === 'q' ? 3 : kind === 'x' ? 5 : 2;
    if (!['r', 'c', 'v', 'q', 'x'].includes(kind)) throw new Error('Unsupported U5 device/directive');
    const nodes = fields.slice(1, 1 + count);
    if (nodes.length !== count || nodes.some((node) => !/^[a-z0-9_]+$/.test(node))) throw new Error('Invalid node');
    if (devices.some((d) => d.name === fields[0])) throw new Error('Duplicate device');
    const d = { name: fields[0], kind, nodes };
    if (kind === 'q') {
      if (fields.length !== 5 || !['kessetsu_power_npn_v1', 'kessetsu_power_pnp_v1'].includes(fields[4])) throw new Error('Unsupported output transistor');
      d.model = fields[4];
    } else if (kind === 'x') {
      if (fields.length !== 7 || fields[6] !== 'kessetsu_opamp_v1') throw new Error('Unsupported U5 driver model');
    } else if (kind === 'v') {
      const tail = fields.slice(3).join(' '), sine = /^sin(?:e)?\(\s*([^()]+)\)\s+ac\s+(\S+)$/.exec(tail);
      if (sine) {
        const values = sine[1].trim().split(/[\s,]+/).map(spiceNumber);
        if (values.length !== 3 || values[0] !== 0 || values[1] !== 0.1 || values[2] !== 1000 || spiceNumber(sine[2]) !== 1) throw new Error('Fixed U5 stimulus changed');
        d.signal = true;
      } else d.value = spiceNumber(tail.replace(/^dc\s+/, ''));
    } else {
      if (fields.length !== 4) throw new Error('Unsupported passive parameters');
      d.value = spiceNumber(fields[3]);
      if (!(d.value > 0)) throw new Error('Invalid passive value');
    }
    devices.push(d);
  }
  if (control || inSub) throw new Error('Incomplete control/model block');
  const opamp = inspectModel(subcircuit.join('\n'));
  if (opamp.name !== 'kessetsu_opamp_v1') throw new Error('Missing/changed generic op-amp model');
  if (modelLines.size !== 2 || !modelLines.has(normalize(NPN_MODEL)) || !modelLines.has(normalize(PNP_MODEL))) throw new Error('Missing/changed power-transistor model');
  const byKind = (kind) => devices.filter((d) => d.kind === kind);
  const sources = byKind('v'), resistors = byKind('r'), transistors = byKind('q'), amps = byKind('x');
  if (sources.length !== 3 || transistors.length !== 2 || amps.length !== 3 || byKind('c').length !== 0 || resistors.length !== 3) throw new Error('Bounded U5 evaluator expects buffer, voltage-gain, error-driver, and complementary output');
  const signal = sources.find((d) => d.signal), positive = sources.find((d) => !d.signal && d.nodes[1] === '0' && d.value === 9),
    negative = sources.find((d) => !d.signal && d.nodes[0] === '0' && d.value === 9);
  if (!signal || !positive || !negative || signal.nodes[1] !== '0') throw new Error('Fixed input or +/-9 V supplies changed');
  const [input, vcc, vee] = [signal.nodes[0], positive.nodes[0], negative.nodes[1]];
  const qn = transistors.find((d) => d.model === 'kessetsu_power_npn_v1'), qp = transistors.find((d) => d.model === 'kessetsu_power_pnp_v1');
  if (!qn || !qp || qn.nodes[0] !== vcc || qp.nodes[0] !== vee || qn.nodes[1] !== qp.nodes[1] || qn.nodes[2] !== qp.nodes[2]) throw new Error('Complementary output stage invalid');
  const [drive, output] = [qn.nodes[1], qn.nodes[2]];
  if (new Set(['0', input, vcc, vee, drive, output]).size !== 6) throw new Error('Shorted fixed/output-stage nodes');
  const connects = (d, a, b) => d.nodes.includes(a) && d.nodes.includes(b);
  const load = resistors.find((d) => connects(d, output, '0'));
  if (!load || load.value !== 4) throw new Error('Fixed 4 ohm load changed');
  if (amps.some((d) => d.nodes[2] !== vcc || d.nodes[3] !== vee)) throw new Error('Op-amp supply pins changed');
  const driver = amps.find((d) => d.nodes[4] === drive && d.nodes[1] === output);
  if (!driver) throw new Error('Output-feedback driver stage missing');
  const gain = amps.find((d) => d.nodes[4] === driver.nodes[0] && d !== driver);
  if (!gain) throw new Error('Voltage-gain stage does not drive output driver');
  const rf = resistors.find((d) => connects(d, gain.nodes[4], gain.nodes[1]));
  const rg = resistors.find((d) => connects(d, gain.nodes[1], '0'));
  if (!rf || !rg || new Set([load, rf, rg]).size !== 3) throw new Error('Voltage-gain feedback divider missing');
  const buffer = amps.find((d) => d !== gain && d !== driver);
  if (!buffer || buffer.nodes[0] !== input || buffer.nodes[1] !== buffer.nodes[4] || gain.nodes[0] !== buffer.nodes[4]) throw new Error('Input buffer/gain signal path invalid');
  if (new Set(['0', input, vcc, vee, drive, output, gain.nodes[4], gain.nodes[1], buffer.nodes[4]]).size !== 9) throw new Error('Shorted gain/feedback signal path');
  return { rf: rf.value, rg: rg.value };
}

function interpolate(a, b, time, column) { return a[column] + (b[column] - a[column]) * (time - a[0]) / (b[0] - a[0]); }
export function measureU5(rows) {
  if (rows.length < 2 || rows[0][0] > 0.01 || rows.at(-1)[0] < 0.03 - 1e-12) throw new Error('Incomplete U5 measurement window');
  let energy = 0, npn = 0, pnp = 0, duration = 0;
  for (let i = 1; i < rows.length; i++) {
    const a = rows[i - 1], b = rows[i];
    if (a.length !== 4 || b.length !== 4 || [...a, ...b].some((v) => !Number.isFinite(v)) || b[0] <= a[0] || b[0] - a[0] > 2.001e-6) throw new Error('Invalid U5 transient data');
    const start = Math.max(0.01, a[0]), end = Math.min(0.03, b[0]);
    if (end <= start) continue;
    const dt = end - start, v0 = interpolate(a, b, start, 1), v1 = interpolate(a, b, end, 1),
      n0 = interpolate(a, b, start, 2), n1 = interpolate(a, b, end, 2), p0 = interpolate(a, b, start, 3), p1 = interpolate(a, b, end, 3);
    energy += dt * (v0 ** 2 + v1 ** 2) / 2;
    npn += dt * (Math.max(0, (9 - v0) * n0) + Math.max(0, (9 - v1) * n1)) / 2;
    pnp += dt * (Math.max(0, (-9 - v0) * p0) + Math.max(0, (-9 - v1) * p1)) / 2;
    duration += dt;
  }
  if (Math.abs(duration - 0.02) > 2e-9) throw new Error('Incomplete integration duration');
  const amplitudes = Array.from({ length: 10 }, (_, index) => fundamental(rows.map((row) => [row[0], row[1]]), 0.01, 0.03, (index + 1) * 1000));
  const thd = Math.sqrt(amplitudes.slice(1).reduce((sum, value) => sum + value ** 2, 0)) / amplitudes[0];
  return { output_power_w: energy / duration / 4, thd, npn_dissipation_w: npn / duration, pnp_dissipation_w: pnp / duration,
    fundamental_v: amplitudes[0], harmonics_v: amplitudes.slice(1) };
}

export function evaluateU5(netlist, simulator, runSimulator) {
  const c = inspectU5(netlist);
  const bench = `Evaluator-owned U5\nVP vcc 0 9\nVN 0 vee 9\nVIN in 0 SIN(0 0.1 1000) AC 1\nX1 in buf vcc vee buf KESSETSU_OPAMP_V1\nX2 buf feedback vcc vee gain KESSETSU_OPAMP_V1\nRF gain feedback ${c.rf}\nRG feedback 0 ${c.rg}\nX3 gain out vcc vee drive KESSETSU_OPAMP_V1\nQN vcc drive out KESSETSU_POWER_NPN_V1\nQP vee drive out KESSETSU_POWER_PNP_V1\nRL out 0 4\n${OPAMP_MODEL}\n${NPN_MODEL}\n${PNP_MODEL}\n.control\nset numdgt=15\nsave all @qn[ic] @qp[ic]\ntran 2u 30m 0 2u\nwrdata tran.data v(out) @qn[ic] @qp[ic]\nquit\n.endc\n.end\n`;
  return runBench('U5', bench, simulator, ['tran'], (raw) => {
    const rawRows = parseRows(raw.tran, 6), rows = rawRows.map((r) => {
      if (r[0] !== r[2] || r[0] !== r[4]) throw new Error('Mismatched U5 transient axes');
      return [r[0], r[1], r[3], r[5]];
    });
    const measurements = measureU5(rows);
    const checks = { fixed_requirements: true, output_power: measurements.output_power_w >= 0.9 && measurements.output_power_w <= 1.1,
      thd: measurements.thd < 0.03, npn_dissipation: measurements.npn_dissipation_w < 2,
      pnp_dissipation: measurements.pnp_dissipation_w < 2 };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks, measurements,
      verification_scope: verificationScope('U5', checks),
      driver_power: { status: 'UNAVAILABLE', reason: 'The generic op-amp template exposes no supply-current branches; total efficiency is not claimed.' },
      model_adequacy: { hardware_validated: false, opamp_model_sha256: digest(OPAMP_MODEL), npn_model_sha256: digest(NPN_MODEL), pnp_model_sha256: digest(PNP_MODEL),
        limitations: ['Generic linear op-amp omits rail saturation, output-current limits, and supply current.', 'Generic power-BJT models omit tolerance, thermal/package/SOA validation.', 'Nominal simulation is not a hardware efficiency or manufacturability claim.'] } };
  }, runSimulator);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) evaluatorMain('U5', evaluateU5);
