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
