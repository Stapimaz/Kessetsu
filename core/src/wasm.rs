use crate::compiler::{CompileOptions, compile_source};
use wasm_bindgen::prelude::*;

/// Thin browser adapter over the canonical, side-effect-free compile pipeline.
#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> Result<JsValue, JsValue> {
    let report = compile_source(input, CompileOptions::all_outputs());
    serde_wasm_bindgen::to_value(&report)
        .map_err(|error| JsValue::from_str(&format!("Could not serialize compile report: {error}")))
}
