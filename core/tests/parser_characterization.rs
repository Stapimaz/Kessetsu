mod common;

use common::{fixture_path, read_fixture};
use netlang_core::ast::Statement;
use netlang_core::parse_program;
use std::fs;
use std::path::{Path, PathBuf};

fn nl_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not read '{}': {error}", directory.display()))
        .map(|entry| {
            entry
                .expect("fixture directory entry must be readable")
                .path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "nl"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_repository_example_parses_and_flattens() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples");
    let files = nl_files(&examples);

    assert_eq!(
        files.len(),
        5,
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
    let files = nl_files(&fixture_path("valid"));
    assert_eq!(files.len(), 3);

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
    let files = nl_files(&fixture_path("invalid/parser"));
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
    let source = read_fixture("valid/module.nl");
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
