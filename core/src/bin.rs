use netlang_core::ast::*;
use netlang_core::layout::*;
use std::collections::HashMap;

fn main() {
    let code = "
module Div
    in: ModulePort
    out: ModulePort
    R1: Resistor 10k
    R2: Resistor 10k
    connect in R1.p1
    connect R1.p2 out
    connect out R2.p1
    connect R2.p2 gnd


B1: Battery 9V
myDiv: Div
R3: Resistor 1k
Q1: NPN 2N2222

connect B1.plus myDiv.in
connect myDiv.out R3.p1
connect R3.p2 Q1.base
connect B1.plus Q1.collector
connect B1.minus Q1.emitter
connect B1.minus myDiv.gnd
";
    let program = netlang_core::parser::parse_program(code).unwrap();
    let layout = generate_layout(&program);
    for (name, pos) in &layout.components {
        println!("{}: x={}, y={}, rot={}", name, pos.x, pos.y, pos.rotation);
    }
}
