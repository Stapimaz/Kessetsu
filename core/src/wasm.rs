use wasm_bindgen::prelude::*;
use crate::parser::parse_program;
use crate::drc::check_rules;
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct CompileResult {
    pub ast: Option<crate::ast::Program>,
    pub drc_errors: Vec<crate::drc::DrcError>,
    pub parse_error: Option<String>,
}

#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> JsValue {
    let mut result = CompileResult {
        ast: None,
        drc_errors: Vec::new(),
        parse_error: None,
    };

    match parse_program(input) {
        Ok(program) => {
            let errors = check_rules(&program);
            result.ast = Some(program);
            result.drc_errors = errors;
        }
        Err(e) => {
            result.parse_error = Some(format!("{}", e));
        }
    }

    serde_wasm_bindgen::to_value(&result).unwrap()
}
