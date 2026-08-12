use crate::compiler::{COMPILE_SCHEMA_VERSION, CompileOptions, compile_source};
use wasm_bindgen::prelude::*;

/// Thin browser adapter over the canonical, side-effect-free compile pipeline.
#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> Result<JsValue, JsValue> {
    let report = compile_source(input, CompileOptions::all_outputs());
    serde_wasm_bindgen::to_value(&report)
        .map_err(|error| JsValue::from_str(&format!("Could not serialize compile report: {error}")))
}

/// Returns the compile-report contract implemented by this WASM build.
///
/// Browser clients must compare reports against this value instead of
/// duplicating the version string in TypeScript.
#[wasm_bindgen]
pub fn compile_schema_version() -> String {
    COMPILE_SCHEMA_VERSION.to_string()
}
