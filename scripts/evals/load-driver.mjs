import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spiceNumber, parseRows } from './passive-filter.mjs';
import { digest, runBench, evaluatorMain } from './runtime.mjs';
import { verificationScope } from './verification-scope.mjs';

export const MODEL = '.model IRF540 VDMOS (Rg=3 Vto=4.0 Rd=45m Rs=12m Rb=10m Kp=18 Cgdmax=2n Cgdmin=1.3n Cgs=1.7n Cjo=1n Is=2p mfg=IR)';
const normalize = (line) => line.trim().toLowerCase().replace(/\s+/g, ' ');

export function inspectU4(netlist) {
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
    if (/^\.(op|tran|meas|measure)(\s|$)/.test(line)) continue;
    if (line.startsWith('.model')) {
      if (modelFound || line !== normalize(MODEL)) throw new Error('Missing/changed frozen IRF540 model');
      modelFound = true; continue;
    }
    const fields = line.split(' '), kind = fields[0][0], count = kind === 'm' ? 4 : 2;
    if (!['r', 'v', 'm'].includes(kind)) throw new Error('Unsupported U4 device/directive');
    const nodes = fields.slice(1, 1 + count);
    if (nodes.length !== count || nodes.some((node) => !/^[a-z0-9_]+$/.test(node))) throw new Error('Invalid nodes');
    if (devices.some((device) => device.name === fields[0])) throw new Error('Duplicate device');
    const device = { name: fields[0], kind, nodes };
    if (kind === 'm') {
      if (fields.length !== 6 || fields[5] !== 'irf540' || nodes[2] !== nodes[3]) throw new Error('Required three-terminal IRF540 changed');
    } else if (kind === 'v') {
      const tail = fields.slice(3).join(' ');
      const pulse = /^pulse\(\s*([^()]+)\)$/.exec(tail);
      if (pulse) {
        const values = pulse[1].trim().split(/[\s,]+/).map(spiceNumber);
        if (values.length !== 7 || values.some((value, i) => Math.abs(value - [0, 10, 0, 1e-6, 1e-6, 500e-6, 1e-3][i]) > 1e-12)) throw new Error('Fixed U4 pulse changed');
        device.signal = true;
      } else device.value = spiceNumber(tail.replace(/^dc\s+/, ''));
    } else {
      if (fields.length !== 4) throw new Error('Unsupported resistor parameters');
      device.value = spiceNumber(fields[3]);
      if (!(device.value > 0)) throw new Error('Invalid resistance');
    }
    devices.push(device);
  }
  if (control || !modelFound) throw new Error('Incomplete netlist/model');
  const resistors = devices.filter((d) => d.kind === 'r'), sources = devices.filter((d) => d.kind === 'v'), mosfets = devices.filter((d) => d.kind === 'm');
  if (sources.length !== 2 || mosfets.length !== 1 || resistors.length < 1 || resistors.length > 3) throw new Error('Unsupported U4 topology');
  const [drain, gate, source] = mosfets[0].nodes;
  if (source !== '0' || new Set(['0', drain, gate]).size !== 3) throw new Error('MOSFET source/body must be grounded');
  const supply = sources.find((d) => !d.signal), signal = sources.find((d) => d.signal);
  if (!supply || !signal || supply.nodes[1] !== '0' || supply.value !== 12) throw new Error('Fixed 12 V supply changed');
  const rail = supply.nodes[0], input = signal.nodes[0];
  if (signal.nodes[1] !== '0' || new Set(['0', rail, drain, gate, input]).size !== (input === gate ? 4 : 5)) throw new Error('Invalid gate/supply connection');
  const connects = (d, a, b) => d.nodes.includes(a) && d.nodes.includes(b);
  const load = resistors.find((d) => connects(d, rail, drain));
  if (!load || load.value !== 120) throw new Error('Fixed 120 ohm load changed');
  const gateParts = resistors.filter((d) => d !== load);
  const seriesParts = gateParts.filter((d) => connects(d, input, gate));
  const pullDownParts = gateParts.filter((d) => connects(d, gate, '0'));
  if (input !== gate) {
    if (seriesParts.length !== 1) throw new Error('Gate is not driven through exactly one resistor');
  } else if (gateParts.some((d) => d.nodes.includes(input) && !connects(d, input, '0'))) throw new Error('Unsupported gate network');
  if (gateParts.some((d) => !(connects(d, input, gate) || connects(d, gate, '0')))) throw new Error('Unsupported gate resistor');
  if (seriesParts.length > 1 || pullDownParts.length > 1 || new Set([...seriesParts, ...pullDownParts]).size !== gateParts.length) throw new Error('Duplicate/ambiguous gate resistor');
  return { gateSeries: seriesParts[0]?.value ?? null, gatePullDown: pullDownParts[0]?.value ?? null };
}

function windowIntervals() {
  const result = [];
  for (let period = 6; period < 10; period++) {
    const base = period / 1000;
    result.push({ state: 'on', start: base + 50e-6, end: base + 450e-6 });
    result.push({ state: 'off', start: base + 550e-6, end: base + 950e-6 });
  }
  return result;
}

export function settledMetrics(rows) {
  if (rows.length < 2 || rows[0][0] > 0.00605 || rows.at(-1)[0] < 0.00995 - 1e-12) throw new Error('Incomplete U4 window');
  let onTime = 0, offTime = 0, onCurrent = 0, offMax = 0, drainMax = 0, energy = 0, acceptedTime = 0;
  for (let i = 1; i < rows.length; i++) {
    const a = rows[i - 1], b = rows[i];
    if (a.length !== 3 || b.length !== 3 || [...a, ...b].some((x) => !Number.isFinite(x)) || b[0] <= a[0] || b[0] - a[0] > 2.001e-6) throw new Error('Invalid U4 transient data');
    for (const window of windowIntervals()) {
      const start = Math.max(a[0], window.start), end = Math.min(b[0], window.end);
      if (end <= start) continue;
      const sample = (row, time) => row[1] + (b[1] - a[1]) * (time - a[0]) / (b[0] - a[0]);
      const vd0 = sample(a, start), vd1 = sample(a, end);
      const id0 = a[2] + (b[2] - a[2]) * (start - a[0]) / (b[0] - a[0]);
      const id1 = a[2] + (b[2] - a[2]) * (end - a[0]) / (b[0] - a[0]);
      const dt = end - start, il0 = (12 - vd0) / 120, il1 = (12 - vd1) / 120;
      // The VDMOS terminal current includes displacement-current ripple at the
      // evaluator's fixed timestep; one milliamp is still a conservative KCL guard.
      if (Math.max(Math.abs(il0 - id0), Math.abs(il1 - id1)) > 1e-3) throw new Error('Drain-current/load KCL disagreement');
      energy += dt * (Math.max(0, vd0 * id0) + Math.max(0, vd1 * id1)) / 2; acceptedTime += dt;
      if (window.state === 'on') { onCurrent += dt * (il0 + il1) / 2; onTime += dt; drainMax = Math.max(drainMax, vd0, vd1); }
      else { offMax = Math.max(offMax, Math.abs(il0), Math.abs(il1)); offTime += dt; }
    }
  }
  if (Math.abs(onTime - 0.0016) > 2e-9 || Math.abs(offTime - 0.0016) > 2e-9) throw new Error('Incomplete settled intervals');
  return { on_current_a: onCurrent / onTime, off_current_peak_a: offMax, on_drain_peak_v: drainMax, mean_dissipation_w: energy / acceptedTime };
}

export function evaluateU4(netlist, simulator, runSimulator) {
  const c = inspectU4(netlist);
  const gate = c.gateSeries ? `RG in gate ${c.gateSeries}\n` : '';
  const pull = c.gatePullDown ? `RPD gate 0 ${c.gatePullDown}\n` : '';
  const gateNode = c.gateSeries ? 'gate' : 'in';
  const bench = `Evaluator-owned U4\nVG in 0 PULSE(0 10 0 1u 1u 500u 1m)\nVP rail 0 12\nRL rail drain 120\n${gate}${pull}M1 drain ${gateNode} 0 0 IRF540\n${MODEL}\n.control\nset numdgt=15\nsave all @m1[id]\ntran 2u 10m 0 2u\nwrdata tran.data v(drain) @m1[id]\nquit\n.endc\n.end\n`;
  return runBench('U4', bench, simulator, ['tran'], (raw) => {
    const rows = parseRows(raw.tran, 4).map((row) => {
      if (row[0] !== row[2]) throw new Error('Mismatched transient axes');
      return [row[0], row[1], row[3]];
    });
    const measurements = settledMetrics(rows);
    const checks = { fixed_requirements: true, on_current: measurements.on_current_a >= 0.095,
      off_current: measurements.off_current_peak_a <= 100e-6, on_drain: measurements.on_drain_peak_v <= 0.6,
      mean_dissipation: measurements.mean_dissipation_w < 0.1 };
    return { status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks, measurements,
      verification_scope: verificationScope('U4', checks),
      model_adequacy: { model: 'IRF540', model_sha256: digest(MODEL), hardware_validated: false,
        limitations: ['Generic nominal VDMOS model; no tolerance, thermal, package, avalanche, or gate-driver validation.', 'Settled-window dissipation intentionally excludes 50 us around switching transitions and therefore is not total switching loss.'] } };
  }, runSimulator);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) evaluatorMain('U4', evaluateU4);
