use std::collections::HashMap;

/// Maps Excel column indices to unified field names.
///
/// K-Matrix Excel files have a multi-row header (rows 0-3):
/// - Row 0: Group headers (Botschaften, Signale, Wertebereich, Sender - Empfänger)
/// - Row 1: Column names (the real field names)
/// - Row 2: Sub-headers for physical values section
/// - Row 3: Often empty
/// - Row 4+: Data
///
/// The Sender-Empfänger columns are dynamic — one column per ECU.
/// ECU columns contain "S" (sender), "E" (receiver), "S*" (router), or empty.

/// The known column groups and their field names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappedField {
    // Message-level fields (Botschaften)
    MessageName,
    IdentifierHex,

    // Signal-level fields (Signale)
    SignalName,
    StartBit,
    BitLength,
    InitValue,
    ErrorValue,

    // Value range fields (Wertebereich) — from row 2 sub-headers
    MinRaw,
    MaxRaw,
    PhysicalValue,
    Unit,
    Offset,
    Scaling,
    RawValue,
    Description,

    // Extras
    SignalComment,

    // Dynamic ECU columns
    EcuColumn(String),

    /// Columns we recognize but don't need to extract
    Ignored,
}

/// Result of analyzing the header rows of a K-Matrix sheet.
#[derive(Debug, Clone)]
pub struct ColumnMap {
    /// Column index → mapped field
    pub mapping: HashMap<usize, MappedField>,
    /// First data row (after headers)
    pub data_start_row: usize,
}

/// Build a column map from the first few rows of a sheet.
///
/// `header_rows` should contain at least 3 rows (indices 0, 1, 2).
/// Each row is a Vec of cell values as Option<String>.
pub fn build_column_map(header_rows: &[Vec<Option<String>>]) -> ColumnMap {
    let mut mapping = HashMap::new();

    if header_rows.len() < 3 {
        return ColumnMap {
            mapping,
            data_start_row: 0,
        };
    }

    let row0 = &header_rows[0]; // Group headers
    let row1 = &header_rows[1]; // Column names
    let row2 = &header_rows[2]; // Sub-headers (for Wertebereich)

    let num_cols = row0.len().max(row1.len()).max(row2.len());

    // Track which group we're in (from row 0)
    let mut current_group;
    let mut in_sender_receiver = false;
    let mut in_wertebereich = false;

    for col in 0..num_cols {
        // Update group from row 0
        if let Some(Some(val)) = row0.get(col) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                current_group = trimmed.to_string();
                in_sender_receiver = current_group.contains("Sender");
                in_wertebereich = current_group.contains("Wertebereich")
                    || current_group.contains("Physikalische");
            }
        }

        let col_name = row1
            .get(col)
            .and_then(|v| v.as_ref())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let sub_name = row2
            .get(col)
            .and_then(|v| v.as_ref())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let field = if in_sender_receiver && !col_name.is_empty() {
            // Dynamic ECU column — the column name IS the ECU name
            MappedField::EcuColumn(col_name.clone())
        } else {
            map_column_name(&col_name, &sub_name, in_wertebereich)
        };

        mapping.insert(col, field);
    }

    ColumnMap {
        mapping,
        data_start_row: find_data_start_row(header_rows),
    }
}

fn map_column_name(col_name: &str, sub_name: &str, in_wertebereich: bool) -> MappedField {
    let lower = col_name.to_lowercase();

    // Message-level
    if lower == "botschaft" || lower == "botschaften" {
        return MappedField::MessageName;
    }
    if lower.contains("identifier") && lower.contains("hex") {
        return MappedField::IdentifierHex;
    }
    if lower.contains("pdu-id") && lower.contains("hex") {
        return MappedField::IdentifierHex;
    }
    if lower == "pdu" {
        return MappedField::MessageName;
    }
    if lower == "frame" {
        return MappedField::MessageName;
    }

    // Signal-level
    if lower == "signal" || lower == "signale" {
        return MappedField::SignalName;
    }
    if lower == "startbit" {
        return MappedField::StartBit;
    }
    if lower.contains("signal") && lower.contains("länge") {
        return MappedField::BitLength;
    }
    if lower.contains("initwert") {
        return MappedField::InitValue;
    }
    if lower.contains("fehlerwert") {
        return MappedField::ErrorValue;
    }
    if lower == "signalkommentar" {
        return MappedField::SignalComment;
    }

    // Value range — use sub-header (row 2) if we're in the Wertebereich group
    if in_wertebereich {
        return map_value_range_sub_header(sub_name, col_name);
    }

    // The value range columns can also appear directly as column names
    let sub_lower = sub_name.to_lowercase();
    if lower.contains("min rohwert") || sub_lower.contains("min rohwert") {
        return MappedField::MinRaw;
    }
    if lower.contains("max rohwert") || sub_lower.contains("max rohwert") {
        return MappedField::MaxRaw;
    }

    MappedField::Ignored
}

fn map_value_range_sub_header(sub_name: &str, col_name: &str) -> MappedField {
    let name = if !sub_name.is_empty() {
        sub_name
    } else {
        col_name
    };
    let lower = name.to_lowercase();

    if lower.contains("min rohwert") {
        MappedField::MinRaw
    } else if lower.contains("max rohwert") {
        MappedField::MaxRaw
    } else if lower.contains("phy werte") || lower.contains("phy. werte") {
        MappedField::PhysicalValue
    } else if lower.contains("einheit") {
        MappedField::Unit
    } else if lower == "offset" {
        MappedField::Offset
    } else if lower.contains("skalierung") {
        MappedField::Scaling
    } else if lower.contains("rohwert") {
        MappedField::RawValue
    } else if lower.contains("beschreibung") {
        MappedField::Description
    } else {
        MappedField::Ignored
    }
}

/// Find the first row that contains actual data (not headers).
/// Typically row 4 (0-indexed), but we verify by looking for non-header content.
fn find_data_start_row(header_rows: &[Vec<Option<String>>]) -> usize {
    // In all observed K-Matrix files, data starts at row index 4
    // (rows 0-3 are multi-level headers)
    // But if row 3 has content that looks like data, start at 3
    if header_rows.len() > 3 {
        let row3 = &header_rows[3];
        let has_content = row3.iter().any(|cell| {
            cell.as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        });
        if has_content {
            // Check if it looks like a header or data
            // Headers typically have text like column names; data has IDs/numbers
            return 3;
        }
    }
    4
}
