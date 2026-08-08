use wasm_bindgen::prelude::*;
use crate::parser::parse_program;
use crate::erc::check_rules;
use crate::graph::{NetlistGraph, generate_spice};
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct CompileResult {
    pub ast: Option<crate::ast::Program>,
    #[serde(rename = "erc_errors")]
    pub erc_errors: Vec<crate::erc::ErcDiagnostic>,
    pub layout: Option<crate::layout::LayoutResult>,
    pub kicad_sch: Option<String>,
    pub parse_error: Option<String>,
    pub spice_netlist: Option<String>,
}

#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> JsValue {
    let mut result = CompileResult {
        ast: None,
        erc_errors: Vec::new(),
        layout: None,
        kicad_sch: None,
        parse_error: None,
        spice_netlist: None,
    };

    match parse_program(input) {
        Ok(program) => {
            match program.flatten() {
                Ok(flat_program) => {
                    match crate::ir::ast_to_ir(&flat_program) {
                        Ok(circuit_ir) => {
                            let graph = NetlistGraph::build(&circuit_ir);
                            let errors = check_rules(&circuit_ir, &graph);
                            
                            if errors.is_empty() {
                                result.spice_netlist = Some(generate_spice(&circuit_ir, &graph));
                                result.layout = Some(crate::layout::generate_layout(&circuit_ir));
                                result.kicad_sch = Some(crate::kicad::generate_kicad_sch(result.layout.as_ref().unwrap()));
                            }
                            
                            result.ast = Some(flat_program.clone());
                            result.erc_errors = errors;
                        },
                        Err(e) => {
                            result.parse_error = Some(e);
                        }
                    }
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
