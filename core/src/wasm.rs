use wasm_bindgen::prelude::*;
use crate::parser::parse_program;
use crate::drc::check_rules;
use crate::graph::{NetlistGraph, generate_spice};
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct CompileResult {
    pub ast: Option<crate::ast::Program>,
    pub drc_errors: Vec<crate::drc::DrcError>,
    pub layout: Option<crate::layout::LayoutResult>,
    pub parse_error: Option<String>,
    pub spice_netlist: Option<String>,
}

#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> JsValue {
    let mut result = CompileResult {
        ast: None,
        drc_errors: Vec::new(),
        layout: None,
        parse_error: None,
        spice_netlist: None,
    };

    match parse_program(input) {
        Ok(program) => {
            match program.flatten() {
                Ok(flat_program) => {
                    let graph = NetlistGraph::build(&flat_program);
                    let errors = check_rules(&flat_program, &graph);
                    
                    if errors.is_empty() {
                        result.spice_netlist = Some(generate_spice(&flat_program, &graph));
                    }
                    
                    result.ast = Some(flat_program.clone());
                    result.drc_errors = errors;
                    result.layout = Some(crate::layout::generate_layout(&flat_program));
                }
                Err(err_msg) => {
                    result.parse_error = Some(err_msg);
                }
            }
        }
        Err(e) => {
            result.parse_error = Some(format!("{}", e));
        }
    }

    serde_wasm_bindgen::to_value(&result).unwrap()
}
