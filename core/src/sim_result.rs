use crate::ast::Cmp;
use crate::ir::{Assertion, CircuitIR};
use crate::simulation::SimulationResult;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub pass: bool,
    pub assertion: Assertion,
    pub actual: f64,
}

const EPSILON: f64 = 1e-6;

fn evaluate_cmp(actual: f64, expected: f64, cmp: &Cmp) -> bool {
    match cmp {
        Cmp::Eq => (actual - expected).abs() < EPSILON,
        Cmp::Lt => actual < expected,
        Cmp::Gt => actual > expected,
        Cmp::Le => actual <= expected + EPSILON,
        Cmp::Ge => actual >= expected - EPSILON,
    }
}

pub fn evaluate_assertions(circuit: &CircuitIR, sim_result: &SimulationResult) -> Vec<TestResult> {
    let mut results = Vec::new();

    for assert in &circuit.assertions {
        let raw_name = format!("{}_{}", assert.metric, assert.signal);
        let safe_name = raw_name.replace("(", "_").replace(")", "").to_lowercase();

        if let Some(&actual) = sim_result.measurements.get(&safe_name) {
            let pass = evaluate_cmp(actual, assert.threshold.value, &assert.cmp);
            results.push(TestResult {
                pass,
                assertion: assert.clone(),
                actual,
            });
        } else {
            results.push(TestResult {
                pass: false,
                assertion: assert.clone(),
                actual: f64::NAN,
            });
        }
    }

    results
}
