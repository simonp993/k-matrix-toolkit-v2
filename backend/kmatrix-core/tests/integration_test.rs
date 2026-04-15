use std::path::Path;

use kmatrix_core::model::{BusType, FileFormat, Platform};
use kmatrix_core::parser::xlsx::parser::parse_xlsx;
use kmatrix_core::parser::dbc::parser::parse_dbc;
use kmatrix_core::ParserRegistry;

/// Path to the K-Matrizen test fixtures directory.
const FIXTURES_DIR: &str =
    "/Users/SP8PTW8/Code_Development/K-matrix-search-tool/K-Matrizen";

fn fixture_exists(rel: &str) -> bool {
    Path::new(FIXTURES_DIR).join(rel).exists()
}

// ── XLSX tests ──────────────────────────────────────────────────────────

#[test]
fn xlsx_parse_lin_kmatrix() {
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/LIN/MLBevo_Konzern_MLBevo_SitzMem_LIN1_KMatrix_V8.26.00F.xlsx");
    if !path.exists() {
        eprintln!("SKIP: fixture not found at {}", path.display());
        return;
    }

    let matrices = parse_xlsx(&path).expect("should parse LIN XLSX");
    assert!(!matrices.is_empty(), "should produce at least one KMatrix");

    let m = &matrices[0];
    assert_eq!(m.format, FileFormat::XLSX);
    assert!(
        matches!(m.platform, Some(Platform::MLBevo2)),
        "platform should be MLBevo2, got {:?}",
        m.platform
    );
    assert_eq!(m.bus_type, BusType::LIN);
    assert!(!m.source_file.is_empty());
    assert!(!m.messages.is_empty(), "should have parsed messages");

    // Every message should have at least one signal
    for msg in &m.messages {
        assert!(!msg.name.is_empty(), "message name should not be empty");
        assert!(!msg.signals.is_empty(), "message '{}' should have signals", msg.name);
    }
}

#[test]
fn xlsx_messages_have_ecu_assignments() {
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/LIN/MLBevo_Konzern_MLBevo_SitzMem_LIN1_KMatrix_V8.26.00F.xlsx");
    if !path.exists() {
        eprintln!("SKIP: fixture not found");
        return;
    }

    let matrices = parse_xlsx(&path).unwrap();
    let m = &matrices[0];

    let with_ecu: usize = m
        .messages
        .iter()
        .filter(|msg| !msg.ecu_assignments.is_empty())
        .count();

    assert!(
        with_ecu > 0,
        "at least some messages should have ECU assignments"
    );
}

// ── DBC tests ───────────────────────────────────────────────────────────

#[test]
fn dbc_parse_can_kmatrix() {
    // Use smallest DBC file — can-dbc struggles with large production files
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/CAN/MLBevo_Gen2_MLBevo_EFP_SUBCAN02_KMatrix_V8.22.00F_20210601_SEn.dbc");
    if !path.exists() {
        eprintln!("SKIP: fixture not found at {}", path.display());
        return;
    }

    let matrices = parse_dbc(&path).expect("should parse CAN DBC");
    assert_eq!(matrices.len(), 1, "DBC should produce exactly one KMatrix");

    let m = &matrices[0];
    assert_eq!(m.format, FileFormat::DBC);
    assert!(
        matches!(m.platform, Some(Platform::MLBevo2)),
        "platform should be MLBevo2, got {:?}",
        m.platform
    );
    assert_eq!(m.bus_type, BusType::CAN);
    assert!(!m.messages.is_empty(), "should have parsed messages");

    // Spot check: every message should have a hex identifier
    for msg in &m.messages {
        assert!(!msg.name.is_empty());
        let id = msg.identifier.as_ref().expect("DBC messages should have identifier");
        assert!(id.starts_with("0x"), "identifier should be hex: {id}");
    }
}

#[test]
fn dbc_signals_have_bit_layout() {
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/CAN/MLBevo_Gen2_MLBevo_EFP_SUBCAN02_KMatrix_V8.22.00F_20210601_SEn.dbc");
    if !path.exists() {
        eprintln!("SKIP: fixture not found");
        return;
    }

    let matrices = parse_dbc(&path).unwrap();
    for msg in &matrices[0].messages {
        for sig in &msg.signals {
            assert!(sig.start_bit.is_some(), "signal '{}' missing start_bit", sig.name);
            assert!(sig.bit_length.is_some(), "signal '{}' missing bit_length", sig.name);
        }
    }
}

// ── ParserRegistry tests ────────────────────────────────────────────────

#[test]
fn registry_dispatches_xlsx() {
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/LIN/MLBevo_Konzern_MLBevo_SitzMem_LIN1_KMatrix_V8.26.00F.xlsx");
    if !path.exists() {
        eprintln!("SKIP: fixture not found");
        return;
    }

    let registry = ParserRegistry::new();
    let matrices = registry.parse(&path).expect("registry should parse XLSX");
    assert!(!matrices.is_empty());
}

#[test]
fn registry_dispatches_dbc() {
    let path = Path::new(FIXTURES_DIR)
        .join("MLBevo 2/K-Matrix/CAN/MLBevo_Gen2_MLBevo_EFP_SUBCAN02_KMatrix_V8.22.00F_20210601_SEn.dbc");
    if !path.exists() {
        eprintln!("SKIP: fixture not found");
        return;
    }

    let registry = ParserRegistry::new();
    let matrices = registry.parse(&path).expect("registry should parse DBC");
    assert!(!matrices.is_empty());
}

#[test]
fn registry_parse_directory() {
    let dir = Path::new(FIXTURES_DIR).join("MLBevo 2/K-Matrix/CAN");
    if !dir.exists() {
        eprintln!("SKIP: directory not found");
        return;
    }

    let registry = ParserRegistry::new();
    let matrices = registry
        .parse_directory(&dir)
        .expect("should parse CAN directory");

    assert!(
        matrices.len() >= 2,
        "CAN directory should have multiple KMatrix results, got {}",
        matrices.len()
    );
}
