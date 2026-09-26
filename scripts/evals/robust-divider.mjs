// Independent M1 evaluator. Candidate control blocks and candidate-generated PASS
// claims are never executed or trusted for the electrical decision.
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { digest, root } from './runtime.mjs';
import { spiceNumber } from './passive-filter.mjs';

const SPEC = 'scripts/evals/specs/agent-workflow-v2.md';
const E24 = Object.freeze([1.0, 1.1, 1.2, 1.3, 1.5, 1.6, 1.8, 2.0, 2.2, 2.4, 2.7, 3.0,
  3.3, 3.6, 3.9, 4.3, 4.7, 5.1, 5.6, 6.2, 6.8, 7.5, 8.2, 9.1]);

function isE24(value) {
  if (!Number.isFinite(value) || value <= 0) return false;
  const decade = 10 ** Math.floor(Math.log10(value));
  const normalized = value / decade;
  return E24.some((entry) => Math.abs(normalized - entry) < 1e-10);
}

export function inspectDivider(netlist) {
  const devices = [];
  let control = false;
  const lines = netlist.replace(/^\uFEFF/, '').split(/\r?\n/);
  for (const original of lines.slice(1)) {
    const line = original.trim();
    if (!line || line.startsWith('*')) continue;
    if (/^\.control$/i.test(line)) { if (control) throw new Error('Nested control block'); control = true; continue; }
    if (/^\.endc$/i.test(line)) { if (!control) throw new Error('Unmatched endc'); control = false; continue; }
    if (control) continue;
    if (/^\./.test(line)) continue;
    const fields = line.split(/\s+/);
    const kind = fields[0][0].toUpperCase();
    if (!['R', 'V'].includes(kind) || fields.length < 4) throw new Error(`Unsupported M1 device: ${line}`);
    if (devices.some((device) => device.name.toLowerCase() === fields[0].toLowerCase())) throw new Error('Duplicate device');
    const nodes = fields.slice(1, 3).map((node) => node.toLowerCase());
    if (nodes.some((node) => !/^[a-z0-9_]+$/.test(node)) || nodes[0] === nodes[1]) throw new Error('Invalid or shorted device nodes');
    const valueText = kind === 'V' && fields[3].toUpperCase() === 'DC' ? fields[4] : fields[3];
    const value = spiceNumber(valueText);
    if (!(value > 0)) throw new Error('M1 values must be positive');
    devices.push({ kind, name: fields[0], nodes, value });
  }
  if (control) throw new Error('Unterminated control block');
  const sources = devices.filter((device) => device.kind === 'V');
  const resistors = devices.filter((device) => device.kind === 'R');
  if (sources.length !== 1 || resistors.length !== 3) throw new Error('M1 requires exactly one source and three resistors');
  const source = sources[0];
  if (source.value !== 12 || source.nodes[1] !== '0') throw new Error('M1 requires one 12 V source referenced to ground');
  const input = source.nodes[0];
  const upper = resistors.find((device) => device.nodes.includes(input) && !device.nodes.includes('0'));
  if (!upper) throw new Error('Missing upper divider resistor');
  const output = upper.nodes.find((node) => node !== input);
  const shunts = resistors.filter((device) => device.nodes.includes(output) && device.nodes.includes('0'));
  if (shunts.length !== 2 || resistors.some((device) => device !== upper && !shunts.includes(device))) {
    throw new Error('M1 requires exactly two OUT-to-ground shunt resistors');
  }
  const loads = shunts.filter((device) => device.value === 100000);
  if (loads.length !== 1) throw new Error('M1 requires one unambiguous fixed 100 kohm load');
  const load = loads[0];
  const lower = shunts.find((device) => device !== load);
  if (!isE24(upper.value) || !isE24(lower.value)
      || upper.value < 1000 || upper.value > 100000
      || lower.value < 1000 || lower.value > 100000) {
    throw new Error('Upper and lower resistors must be E24 values from 1 kohm through 100 kohm');
  }
  return { upper: upper.value, lower: lower.value, load: load.value };
}

export function evaluateCorners(circuit) {
  const cases = [];
  const deviations = [{ name: 'nominal', upper: 0, lower: 0, load: 0 }];
  for (const upper of [-0.01, 0.01]) for (const lower of [-0.01, 0.01]) for (const load of [-0.10, 0.10]) {
    deviations.push({ name: `u${upper > 0 ? '+' : '-'}_l${lower > 0 ? '+' : '-'}_load${load > 0 ? '+' : '-'}`, upper, lower, load });
  }
  for (const supply of [10.8, 12, 13.2]) for (const deviation of deviations) {
    const upper = circuit.upper * (1 + deviation.upper);
    const lower = circuit.lower * (1 + deviation.lower);
    const load = circuit.load * (1 + deviation.load);
    const shunt = lower * load / (lower + load);
    const current = supply / (upper + shunt);
    const output = current * shunt;
    const upperPower = current * current * upper;
    const lowerPower = output * output / lower;
    const checks = {
      output_range: output >= 2.75 && output <= 3.60,
      source_current: current <= 350e-6,
      upper_power: upperPower < 4e-3,
      lower_power: lowerPower < 4e-3,
    };
    cases.push({ id: `${supply}V_${deviation.name}`, supply_v: supply, upper_ohm: upper,
      lower_ohm: lower, load_ohm: load, output_v: output, source_current_a: current,
      upper_power_w: upperPower, lower_power_w: lowerPower,
      status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks });
  }
  const checks = {
    case_count: cases.length === 27,
    output_range: cases.every((item) => item.checks.output_range),
    source_current: cases.every((item) => item.checks.source_current),
    upper_power: cases.every((item) => item.checks.upper_power),
    lower_power: cases.every((item) => item.checks.lower_power),
  };
  return {
    status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL',
    checks,
    cases,
    measurements: {
      output_min_v: Math.min(...cases.map((item) => item.output_v)),
      output_max_v: Math.max(...cases.map((item) => item.output_v)),
      source_current_max_a: Math.max(...cases.map((item) => item.source_current_a)),
      upper_power_max_w: Math.max(...cases.map((item) => item.upper_power_w)),
      lower_power_max_w: Math.max(...cases.map((item) => item.lower_power_w)),
    },
  };
}

function parseUtf8Json(path) {
  return JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
}

function hasNonEmptyFile(directory, names) {
  return names.some((name) => {
    const path = join(directory, name);
    return existsSync(path) && statSync(path).size > 0;
  });
}

export function inspectWorkflow(candidatePath, arm) {
  const directory = dirname(candidatePath);
  const resultPath = join(directory, 'study-results.json');
  let study = { present: false, valid: false, case_count: 0, error: null };
  if (existsSync(resultPath)) {
    try {
      const parsed = parseUtf8Json(resultPath);
      const cases = Array.isArray(parsed.cases) ? parsed.cases : [];
      const statusesPass = cases.length === 27 && cases.every((item) => String(item.status).toLowerCase() === 'pass' || String(item.status).toLowerCase() === 'passed');
      const nativeSummary = arm !== 'kessetsu' || (parsed.schema_version === 'kessetsu.experiment-results.v1'
        && parsed.summary?.total === 27 && parsed.summary?.passed === 27);
      study = { present: true, valid: statusesPass && nativeSummary, case_count: cases.length, error: null,
        schema_version: parsed.schema_version ?? null, sha256: digest(readFileSync(resultPath)) };
    } catch (error) { study = { present: true, valid: false, case_count: 0, error: error.message }; }
  }
  const artifacts = {
    'summary.csv': ['summary.csv'],
    'REPORT.md': ['REPORT.md'],
    'schematic.svg': ['schematic.svg', 'candidate.svg'],
    'schematic.png': ['schematic.png', 'candidate.png'],
    'candidate.kicad_sch': ['candidate.kicad_sch'],
    'candidate.asc': ['candidate.asc'],
    'bom.csv': ['bom.csv', 'candidate.bom.csv'],
    'handoff.json': ['handoff.json', 'candidate.handoff.json'],
  };
  const files = Object.fromEntries(Object.entries(artifacts)
    .map(([kind, names]) => [kind, hasNonEmptyFile(directory, names)]));
  const revisions = existsSync(join(directory, 'revisions'))
    ? readdirSync(join(directory, 'revisions'), { withFileTypes: true }).filter((entry) => entry.isFile()).length : 0;
  const report = files['REPORT.md'] ? readFileSync(join(directory, 'REPORT.md'), 'utf8') : '';
  return { study, files, revisions, direct_limit_truthful: arm !== 'direct' || /unavailable|not available|no automatic|does not provide/i.test(report) };
}

function main() {
  const [arm, path] = process.argv.slice(2);
  const record = { schema_version: 'kessetsu.agent-workflow-m1-evaluation.v1', evaluator_revision: 2, task: 'M1', arm };
  try {
    if (!['kessetsu', 'direct'].includes(arm) || !path) throw new Error('Expected <kessetsu|direct> <candidate-file>');
    record.spec_sha256 = digest(readFileSync(join(root, SPEC)));
    const candidatePath = resolve(path);
    const source = readFileSync(candidatePath, 'utf8');
    Object.assign(record, { candidate_source: source, candidate_sha256: digest(source) });
    let netlist = source;
    if (arm === 'kessetsu') {
      const binary = process.env.KESSETSU_BINARY ?? join(root, 'core/target/release', process.platform === 'win32' ? 'kess.exe' : 'kess');
      const run = spawnSync(binary, ['compile', '-', '--format', 'json', '--include', 'spice'], {
        input: source, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true,
      });
      Object.assign(record, { compiler_stdout: run.stdout, compiler_stderr: run.stderr });
      if (run.error || run.status !== 0) throw new Error(`Compilation failed: ${run.error?.message ?? run.status}`);
      netlist = JSON.parse(run.stdout).debug?.spice_netlist;
      if (typeof netlist !== 'string') throw new Error('Missing compiled netlist');
    }
    const circuit = inspectDivider(netlist);
    const result = evaluateCorners(circuit);
    Object.assign(record, result, { nominal_components: circuit, compiled_netlist: netlist,
      netlist_sha256: digest(netlist), workflow: inspectWorkflow(candidatePath, arm),
      verification_scope: {
        schema_version: 'kessetsu.evaluation-verification-scope.v1',
        specification_id: 'agent-workflow-v2/M1',
        conclusion: result.status === 'PASS' ? 'requirements_satisfied_within_scope' : 'requirements_not_satisfied_within_scope',
        hardware_validated: false,
        operating_conditions: ['Ideal divider; three supplies; nominal plus eight independent finite tolerance corners per supply.'],
        model_assumptions: ['Ideal linear source and resistors; exact stated nominal/tolerance values.'],
        untested_effects: ['ADC input behavior, protection, parasitics, temperature coefficients, arbitrary distributions, production yield and hardware measurements.'],
      } });
    process.exitCode = result.status === 'PASS' ? 0 : 1;
  } catch (error) {
    Object.assign(record, { status: 'ERROR', message: error.message });
    process.exitCode = 2;
  }
  console.log(JSON.stringify(record));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
