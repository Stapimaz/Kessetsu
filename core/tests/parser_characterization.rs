mod common;

use common::{fixture_path, read_fixture};
use kessetsu_core::ast::Statement;
use kessetsu_core::parse_program;
use std::fs;
use std::path::{Path, PathBuf};

fn kess_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not read '{}': {error}", directory.display()))
        .map(|entry| {
            entry
                .expect("fixture directory entry must be readable")
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "kess")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn every_repository_example_parses_and_flattens() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples");
    let files = kess_files(&examples);

    assert_eq!(
        files.len(),
        8,
        "example matrix changed; review the new fixture"
    );
    for path in files {
        let source = fs::read_to_string(&path).expect("example source must be readable");
        let program = parse_program(&source)
            .unwrap_or_else(|error| panic!("'{}' did not parse: {error}", path.display()));
        program
            .flatten()
            .unwrap_or_else(|error| panic!("'{}' did not flatten: {error}", path.display()));
    }
}

#[test]
fn valid_fixture_corpus_parses_and_flattens() {
    let files = kess_files(&fixture_path("valid"));
    assert_eq!(
        files.len(),
        5,
        "valid fixture matrix changed; review the new fixture"
    );

    for path in files {
        let source = fs::read_to_string(&path).expect("valid fixture must be readable");
        let program = parse_program(&source)
            .unwrap_or_else(|error| panic!("valid fixture '{}' failed: {error}", path.display()));
        program
            .flatten()
            .unwrap_or_else(|error| panic!("valid fixture '{}' failed: {error}", path.display()));
    }
}

#[test]
fn parser_invalid_fixture_corpus_is_rejected() {
    let files = kess_files(&fixture_path("invalid/parser"));
    assert_eq!(files.len(), 4);

    for path in files {
        let source = fs::read_to_string(&path).expect("invalid fixture must be readable");
        assert!(
            parse_program(&source).is_err(),
            "invalid parser fixture unexpectedly succeeded: {}",
            path.display()
        );
    }
}

#[test]
fn empty_and_comment_only_sources_are_valid_empty_programs() {
    for source in ["", "\n\r\n", "// yalnızca UTF-8 yorum: ölçüm Ω\n"] {
        let program = parse_program(source).expect("empty source form should parse");
        assert!(program.modules.is_empty());
        assert!(program.statements.is_empty());
    }
}

#[test]
fn utf8_bom_is_explicitly_rejected_until_normalized_by_the_frontend() {
    let source = "\u{feff}resistor R1 1k\n";
    assert!(parse_program(source).is_err());
}

#[test]
fn identifiers_are_ascii_while_unicode_comments_are_supported() {
    assert!(parse_program("// ölçüm Ω\nresistor R1 1k\n").is_ok());
    assert!(parse_program("resistor ölçüm 1k\n").is_err());
}

#[test]
fn module_use_flattening_prefixes_components_and_preserves_port_connections() {
    let source = read_fixture("valid/module.kess");
    let flat = parse_program(&source)
        .expect("module fixture should parse")
        .flatten()
        .expect("module fixture should flatten");

    let declarations: Vec<_> = flat
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Decl(declaration) => Some(declaration.name.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(declarations, ["V1", "divider", "divider_R1", "divider_R2"]);
    assert!(flat.statements.iter().any(|statement| {
        match statement {
            Statement::Connect(connection) => connection
                .pins
                .iter()
                .any(|pin| pin.component == "divider" && pin.pin == "in"),
            _ => false,
        }
    }));
}

#[test]
fn crlf_and_tabs_follow_the_same_grammar_as_lf_and_spaces() {
    let source = "source\tV1\t5V\r\nresistor\tR1\t1k\r\nconnect\tV1.plus\tto\tR1.p1\r\nconnect V1.minus to R1.p2\r\n";
    let program = parse_program(source).expect("CRLF/tab source should parse");
    assert_eq!(program.statements.len(), 4);
}

#[test]
fn multi_pin_connections_preserve_source_order_in_the_ast() {
    let source = "connect A.p1, B.p2, named_net to C.p3\n";
    let program = parse_program(source).expect("multi-pin connection should parse");
    let Statement::Connect(connection) = &program.statements[0] else {
        panic!("expected a connection statement");
    };
    let pins: Vec<_> = connection
        .pins
        .iter()
        .map(|pin| (pin.component.as_str(), pin.pin.as_str()))
        .collect();
    assert_eq!(
        pins,
        [("A", "p1"), ("B", "p2"), ("", "named_net"), ("C", "p3")]
    );
}

#[test]
fn flattening_an_unknown_module_fails_explicitly() {
    let program = parse_program("use Missing instance\n").expect("use statement should parse");
    let error = program
        .flatten()
        .expect_err("unknown module must not flatten");
    assert_eq!(error, "Module not found: Missing");
}

#[test]
fn unsupported_legacy_battery_and_connect_forms_are_rejected() {
    assert!(parse_program("battery B1 9V\n").is_err());
    assert!(parse_program("connect B1.plus R1.p1\n").is_err());
}

#[test]
fn external_subcircuit_declaration_preserves_typed_fields() {
    let source = "external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file=\"models/OPAx197.LIB\" entry=OPAx197 sha256=fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5 version=\"Final 1.3\" license=\"TI terms\" source=\"https://www.ti.com/lit/zip/SBOMA34\" simulator=ngspice_ps redistribution=prohibited\n";
    let program = parse_program(source).expect("typed external declaration should parse");
    assert_eq!(program.external_subcircuits.len(), 1);
    let declaration = &program.external_subcircuits[0];
    assert_eq!(declaration.kind, "opamp");
    assert_eq!(declaration.name, "OPA197");
    assert_eq!(declaration.pins, ["in_p", "in_n", "vcc", "vee", "out"]);
    assert!(
        declaration
            .parameters
            .iter()
            .any(|field| { field.name == "file" && field.value == "models/OPAx197.LIB" })
    );
}
