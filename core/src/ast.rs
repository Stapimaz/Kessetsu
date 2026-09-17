use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    Resistor,
    Source,
    CurrentSource,
    Capacitor,
    Inductor,
    Diode,
    Transistor,
    Mosfet,
    OpAmp,
    ModulePort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDecl {
    pub comp_type: ComponentType,
    pub name: String,
    pub subtype: Option<String>,
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<crate::expression::Expression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interface_pins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinRef {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub pins: Vec<PinRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetDecl {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cmp {
    Lt,
    Gt,
    Eq,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertStmt {
    pub metric: String,
    pub signal: String,
    pub cmp: Cmp,
    pub threshold: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseStmt {
    pub module_name: String,
    pub inst_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<ParameterOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterOverride {
    pub name: String,
    pub expression: crate::expression::Expression,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulateStmt {
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDeclKind {
    Diode,
    BJT,
    MOSFET,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDecl {
    pub kind: ModelDeclKind,
    pub name: String,
    pub polarity: Option<String>,
    pub parameters: Vec<NamedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubcircuitDecl {
    pub kind: String,
    pub name: String,
    pub pins: Vec<String>,
    pub parameters: Vec<NamedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalSubcircuitDecl {
    pub kind: String,
    pub name: String,
    pub pins: Vec<String>,
    pub parameters: Vec<NamedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInclude {
    pub package: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Param(crate::expression::ParameterDecl),
    Decl(ComponentDecl),
    Connect(Connection),
    Net(NetDecl),
    Assert(AssertStmt),
    Use(UseStmt),
    Simulate(SimulateStmt),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub pins: Vec<String>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub modules: Vec<ModuleDef>,
    pub model_includes: Vec<ModelInclude>,
    pub models: Vec<ModelDecl>,
    pub subcircuits: Vec<SubcircuitDecl>,
    pub external_subcircuits: Vec<ExternalSubcircuitDecl>,
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn flatten(&self) -> Result<Program, String> {
        crate::elaboration::flatten(self)
    }
}
