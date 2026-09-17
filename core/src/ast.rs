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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform_expression: Option<WaveformCall>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interface_pins: Vec<String>,
    /// Structured identity of a virtual module interface; not an electrical ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_path: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_expression: Option<crate::expression::Expression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numeric_expressions: Vec<IndexedExpression>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numeric_expressions: Vec<IndexedExpression>,
}

impl SimulateStmt {
    pub(crate) fn numeric_expression(
        &self,
        index: usize,
    ) -> Option<&crate::expression::Expression> {
        self.numeric_expressions
            .iter()
            .find(|arg| arg.index == index)
            .map(|arg| &arg.expression)
    }
}

/// A sequence is JSON-compatible in native and WASM serializers alike;
/// integer-keyed maps are not supported by the browser's object serializer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedExpression {
    pub index: usize,
    pub expression: crate::expression::Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveformCall {
    pub name: String,
    pub args: Vec<NumericArgument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericArgument {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<crate::expression::Expression>,
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

impl Statement {
    pub(crate) fn expression_nodes(&self) -> usize {
        match self {
            Self::Param(parameter) => parameter.expression.node_count(),
            Self::Decl(decl) => {
                decl.value_expression.as_ref().map_or(0, |e| e.node_count())
                    + decl.waveform_expression.as_ref().map_or(0, |call| {
                        call.args
                            .iter()
                            .filter_map(|arg| arg.expression.as_ref())
                            .map(|e| e.node_count())
                            .sum()
                    })
            }
            Self::Use(instance) => instance
                .overrides
                .iter()
                .map(|arg| arg.expression.node_count())
                .sum(),
            Self::Simulate(analysis) => analysis
                .numeric_expressions
                .iter()
                .map(|arg| arg.expression.node_count())
                .sum(),
            Self::Assert(assertion) => {
                assertion
                    .threshold_expression
                    .as_ref()
                    .map_or(0, |e| e.node_count())
                    + assertion
                        .numeric_expressions
                        .iter()
                        .map(|arg| arg.expression.node_count())
                        .sum::<usize>()
            }
            _ => 0,
        }
    }
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
            .map(|result| result.program)
            .map_err(|error| error.message)
    }
}
