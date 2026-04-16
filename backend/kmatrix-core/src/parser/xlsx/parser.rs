use std::path::Path;

use anyhow::{Context, Result};
use calamine::{open_workbook, Data, Reader, Xlsx};
use chrono::Utc;
use uuid::Uuid;

use crate::model::{
    BusType, EcuAssignment, EcuRole, FileFormat, KMatrix, Message, Signal, extract_metadata,
};

use super::column_map::{self, ColumnMap, MappedField};
use super::sheet_detect;

/// Parse an Excel K-Matrix file into one or more KMatrix structs.
/// One sheet = one KMatrix (FlexRay files may have Channel A + Channel B).
pub fn parse_xlsx(path: &Path) -> Result<Vec<KMatrix>> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).with_context(|| format!("Failed to open {}", path.display()))?;

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let data_sheets = sheet_detect::detect_data_sheets(&sheet_names);

    if data_sheets.is_empty() {
        anyhow::bail!(
            "No K-Matrix data sheets found in {}. Sheets: {:?}",
            path.display(),
            sheet_names
        );
    }

    let (platform, bus_type, bus_name) = extract_metadata(path);
    let source_file = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut matrices = Vec::new();

    for sheet_name in &data_sheets {
        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet '{sheet_name}'"))?;

        let rows: Vec<Vec<Option<String>>> = range
            .rows()
            .map(|row| {
                row.iter()
                    .map(|cell| cell_to_string(cell))
                    .collect()
            })
            .collect();

        if rows.len() < 5 {
            tracing::warn!(
                "Sheet '{}' in {} has fewer than 5 rows, skipping",
                sheet_name,
                path.display()
            );
            continue;
        }

        // Build column map from header rows (first 4 rows)
        let header_rows = &rows[..4.min(rows.len())];
        let col_map = column_map::build_column_map(header_rows);

        // Parse data rows into messages and signals
        let messages = parse_data_rows(&rows, &col_map, &bus_type);

        // Determine bus name — prefer sheet-based name for multi-sheet files
        let effective_bus_name = if data_sheets.len() > 1 {
            sheet_name.trim().to_string()
        } else {
            bus_name.clone()
        };

        matrices.push(KMatrix {
            id: Uuid::new_v4(),
            source_file: source_file.clone(),
            source_path: path.to_path_buf(),
            platform: platform.clone(),
            bus_type: bus_type.clone(),
            bus_name: effective_bus_name,
            format: FileFormat::XLSX,
            messages,
            parsed_at: Utc::now(),
        });
    }

    Ok(matrices)
}

/// Convert a calamine Data cell to an Option<String>.
fn cell_to_string(cell: &Data) -> Option<String> {
    match cell {
        Data::Empty => None,
        Data::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Data::Float(f) => {
            // Avoid ".0" suffix for integers
            if *f == f.floor() && f.is_finite() {
                Some(format!("{}", *f as i64))
            } else {
                Some(format!("{f}"))
            }
        }
        Data::Int(i) => Some(format!("{i}")),
        Data::Bool(b) => Some(format!("{b}")),
        Data::Error(e) => Some(format!("#{e:?}")),
        Data::DateTime(dt) => Some(format!("{dt}")),
        Data::DurationIso(d) => Some(d.clone()),
        Data::DateTimeIso(s) => Some(s.clone()),
    }
}

/// Parse the data rows of a sheet into a Vec of Messages.
///
/// In K-Matrix format, each row is a signal. Multiple consecutive signals
/// with the same message name belong to the same message. When a new
/// message name appears, a new Message starts.
fn parse_data_rows(
    rows: &[Vec<Option<String>>],
    col_map: &ColumnMap,
    _bus_type: &BusType,
) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();
    let mut current_msg_name = String::new();

    for row_idx in col_map.data_start_row..rows.len() {
        let row = &rows[row_idx];

        // Extract signal-level fields
        let signal_name = get_field(row, col_map, &MappedField::SignalName);

        // Skip rows that have no signal name (empty rows or separators)
        let signal_name = match signal_name {
            Some(name) if !name.is_empty() && name.to_lowercase() != "nan" => name,
            _ => continue,
        };

        // Skip "void" signals (reserved bit positions)
        if signal_name.to_lowercase() == "void" {
            continue;
        }

        // Extract message name — or reuse the current one if cell is empty
        // (message name is only in the first signal row of each message)
        let msg_name = get_field(row, col_map, &MappedField::MessageName)
            .unwrap_or_else(|| current_msg_name.clone());

        let identifier = get_field(row, col_map, &MappedField::IdentifierHex);

        let signal = Signal {
            name: signal_name,
            comment: get_field(row, col_map, &MappedField::SignalComment),
            description: get_field(row, col_map, &MappedField::Description),
            init_value: get_field(row, col_map, &MappedField::InitValue),
            error_value: get_field(row, col_map, &MappedField::ErrorValue),
            min_raw: get_field(row, col_map, &MappedField::MinRaw),
            max_raw: get_field(row, col_map, &MappedField::MaxRaw),
            physical_value: get_field(row, col_map, &MappedField::PhysicalValue),
            unit: get_field(row, col_map, &MappedField::Unit),
            offset: get_field(row, col_map, &MappedField::Offset),
            scaling: get_field(row, col_map, &MappedField::Scaling),
            raw_value: get_field(row, col_map, &MappedField::RawValue),
            start_bit: get_field(row, col_map, &MappedField::StartBit)
                .and_then(|s| s.parse::<u32>().ok()),
            bit_length: get_field(row, col_map, &MappedField::BitLength)
                .and_then(|s| s.parse::<u32>().ok()),
        };

        // Extract ECU assignments from dynamic columns
        let ecu_assignments = extract_ecu_assignments(row, col_map);

        // Check if we need a new message or append to the current one
        if msg_name != current_msg_name || messages.is_empty() {
            current_msg_name = msg_name.clone();
            messages.push(Message {
                name: msg_name,
                identifier: identifier.clone(),
                signals: vec![signal],
                ecu_assignments,
            });
        } else {
            if let Some(last_msg) = messages.last_mut() {
                last_msg.signals.push(signal);
                // Update identifier if this row has one and the message doesn't yet
                if last_msg.identifier.is_none() {
                    last_msg.identifier = identifier;
                }
                // Merge ECU assignments (avoid duplicates)
                for ecu in ecu_assignments {
                    if !last_msg.ecu_assignments.iter().any(|a| a.ecu_name == ecu.ecu_name && a.role == ecu.role) {
                        last_msg.ecu_assignments.push(ecu);
                    }
                }
            }
        }
    }

    messages
}

/// Get a specific field value from a row using the column map.
fn get_field(
    row: &[Option<String>],
    col_map: &ColumnMap,
    target: &MappedField,
) -> Option<String> {
    for (col_idx, field) in &col_map.mapping {
        if field == target {
            return row
                .get(*col_idx)
                .and_then(|v| v.as_ref())
                .map(|s| s.to_string());
        }
    }
    None
}

/// Extract ECU sender/receiver/router assignments from dynamic ECU columns.
fn extract_ecu_assignments(row: &[Option<String>], col_map: &ColumnMap) -> Vec<EcuAssignment> {
    let mut assignments = Vec::new();

    for (col_idx, field) in &col_map.mapping {
        if let MappedField::EcuColumn(ecu_name) = field {
            if let Some(Some(value)) = row.get(*col_idx) {
                let trimmed = value.trim();
                let role = match trimmed {
                    "S" => Some(EcuRole::Sender),
                    "E" => Some(EcuRole::Receiver),
                    "S*" | "0*" => Some(EcuRole::Router),
                    _ => None,
                };
                if let Some(role) = role {
                    assignments.push(EcuAssignment {
                        ecu_name: ecu_name.clone(),
                        role,
                    });
                }
            }
        }
    }

    assignments
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;
    use std::collections::HashMap;

    // ── cell_to_string ──────────────────────────────────────────────

    #[test]
    fn cell_empty() {
        assert_eq!(cell_to_string(&Data::Empty), None);
    }

    #[test]
    fn cell_string_trimmed() {
        assert_eq!(cell_to_string(&Data::String("  hello  ".into())), Some("hello".into()));
    }

    #[test]
    fn cell_string_whitespace_only() {
        assert_eq!(cell_to_string(&Data::String("   ".into())), None);
    }

    #[test]
    fn cell_float_integer_no_dot_zero() {
        // 42.0 should become "42", not "42.0"
        assert_eq!(cell_to_string(&Data::Float(42.0)), Some("42".into()));
        assert_eq!(cell_to_string(&Data::Float(0.0)), Some("0".into()));
        assert_eq!(cell_to_string(&Data::Float(-7.0)), Some("-7".into()));
    }

    #[test]
    fn cell_float_decimal_preserved() {
        assert_eq!(cell_to_string(&Data::Float(3.14)), Some("3.14".into()));
        assert_eq!(cell_to_string(&Data::Float(0.5)), Some("0.5".into()));
    }

    #[test]
    fn cell_int() {
        assert_eq!(cell_to_string(&Data::Int(99)), Some("99".into()));
        assert_eq!(cell_to_string(&Data::Int(-1)), Some("-1".into()));
    }

    #[test]
    fn cell_bool() {
        assert_eq!(cell_to_string(&Data::Bool(true)), Some("true".into()));
        assert_eq!(cell_to_string(&Data::Bool(false)), Some("false".into()));
    }

    // ── Helper: build a minimal ColumnMap for testing ────────────────

    /// Creates a ColumnMap with columns:
    ///   0=MessageName, 1=IdentifierHex, 2=SignalName, 3=StartBit,
    ///   4=BitLength, 5=SignalComment, 6=EcuColumn("ECU_A"), 7=EcuColumn("ECU_B")
    fn test_column_map() -> ColumnMap {
        let mut mapping = HashMap::new();
        mapping.insert(0, MappedField::MessageName);
        mapping.insert(1, MappedField::IdentifierHex);
        mapping.insert(2, MappedField::SignalName);
        mapping.insert(3, MappedField::StartBit);
        mapping.insert(4, MappedField::BitLength);
        mapping.insert(5, MappedField::SignalComment);
        mapping.insert(6, MappedField::EcuColumn("ECU_A".into()));
        mapping.insert(7, MappedField::EcuColumn("ECU_B".into()));
        ColumnMap {
            mapping,
            data_start_row: 4,
        }
    }

    fn s(val: &str) -> Option<String> {
        Some(val.to_string())
    }

    // ── get_field ───────────────────────────────────────────────────

    #[test]
    fn get_field_returns_value() {
        let col_map = test_column_map();
        let row = vec![s("MSG_01"), s("0x100"), s("SIG_A"), s("0"), s("8"), s("A comment"), None, None];
        assert_eq!(get_field(&row, &col_map, &MappedField::MessageName), Some("MSG_01".into()));
        assert_eq!(get_field(&row, &col_map, &MappedField::SignalName), Some("SIG_A".into()));
        assert_eq!(get_field(&row, &col_map, &MappedField::StartBit), Some("0".into()));
    }

    #[test]
    fn get_field_returns_none_when_missing() {
        let col_map = test_column_map();
        let row = vec![s("MSG_01"), None, s("SIG_A")];
        assert_eq!(get_field(&row, &col_map, &MappedField::IdentifierHex), None);
        // Field not in mapping at all
        assert_eq!(get_field(&row, &col_map, &MappedField::InitValue), None);
    }

    // ── extract_ecu_assignments ─────────────────────────────────────

    #[test]
    fn ecu_assignments_all_roles() {
        let col_map = test_column_map();
        // col 6 = ECU_A → "S" (Sender), col 7 = ECU_B → "E" (Receiver)
        let row = vec![None, None, None, None, None, None, s("S"), s("E")];
        let assignments = extract_ecu_assignments(&row, &col_map);
        assert_eq!(assignments.len(), 2);
        assert!(assignments.iter().any(|a| a.ecu_name == "ECU_A" && a.role == EcuRole::Sender));
        assert!(assignments.iter().any(|a| a.ecu_name == "ECU_B" && a.role == EcuRole::Receiver));
    }

    #[test]
    fn ecu_assignments_router_variants() {
        let col_map = test_column_map();
        let row = vec![None, None, None, None, None, None, s("S*"), s("0*")];
        let assignments = extract_ecu_assignments(&row, &col_map);
        assert_eq!(assignments.len(), 2);
        assert!(assignments.iter().all(|a| a.role == EcuRole::Router));
    }

    #[test]
    fn ecu_assignments_unknown_values_skipped() {
        let col_map = test_column_map();
        let row = vec![None, None, None, None, None, None, s("X"), s("")];
        let assignments = extract_ecu_assignments(&row, &col_map);
        assert!(assignments.is_empty());
    }

    #[test]
    fn ecu_assignments_empty_cells_skipped() {
        let col_map = test_column_map();
        let row = vec![None, None, None, None, None, None, None, None];
        let assignments = extract_ecu_assignments(&row, &col_map);
        assert!(assignments.is_empty());
    }

    // ── parse_data_rows ─────────────────────────────────────────────

    fn make_rows_with_headers(data_rows: Vec<Vec<Option<String>>>) -> Vec<Vec<Option<String>>> {
        // Rows 0-3 are headers (ignored by parse_data_rows which starts at data_start_row=4)
        let mut rows = vec![vec![None]; 4];
        rows.extend(data_rows);
        rows
    }

    #[test]
    fn parse_single_message_two_signals() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            // Row 4: first signal of message
            vec![s("MSG_01"), s("0x100"), s("SIG_A"), s("0"), s("8"), s("crc"), s("S"), s("E")],
            // Row 5: second signal, same message (empty message name)
            vec![None, None, s("SIG_B"), s("8"), s("4"), None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].name, "MSG_01");
        assert_eq!(messages[0].identifier, Some("0x100".into()));
        assert_eq!(messages[0].signals.len(), 2);
        assert_eq!(messages[0].signals[0].name, "SIG_A");
        assert_eq!(messages[0].signals[0].start_bit, Some(0));
        assert_eq!(messages[0].signals[0].bit_length, Some(8));
        assert_eq!(messages[0].signals[0].comment, Some("crc".into()));
        assert_eq!(messages[0].signals[1].name, "SIG_B");
        assert_eq!(messages[0].signals[1].start_bit, Some(8));
        // ECU assignments merged from first row
        assert!(!messages[0].ecu_assignments.is_empty());
    }

    #[test]
    fn parse_two_messages() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            vec![s("MSG_01"), s("0x100"), s("SIG_A"), s("0"), s("8"), None, None, None],
            vec![s("MSG_02"), s("0x200"), s("SIG_B"), s("0"), s("16"), None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].name, "MSG_01");
        assert_eq!(messages[1].name, "MSG_02");
    }

    #[test]
    fn parse_skips_void_signals() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            vec![s("MSG_01"), None, s("SIG_A"), s("0"), s("8"), None, None, None],
            vec![None, None, s("void"), s("8"), s("4"), None, None, None],
            vec![None, None, s("VOID"), s("12"), s("4"), None, None, None],
            vec![None, None, s("SIG_B"), s("16"), s("8"), None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].signals.len(), 2);
        assert_eq!(messages[0].signals[0].name, "SIG_A");
        assert_eq!(messages[0].signals[1].name, "SIG_B");
    }

    #[test]
    fn parse_skips_nan_signals() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            vec![s("MSG_01"), None, s("SIG_A"), s("0"), s("8"), None, None, None],
            vec![None, None, s("nan"), None, None, None, None, None],
            vec![None, None, s("NaN"), None, None, None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages[0].signals.len(), 1);
        assert_eq!(messages[0].signals[0].name, "SIG_A");
    }

    #[test]
    fn parse_skips_empty_signal_names() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            vec![s("MSG_01"), None, s("SIG_A"), s("0"), s("8"), None, None, None],
            vec![None, None, None, None, None, None, None, None], // empty row
            vec![None, None, s("SIG_B"), s("8"), s("4"), None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages[0].signals.len(), 2);
    }

    #[test]
    fn parse_message_name_carry_over() {
        // When message name cell is empty, the previous message name is reused
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            vec![s("MSG_01"), s("0x100"), s("SIG_A"), s("0"), s("8"), None, None, None],
            vec![None, None, s("SIG_B"), s("8"), s("4"), None, None, None], // same message
            vec![None, None, s("SIG_C"), s("12"), s("4"), None, None, None], // same message
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].signals.len(), 3);
    }

    #[test]
    fn parse_ecu_assignments_merged_no_duplicates() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            // Both rows have the same ECU assignments — should not duplicate
            vec![s("MSG_01"), None, s("SIG_A"), s("0"), s("8"), None, s("S"), s("E")],
            vec![None, None, s("SIG_B"), s("8"), s("4"), None, s("S"), s("E")],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages[0].ecu_assignments.len(), 2); // not 4
    }

    #[test]
    fn parse_identifier_filled_from_later_row() {
        let col_map = test_column_map();
        let rows = make_rows_with_headers(vec![
            // First row has no identifier
            vec![s("MSG_01"), None, s("SIG_A"), s("0"), s("8"), None, None, None],
            // Second row has the identifier
            vec![None, s("0xFF"), s("SIG_B"), s("8"), s("4"), None, None, None],
        ]);

        let messages = parse_data_rows(&rows, &col_map, &BusType::CAN);
        assert_eq!(messages[0].identifier, Some("0xFF".into()));
    }
}
