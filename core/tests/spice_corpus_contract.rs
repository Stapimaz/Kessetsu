//! Clean-Room First-Party SPICE Compatibility Corpus & Empirical Gap Contract
//!
//! # Provenance & Clean-Room Statement
//! All 30 SPICE fixtures in this file were authored exclusively from first-party
//! synthetic specifications for Kessetsu. No third-party copyrighted netlists,
//! LTspice example files, Ngspice distribution decks, textbook listings, or
//! proprietary vendor application note decks were copied verbatim.
//!
//! Each fixture represents a standard, minimal, self-contained SPICE dialect idiom
//! produced by EDA tools (KiCad SPICE export, LTspice, Ngspice, PSpice, or manufacturer
//! SPICE models) to evaluate Kessetsu's SPICE importer (`import_spice()`),
//! AST-to-IR lowering, Electrical Rules Check (ERC), and SPICE re-generation.

mod common;

use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use kessetsu_core::spice_import::{ImportSeverity, import_spice};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyTier {
    High,
    Medium,
    Low,
}

impl FrequencyTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactTier {
    High,
    Medium,
    Low,
}

impl ImpactTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedOutcome {
    /// Importer succeeds with 0 errors, generated .kess compiles through IR & ERC with 0 errors,
    /// and generates a canonical SPICE netlist.
    SupportedRoundTrip,
    /// Importer succeeds with 0 errors but emits an expected warning diagnostic code, and
    /// generated .kess compiles cleanly through IR & ERC.
    SupportedWithWarning {
        expected_warning_code: &'static str,
    },
    /// Importer fails closed with an expected diagnostic error code (e.g. KES-N002, KES-N003).
    ExpectedUnsupportedImport {
        expected_error_code: &'static str,
        construct_gap: &'static str,
    },
    /// Importer produces .kess that fails downstream parse, IR compilation, or ERC.
    ExpectedUnsupportedCompile {
        construct_gap: &'static str,
    },
}

#[derive(Debug, Clone)]
pub struct SpiceCorpusFixture {
    pub id: &'static str,
    pub category: &'static str,
    pub construct: &'static str,
    pub real_world_frequency: FrequencyTier,
    pub engineering_impact: ImpactTier,
    pub spice_netlist: &'static str,
    pub expected: ExpectedOutcome,
}

pub const CORPUS_FIXTURES: &[SpiceCorpusFixture] = &[
    // =========================================================================
    // Category 1 — Passive & Value Notations (3 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C01_PASSIVE_STANDARD_SUFFIXES",
        category: "1. Passive & Value Notations",
        construct: "Standard R/L/C suffixes (meg, k, m, u, n, p, f) and trailing unit letters",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Standard R/L/C suffixes and trailing units
V1 in 0 5
R1 in n1 10kOhm
R2 n1 n2 1MEG
L1 n2 n3 1mH
C1 n3 0 100nF
C2 n3 0 10pF
.op
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C02_PASSIVE_RKM_NOTATION",
        category: "1. Passive & Value Notations",
        construct: "RKM European value notation (4k7, 1u5, 2m2)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: RKM European notation
V1 in 0 5
R1 in out 4k7
R2 out 0 10k
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "RKM European notation (e.g. 4k7) rejected as non-literal numeric",
        },
    },
    SpiceCorpusFixture {
        id: "C03_PASSIVE_EXTRA_INSTANCE_TOKENS",
        category: "1. Passive & Value Notations",
        construct: "Resistor/capacitor extra instance parameters (tc1=..., IC=...)",
        real_world_frequency: FrequencyTier::Medium,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: Extra passive instance parameters
V1 in 0 5
R1 in out 10k tc1=0.001
R2 out 0 10k
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Passive element instance parameters (TC, IC) exceed 4-field limit",
        },
    },

    // =========================================================================
    // Category 2 — Independent Source Idioms (3 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C04_SOURCE_COMBINED_DC_AC",
        category: "2. Independent Source Idioms",
        construct: "Combined non-zero DC bias and AC stimulus (V1 in 0 DC 5 AC 1 0)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Combined DC and AC independent source
V1 in 0 DC 5 AC 1 0
R1 in out 1k
R2 out 0 1k
.ac dec 10 10 100k
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Combined non-zero DC bias and AC magnitude is not representable in IR",
        },
    },
    SpiceCorpusFixture {
        id: "C05_SOURCE_SPACE_SEPARATED_WAVEFORM",
        category: "2. Independent Source Idioms",
        construct: "Space-separated waveform arguments without parentheses (PULSE 0 5 ...)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: Space-separated PULSE arguments without parentheses
V1 in 0 PULSE 0 5 1u 10n 10n 5u 10u
R1 in 0 1k
.tran 10n 20u
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C06_SOURCE_EXP_WAVEFORM",
        category: "2. Independent Source Idioms",
        construct: "Exponential transient source waveform EXP(v1 v2 td1 tau1 td2 tau2)",
        real_world_frequency: FrequencyTier::Medium,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: Exponential transient source EXP
V1 in 0 EXP(0 5 1u 5u 10u 20u)
R1 in 0 1k
.tran 100n 50u
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Exponential transient waveform (EXP) is unsupported",
        },
    },

    // =========================================================================
    // Category 3 — Semiconductor & .model Dialect Idioms (6 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C07_MODEL_DIODE_ZENER_BREAKDOWN",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "Zener diode model with breakdown parameters (BV, IBV) and continuation (+)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Zener diode model with breakdown and continuation
V1 in 0 10
R1 in out 1k
D1 out 0 BZX84C5V1
.model BZX84C5V1 D(IS=1e-14 RS=0.5
+ BV=5.1 IBV=1m CJO=50p)
.op
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C08_BJT_FOUR_TERMINAL_SUBSTRATE",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "Four-terminal BJT with explicit substrate node (Q1 c b e 0 2N2222)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: BJT with explicit 4th substrate node
Vcc vcc 0 12
Vin vin 0 1
Rb vin b 10k
Rc vcc c 1k
Q1 c b 0 0 2N2222
.model 2N2222 NPN(IS=1e-14 BF=200)
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "4-terminal BJT with substrate node is unsupported",
        },
    },
    SpiceCorpusFixture {
        id: "C09_MOSFET_BULK_TIED_SOURCE",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "Four-terminal MOSFET with bulk tied to source (M1 d g s s NMOS_MOD)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Four-terminal MOSFET with bulk tied to source
Vdd vdd 0 5
Vg vg 0 3
Rload vdd d 1k
M1 d vg 0 0 NMOS_MOD
.model NMOS_MOD NMOS(VTO=1.0 KP=1e-4)
.op
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C10_MOSFET_BULK_TIED_RAIL",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "Four-terminal MOSFET with bulk tied to separate rail/substrate (s != b)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Four-terminal MOSFET with bulk tied to rail (bulk != source)
Vdd vdd 0 5
Vg vg 0 3
M1 d vg s 0 NMOS_MOD
Rs s 0 100
Rload vdd d 1k
.model NMOS_MOD NMOS(VTO=1.0 KP=1e-4)
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "4-terminal MOSFET with bulk tied to separate rail/substrate is unsupported",
        },
    },
    SpiceCorpusFixture {
        id: "C11_MOSFET_INLINE_GEOMETRY_PARAMS",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "MOSFET instance with inline geometry parameters (W=10u L=1u)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: MOSFET with inline geometry parameters W and L
Vdd vdd 0 5
Vg vg 0 3
Rload vdd d 1k
M1 d vg 0 0 NMOS_MOD W=10u L=1u
.model NMOS_MOD NMOS(VTO=1.0 KP=1e-4)
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "MOSFET instance geometry parameters (W, L, AD) exceed expected token count",
        },
    },
    SpiceCorpusFixture {
        id: "C12_JFET_DEVICE_AND_MODEL",
        category: "3. Semiconductor & .model Dialect Idioms",
        construct: "JFET element and .model NJF (J1 d g s J2N5457)",
        real_world_frequency: FrequencyTier::Medium,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: JFET device and NJF model card
Vdd vdd 0 15
Vg vg 0 -1
Rload vdd d 2k
J1 d vg 0 J2N5457
.model J2N5457 NJF(VTO=-2.5 BETA=1.5m)
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N012",
            construct_gap: "JFET component family 'J' and .model type 'NJF' outside supported subset",
        },
    },

    // =========================================================================
    // Category 4 — Controlled Sources & Switches (5 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C13_CONTROLLED_CLASSIC_LINEAR_ALL",
        category: "4. Controlled Sources & Switches",
        construct: "Classic linear controlled sources E (VCVS), G (VCCS), F (CCCS), H (CCVS)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Classic linear controlled sources E, G, F, H
V1 in 0 2
Vsense1 in n1 0
R1 n1 0 1k
E1 out1 0 in 0 2.0
Rload1 out1 0 1k
G1 out2 0 in 0 1m
Rload2 out2 0 1k
F1 out3 0 Vsense1 3.0
Rload3 out3 0 1k
Vsense2 in n2 0
R2 n2 0 1k
H1 out4 0 Vsense2 200
Rload4 out4 0 1k
.op
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C14_CONTROLLED_BEHAVIORAL_EXPRESSION_E",
        category: "4. Controlled Sources & Switches",
        construct: "Behavioral expression VCVS (E1 out 0 VALUE={V(in)*2})",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Behavioral expression VCVS E VALUE={...}
V1 in 0 2.5
R1 in 0 1k
E1 out 0 VALUE={V(in)*2}
Rload out 0 1k
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Behavioral expression source (E/G VALUE={...}) is unsupported",
        },
    },
    SpiceCorpusFixture {
        id: "C15_CONTROLLED_ARBITRARY_B_SOURCE",
        category: "4. Controlled Sources & Switches",
        construct: "Arbitrary behavioral source B (B1 out 0 V=V(in)*2)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Arbitrary behavioral source B
V1 in 0 2.5
R1 in 0 1k
B1 out 0 V=V(in)*2
Rload out 0 1k
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Arbitrary behavioral source B (B1 out 0 V=...) is unsupported",
        },
    },
    SpiceCorpusFixture {
        id: "C16_CONTROLLED_SWITCH_S_VOLTAGE",
        category: "4. Controlled Sources & Switches",
        construct: "Voltage-controlled switch S with .model SW(VT=... VH=... RON=... ROFF=...)",
        real_world_frequency: FrequencyTier::Medium,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Voltage-controlled switch S with SW model
Vctrl ctrl 0 5
Vsrc in 0 10
S1 out in ctrl 0 SW1
Rload out 0 1k
.model SW1 SW(VT=2.5 VH=0.1 RON=1 ROFF=1MEG)
.op
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C17_CONTROLLED_SWITCH_W_CURRENT",
        category: "4. Controlled Sources & Switches",
        construct: "Current-controlled switch W with .model CSW(...)",
        real_world_frequency: FrequencyTier::Low,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: Current-controlled switch W with CSW model
Vctrl ctrl 0 5
Vsense ctrl 0 0
Vsrc in 0 10
W1 out in Vsense CSW1
Rload out 0 1k
.model CSW1 CSW(IT=1m IH=0.1m RON=1 ROFF=1MEG)
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N012",
            construct_gap: "Current-controlled switch 'W' and 'CSW' model are unsupported",
        },
    },

    // =========================================================================
    // Category 5 — Magnetic & Coupled Elements (2 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C18_MAGNETIC_COUPLED_INDUCTORS_TWO_WINDING",
        category: "5. Magnetic & Coupled Elements",
        construct: "Coupled inductors two-winding transformer (K1 L1 L2 0.99)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Coupled inductors two-winding transformer
V1 in 0 AC 1
L1 in 0 1m
L2 out 0 4m
Rload out 0 1k
K1 L1 L2 0.99
.ac dec 10 100 100k
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Coupled inductors mutual coupling directive 'K' is unsupported",
        },
    },
    SpiceCorpusFixture {
        id: "C19_MAGNETIC_MULTI_WINDING_COUPLING",
        category: "5. Magnetic & Coupled Elements",
        construct: "Multi-winding coupled inductor statement (K1 L1 L2 L3 0.98)",
        real_world_frequency: FrequencyTier::Medium,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Multi-winding coupled inductors
V1 in 0 AC 1
L1 in 0 1m
L2 out1 0 2m
L3 out2 0 4m
Rload1 out1 0 1k
Rload2 out2 0 1k
K1 L1 L2 L3 0.98
.ac dec 10 100 100k
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N002",
            construct_gap: "Multi-winding mutual coupling directive 'K' is unsupported",
        },
    },

    // =========================================================================
    // Category 6 — Parameterization & Expressions (3 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C20_PARAM_LITERAL_SUBSTITUTION",
        category: "6. Parameterization & Expressions",
        construct: "Simple .param literal substitution in resistor and capacitor ({RVAL})",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Simple parameter literal substitution
.param RVAL=10k CVAL=100n
V1 in 0 AC 1
R1 in out {RVAL}
C1 out 0 {CVAL}
.ac dec 10 10 100k
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C21_PARAM_ARITHMETIC_EXPRESSION",
        category: "6. Parameterization & Expressions",
        construct: "Arithmetic expression parameter (.param R2_VAL={R1_VAL * 2})",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Parameter arithmetic expression
.param R1_VAL=10k
.param R2_VAL={R1_VAL * 2}
V1 in 0 5
R1 in out {R1_VAL}
R2 out 0 {R2_VAL}
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N006",
            construct_gap: "Evaluated parameter arithmetic expression is outside literal parameter subset",
        },
    },
    SpiceCorpusFixture {
        id: "C22_PARAM_BUILTIN_FUNCTIONS",
        category: "6. Parameterization & Expressions",
        construct: "Mathematical expression in .param ({1/(2*pi*1k*100n)})",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Parameter with math formula
.param FC={1/(2*3.14159265*1k*100n)}
V1 in 0 AC 1
R1 in out 1k
C1 out 0 100n
.ac dec 10 10 100k
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N006",
            construct_gap: "Formula/function evaluation in .param is outside literal parameter subset",
        },
    },

    // =========================================================================
    // Category 7 — Subcircuits, Hierarchy & Includes (3 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C23_SUBCKT_BUILTIN_OPAMP",
        category: "7. Subcircuits, Hierarchy & Includes",
        construct: "Built-in 5-pin op-amp subcircuit definition and X instance",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Built-in 5-pin op-amp subcircuit
.subckt KESSETSU_OPAMP_V1 in_p in_n vcc vee out
EGAIN n_int 0 in_p in_n 200000
RPOLE n_int out 1
CPOLE out 0 0.031830988618
RLOAD out 0 1e9
.ends KESSETSU_OPAMP_V1
Vcc vcc 0 15
Vee vee 0 -15
Vin in 0 AC 1
X1 0 in vcc vee out KESSETSU_OPAMP_V1
Rf out in 10k
Rin in 0 1k
.ac dec 10 1 100k
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C24_SUBCKT_GENERAL_USER_MODULE",
        category: "7. Subcircuits, Hierarchy & Includes",
        construct: "General multi-pin .subckt definition and X instance with parameters",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: General multi-pin subcircuit with parameters
.subckt RC_FILTER IN OUT REF PARAMS: RVAL=1k CVAL=100n
R1 IN OUT {RVAL}
C1 OUT REF {CVAL}
.ends RC_FILTER
V1 in 0 AC 1
X1 in out 0 RC_FILTER RVAL=2k CVAL=50n
.ac dec 10 10 100k
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C25_DIRECTIVE_INCLUDE_FILE",
        category: "7. Subcircuits, Hierarchy & Includes",
        construct: ".include or .lib external file directive",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: External file include directive
.include ./models/diode.lib
V1 in 0 5
R1 in out 1k
D1 out 0 1N4148
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N003",
            construct_gap: "Model libraries and file inclusions (.include, .lib) require explicit resource binding",
        },
    },

    // =========================================================================
    // Category 8 — Analysis, Initial Conditions & Measurement Directives (5 fixtures)
    // =========================================================================
    SpiceCorpusFixture {
        id: "C26_ANALYSIS_TRAN_FOUR_ARG_LTSPICE",
        category: "8. Analysis, Initial Conditions & Measurement Directives",
        construct: "Four-argument LTspice style .tran (tstep=0, tstop, tstart=0, tmax)",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: LTspice 4-argument .tran statement
V1 in 0 PULSE(0 5 0 1u 1u 5m 10m)
R1 in out 1k
C1 out 0 1u
.tran 0 20m 0 10u
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C27_ANALYSIS_DC_SWEEP",
        category: "8. Analysis, Initial Conditions & Measurement Directives",
        construct: "Standard .dc sweep of independent voltage source",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: DC sweep of independent source
V1 in 0 0
R1 in out 2k
R2 out 0 3k
.dc V1 0 10 0.5
.end
",
        expected: ExpectedOutcome::SupportedRoundTrip,
    },
    SpiceCorpusFixture {
        id: "C28_DIRECTIVE_INITIAL_CONDITIONS_IC",
        category: "8. Analysis, Initial Conditions & Measurement Directives",
        construct: "Initial condition directive (.ic V(out)=2.5 or .nodeset) skipped with warning",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::Medium,
        spice_netlist: "\
* Clean-room synthetic fixture: Initial condition directive skipped with warning
V1 in 0 5
R1 in out 1k
C1 out 0 100n
.ic V(out)=2.5
.tran 10u 2m
.end
",
        expected: ExpectedOutcome::SupportedWithWarning {
            expected_warning_code: "KES-N011",
        },
    },
    SpiceCorpusFixture {
        id: "C29_DIRECTIVE_MEASURE_STATEMENT",
        category: "8. Analysis, Initial Conditions & Measurement Directives",
        construct: ".meas or .measure directive skipped with warning",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Measure directive skipped with warning
V1 in 0 AC 1
R1 in out 1k
C1 out 0 100n
.ac dec 10 10 100k
.meas AC bw_hz TRIG V(out) VAL=0.707 FALL=1
.end
",
        expected: ExpectedOutcome::SupportedWithWarning {
            expected_warning_code: "KES-N011",
        },
    },
    SpiceCorpusFixture {
        id: "C30_DIRECTIVE_STEP_PARAM",
        category: "8. Analysis, Initial Conditions & Measurement Directives",
        construct: ".step param sweep directive",
        real_world_frequency: FrequencyTier::High,
        engineering_impact: ImpactTier::High,
        spice_netlist: "\
* Clean-room synthetic fixture: Parameter sweep directive .step
.param RVAL=1k
V1 in 0 5
R1 in out {RVAL}
R2 out 0 1k
.step param RVAL 1k 10k 1k
.op
.end
",
        expected: ExpectedOutcome::ExpectedUnsupportedImport {
            expected_error_code: "KES-N003",
            construct_gap: "Multi-run parameter stepping (.step) is unsupported",
        },
    },
];

#[test]
fn spice_corpus_all_fixtures_match_declared_disposition() {
    assert_eq!(
        CORPUS_FIXTURES.len(),
        30,
        "Corpus must contain exactly 30 fixtures"
    );

    let mut supported_roundtrip_count = 0;
    let mut supported_warning_count = 0;
    let mut unsupported_import_count = 0;
    let mut unsupported_compile_count = 0;

    let runner = NgspiceRunner::discover();
    let ngspice_available = runner.executable().exists();

    println!("\n| ID | Category | Construct | Disposition | Diagnostic / Gap | Freq | Impact |");
    println!("|---|---|---|---|---|---|---|");

    for fixture in CORPUS_FIXTURES {
        let report = import_spice(fixture.spice_netlist);

        match &fixture.expected {
            ExpectedOutcome::SupportedRoundTrip => {
                assert!(
                    !report.has_errors(),
                    "[{}] Expected successful import with 0 errors, got: {:?}",
                    fixture.id,
                    report.diagnostics
                );
                let source = report
                    .kess_source
                    .as_deref()
                    .expect("source emitted on successful import");
                let compiled = compile_source(source, CompileOptions::default());
                assert!(
                    !compiled.has_errors(),
                    "[{}] Downstream compile failed on generated source:\n---\n{}\n---\nDiagnostics: {:?}",
                    fixture.id,
                    source,
                    compiled.diagnostics
                );
                assert!(
                    compiled.ir.is_some(),
                    "[{}] CircuitIR was not produced",
                    fixture.id
                );
                assert!(
                    compiled.spice_netlist.is_some(),
                    "[{}] Regenerated SPICE netlist was not produced",
                    fixture.id
                );

                if ngspice_available {
                    let netlist = compiled.spice_netlist.as_ref().unwrap();
                    let analyses = compiled.ir.as_ref().unwrap().analyses.clone();
                    let sim_res = runner
                        .run(
                            &SimulationRequest::new(netlist, analyses),
                            &CancellationToken::new(),
                        )
                        .expect("Ngspice run should launch");
                    assert!(
                        sim_res.succeeded(),
                        "[{}] Ngspice simulation failed on regenerated netlist: {:?}",
                        fixture.id,
                        sim_res.errors
                    );
                }

                supported_roundtrip_count += 1;
                println!(
                    "| `{}` | {} | {} | SupportedRoundTrip | Clean | {} | {} |",
                    fixture.id,
                    fixture.category,
                    fixture.construct,
                    fixture.real_world_frequency.as_str(),
                    fixture.engineering_impact.as_str()
                );
            }
            ExpectedOutcome::SupportedWithWarning {
                expected_warning_code,
            } => {
                assert!(
                    !report.has_errors(),
                    "[{}] Expected import without errors, got: {:?}",
                    fixture.id,
                    report.diagnostics
                );
                assert!(
                    report.diagnostics.iter().any(|d| {
                        d.severity == ImportSeverity::Warning && d.code == *expected_warning_code
                    }),
                    "[{}] Expected warning code '{}', diagnostics were: {:?}",
                    fixture.id,
                    expected_warning_code,
                    report.diagnostics
                );
                let source = report
                    .kess_source
                    .as_deref()
                    .expect("source emitted on warning import");
                let compiled = compile_source(source, CompileOptions::default());
                assert!(
                    !compiled.has_errors(),
                    "[{}] Downstream compile failed for warning fixture: {:?}",
                    fixture.id,
                    compiled.diagnostics
                );
                assert!(compiled.ir.is_some(), "[{}] CircuitIR was not produced", fixture.id);

                supported_warning_count += 1;
                println!(
                    "| `{}` | {} | {} | SupportedWithWarning | `{}` | {} | {} |",
                    fixture.id,
                    fixture.category,
                    fixture.construct,
                    expected_warning_code,
                    fixture.real_world_frequency.as_str(),
                    fixture.engineering_impact.as_str()
                );
            }
            ExpectedOutcome::ExpectedUnsupportedImport {
                expected_error_code,
                construct_gap,
            } => {
                assert!(
                    report.has_errors(),
                    "[{}] Expected import to fail closed with errors, but it succeeded: {:?}",
                    fixture.id,
                    report.diagnostics
                );
                assert!(
                    report.kess_source.is_none(),
                    "[{}] Failed import must not produce kess source",
                    fixture.id
                );
                assert!(
                    report.diagnostics.iter().any(|d| {
                        d.severity == ImportSeverity::Error && d.code == *expected_error_code
                    }),
                    "[{}] Expected error diagnostic code '{}', but got: {:?}",
                    fixture.id,
                    expected_error_code,
                    report.diagnostics
                );

                unsupported_import_count += 1;
                println!(
                    "| `{}` | {} | {} | UnsupportedImport | `{}`: {} | {} | {} |",
                    fixture.id,
                    fixture.category,
                    fixture.construct,
                    expected_error_code,
                    construct_gap,
                    fixture.real_world_frequency.as_str(),
                    fixture.engineering_impact.as_str()
                );
            }
            ExpectedOutcome::ExpectedUnsupportedCompile { construct_gap } => {
                let source = report
                    .kess_source
                    .as_deref()
                    .expect("kess source must be present to test downstream compile gap");
                let compiled = compile_source(source, CompileOptions::default());
                assert!(
                    compiled.has_errors(),
                    "[{}] Expected downstream compile to fail, but it succeeded",
                    fixture.id
                );

                unsupported_compile_count += 1;
                println!(
                    "| `{}` | {} | {} | UnsupportedCompile | {} | {} | {} |",
                    fixture.id,
                    fixture.category,
                    fixture.construct,
                    construct_gap,
                    fixture.real_world_frequency.as_str(),
                    fixture.engineering_impact.as_str()
                );
            }
        }
    }

    println!("\nCorpus Summary: 30 fixtures (RoundTrip: {}, Warning: {}, UnsupportedImport: {}, UnsupportedCompile: {})",
        supported_roundtrip_count, supported_warning_count, unsupported_import_count, unsupported_compile_count);
}

#[test]
fn spice_corpus_metadata_integrity_and_clean_room_coverage() {
    assert_eq!(CORPUS_FIXTURES.len(), 30);

    let mut ids = std::collections::BTreeSet::new();
    let mut categories = std::collections::BTreeSet::new();

    for fixture in CORPUS_FIXTURES {
        assert!(
            ids.insert(fixture.id),
            "Duplicate fixture ID discovered: {}",
            fixture.id
        );
        assert!(!fixture.category.is_empty(), "Empty category on {}", fixture.id);
        assert!(!fixture.construct.is_empty(), "Empty construct on {}", fixture.id);
        assert!(
            fixture.spice_netlist.contains("* Clean-room synthetic fixture:"),
            "Fixture {} must include clean-room provenance header comment",
            fixture.id
        );
        assert!(
            fixture.spice_netlist.contains(".end"),
            "Fixture {} must end with .end",
            fixture.id
        );
        categories.insert(fixture.category);
    }

    assert_eq!(
        categories.len(),
        8,
        "All 8 required real-world SPICE categories must be present: {:?}",
        categories
    );
}

