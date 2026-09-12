import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spiceNumber, parseRows } from './passive-filter.mjs';
import { digest, runBench, evaluatorMain } from './runtime.mjs';
import { verificationScope } from './verification-scope.mjs';

// Frozen generic model, identical for both arms; not sourced from candidate assertions.
export const MODEL = '.model 2N3904 NPN (Is=6.734f Xti=3 Eg=1.11 Vaf=74.03 Bf=416.4 Ne=1.259 Ise=6.734f Ikf=66.78m Xtb=1.5 Br=.7371 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=3.638p Mjc=.3085 Vjc=.75 Fc=.5 Cje=4.493p Mje=.2593 Vje=.75 Tr=239.5n Tf=301.2p Itf=.4 Vtf=4 Xtf=2 Rb=10)';
const normalize = (line) => line.trim().toLowerCase().replace(/\s+/g, ' ');

export function inspectU3(netlist) {
  const devices = [];
  let control = false, ended = false, modelFound = false;
  for (const original of netlist.replace(/^\uFEFF/, '').split(/\r?\n/).slice(1)) {
    const line = normalize(original);
    if (!line || line.startsWith('*')) continue;
    if (ended) throw new Error('Content after SPICE end');
    if (line === '.control') { if (control) throw new Error('Nested control'); control = true; continue; }
    if (line === '.endc') { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue;
    if (line === '.end') { ended = true; continue; }
    // Candidate analyses and presentation commands are inert: scoring always
    // uses the evaluator-owned testbench below.
    if (/^\.(op|ac|tran|meas|measure|print|plot|save)(\s|$)/.test(line)) continue;
    if (line.startsWith('.model')) {
      if (modelFound || line !== normalize(MODEL)) throw new Error('Missing/changed frozen 2N3904 model');
      modelFound = true; continue;
    }
    const fields = line.split(' '), kind = fields[0][0], count = kind === 'q' ? 3 : 2;
    if (!['r', 'c', 'v', 'q'].includes(kind)) throw new Error('Unsupported U3 device/directive');
    const nodes = fields.slice(1, 1 + count);
    if (nodes.length !== count || new Set(nodes).size !== count || nodes.some((node) => !/^[a-z0-9_]+$/.test(node))) throw new Error('Invalid/shorted nodes');
    if (devices.some((device) => device.name === fields[0])) throw new Error('Duplicate device');
    const device = { name: fields[0], kind, nodes };
    if (kind === 'q') {
      if (fields.length !== 5 || fields[4] !== '2n3904') throw new Error('Required 2N3904 changed');
    } else if (kind === 'v') {
      const tail = fields.slice(3).join(' ');
      const sine = /^sin(?:e)?\(\s*([^()]+)\)\s+ac\s+(\S+)$/.exec(tail);
      if (sine) {
        const values = sine[1].trim().split(/[\s,]+/).map(spiceNumber);
        if (values.length !== 3 || values[0] !== 0 || values[1] !== 0.005 || values[2] !== 1000 || spiceNumber(sine[2]) !== 1) throw new Error('Fixed 5 mV/1 kHz stimulus changed');
        device.signal = true;
      } else device.value = spiceNumber(tail.replace(/^dc\s+/, ''));
    } else {
      if (fields.length !== 4) throw new Error('Unsupported passive parameters');
      device.value = spiceNumber(fields[3]);
      if (!(device.value > 0)) throw new Error('Invalid passive value');
    }
    devices.push(device);
  }
  if (control || !modelFound) throw new Error('Incomplete netlist/model');
  const [resistors, capacitors, sources, transistors] = ['r', 'c', 'v', 'q'].map((kind) => devices.filter((device) => device.kind === kind));
  if (resistors.length !== 5 || capacitors.length !== 2 || sources.length !== 2 || transistors.length !== 1) throw new Error('Unsupported U3 topology: divider bias, unbypassed emitter resistor, input/output coupling');
  const [collector, base, emitter] = transistors[0].nodes;
  const signal = sources.find((device) => device.signal), supply = sources.find((device) => !device.signal);
  if (!signal || !supply || signal.nodes[1] !== '0') throw new Error('Missing fixed sources');
  const rail = supply.nodes[1] === '0' && supply.value === 9 ? supply.nodes[0] : supply.nodes[0] === '0' && supply.value === -9 ? supply.nodes[1] : null;
  if (!rail) throw new Error('Fixed 9 V supply changed');
  const input = signal.nodes[0];
  const connects = (device, a, b) => device.nodes.includes(a) && device.nodes.includes(b);
  const cin = capacitors.find((device) => connects(device, input, base));
  const cout = capacitors.find((device) => device.nodes.includes(collector));
  if (!cin || !cout || cin === cout) throw new Error('Required input/output coupling missing');
  const output = cout.nodes.find((node) => node !== collector);
  if (new Set(['0', rail, input, collector, base, emitter, output]).size !== 7) throw new Error('Shorted amplifier topology');
  const pairs = [[rail, collector], [emitter, '0'], [rail, base], [base, '0'], [output, '0']];
  const selected = pairs.map(([a, b]) => resistors.find((device) => connects(device, a, b)));
  if (selected.some((device) => !device) || new Set(selected).size !== 5) throw new Error('Unsupported bias/load topology');
  const [rc, re, top, bottom, load] = selected.map((device) => device.value);
  if (load !== 10000) throw new Error('Fixed 10 kohm load changed');
  return { rc, re, top, bottom, cin: cin.value, cout: cout.value };
}

// Trapezoidal Fourier integration with interpolated exact window boundaries.
export function fundamental(rows, start, end, frequency, maxStep = 2.001e-6) {
  if (rows.length < 2 || rows[0][0] > start || rows.at(-1)[0] < end - 1e-12) throw new Error('Incomplete transient window');
  if (!(end > start) || frequency <= 0 || Math.abs((end - start) * frequency - Math.round((end - start) * frequency)) > 1e-8) throw new Error('Window must contain complete periods');
  let sine = 0, cosine = 0;
  for (let i = 1; i < rows.length; i++) {
    const [t0, v0] = rows[i - 1], [t1, v1] = rows[i];
    if (![t0, t1, v0, v1].every(Number.isFinite) || t1 <= t0 || t1 - t0 > maxStep) throw new Error('Invalid transient axis/data');
    const a = Math.max(start, t0), b = Math.min(end, t1);
    if (b <= a) continue;
    const va = v0 + (v1 - v0) * (a - t0) / (t1 - t0), vb = v0 + (v1 - v0) * (b - t0) / (t1 - t0);
    const w = 2 * Math.PI * frequency;
    sine += (b - a) * (va * Math.sin(w * a) + vb * Math.sin(w * b)) / 2;
    cosine += (b - a) * (va * Math.cos(w * a) + vb * Math.cos(w * b)) / 2;
  }
  return 2 * Math.hypot(sine, cosine) / (end - start);
}

export function evaluateU3(netlist, simulator, runSimulator) {
  const c = inspectU3(netlist);
  const bench = `Evaluator-owned U3\nVIN in 0 SIN(0 0.005 1000) AC 1\nVP rail 0 9\nRC rail collector ${c.rc}\nRE emitter 0 ${c.re}\nRT rail base ${c.top}\nRB base 0 ${c.bottom}\nRL out 0 10000\nCI in base ${c.cin}\nCO collector out ${c.cout}\nQ1 collector base emitter 2N3904\n${MODEL}\n.control\nset numdgt=15\nsave all @q1[ic]\nop\nwrdata op.data v(collector) @q1[ic]\nac dec 100 10 1000000\nwrdata ac.data v(out) v(in)\ntran 2u 10m 0 2u\nwrdata tran.data v(out)\nquit\n.endc\n.end\n`;
  return runBench('U3', bench, simulator, ['op', 'ac', 'tran'], (raw) => {
    const op = parseRows(raw.op, 4), ac = parseRows(raw.ac, 6);
    if (op.length !== 1 || ac.length !== 501 || Math.abs(ac[0][0] - 10) > 1e-6 || Math.abs(ac.at(-1)[0] - 1e6) > 1) throw new Error('Incomplete OP/AC');
    ac.forEach((row, index) => { if (row[0] !== row[3] || (index && row[0] <= ac[index - 1][0]) || Math.abs(row[4] - 1) > 1e-9 || Math.abs(row[5]) > 1e-9) throw new Error('Invalid AC data'); });
    const sample = ac.find((row) => Math.abs(row[0] - 1000) < 1e-6);
    if (!sample) throw new Error('Missing 1 kHz sample');
    const gain = Math.hypot(sample[1], sample[2]), phase = Math.atan2(sample[2], sample[1]) * 180 / Math.PI;
    const amplitude = fundamental(parseRows(raw.tran, 2), 0.005, 0.01, 1000);
    const collector = op[0][1], current = op[0][3];
    const checks = { fixed_requirements: true, collector_bias: collector >= 3 && collector <= 6,
      collector_current: current >= 0.0005 && current <= 0.002, gain: gain >= 8 && gain <= 15,
      inversion: Math.abs(phase) >= 150, fundamental: amplitude >= 0.035 && amplitude <= 0.08,
      independent_dc_kcl: Math.abs((9 - collector) / c.rc - current) < 1e-8,
      small_signal_transient_agreement: Math.abs(amplitude / (0.005 * gain) - 1) < 0.02 };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks,
      measurements: { collector_v: collector, collector_current_a: current, gain_1khz: gain, phase_deg: phase, fundamental_v: amplitude },
      verification_scope: verificationScope('U3', checks),
      model_adequacy: { model: '2N3904', model_sha256: digest(MODEL), hardware_validated: false,
        limitations: ['Generic model, nominal operating point only; no tolerance/thermal or manufacturer-part guarantee.', 'Evaluator topology is limited to divider bias and an unbypassed emitter resistor.'] } };
  }, runSimulator);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) evaluatorMain('U3', evaluateU3, 2);
