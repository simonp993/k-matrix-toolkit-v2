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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a cell value.
    fn s(val: &str) -> Option<String> {
        Some(val.to_string())
    }

    // ── build_column_map ────────────────────────────────────────────

    #[test]
    fn build_column_map_typical_canfd() {
        // Simulates a real CAN FD K-Matrix header structure.
        let row0 = vec![
            s("Botschaften"), None, s("Signale"), None, None, None,
            s("Wertebereich"), None, None, None,
            s("Sender - Empfänger"), None, None,
        ];
        let row1 = vec![
            s("Botschaft"), s("Identifier [hex]"),
            s("Signal"), s("StartBit"), s("Signal Länge [Bits]"), s("Signalkommentar"),
            s("Min Rohwert [dez]"), s("Max Rohwert [dez]"), s("phy Werte [dez]"), s("Einheit"),
            s("HCP1"), s("Gateway"), s("BCM2"),
        ];
        let row2: Vec<Option<String>> = vec![
            None, None, None, None, None, None,
            s("Min Rohwert [dez]"), s("Max Rohwert [dez]"), s("phy Werte [dez]"), s("Einheit"),
            None, None, None,
        ];
        let row3: Vec<Option<String>> = vec![None; 13];

        let col_map = build_column_map(&[row0, row1, row2, row3]);

        assert_eq!(col_map.mapping[&0], MappedField::MessageName);
        assert_eq!(col_map.mapping[&1], MappedField::IdentifierHex);
        assert_eq!(col_map.mapping[&2], MappedField::SignalName);
        assert_eq!(col_map.mapping[&3], MappedField::StartBit);
        assert_eq!(col_map.mapping[&4], MappedField::BitLength);
        assert_eq!(col_map.mapping[&5], MappedField::SignalComment);
        assert_eq!(col_map.mapping[&6], MappedField::MinRaw);
        assert_eq!(col_map.mapping[&7], MappedField::MaxRaw);
        assert_eq!(col_map.mapping[&8], MappedField::PhysicalValue);
        assert_eq!(col_map.mapping[&9], MappedField::Unit);
        assert_eq!(col_map.mapping[&10], MappedField::EcuColumn("HCP1".into()));
        assert_eq!(col_map.mapping[&11], MappedField::EcuColumn("Gateway".into()));
        assert_eq!(col_map.mapping[&12], MappedField::EcuColumn("BCM2".into()));
        assert_eq!(col_map.data_start_row, 4);
    }

    #[test]
    fn build_column_map_fewer_than_3_rows() {
        let row0 = vec![s("Botschaften")];
        let row1 = vec![s("Botschaft")];
        let col_map = build_column_map(&[row0, row1]);

        assert!(col_map.mapping.is_empty());
        assert_eq!(col_map.data_start_row, 0);
    }

    #[test]
    fn data_start_row_with_content_in_row3() {
        let row0 = vec![s("Botschaften")];
        let row1 = vec![s("Botschaft")];
        let row2 = vec![None];
        let row3 = vec![s("SomeData")]; // row 3 has content → start at 3

        let col_map = build_column_map(&[row0, row1, row2, row3]);
        assert_eq!(col_map.data_start_row, 3);
    }

    #[test]
    fn data_start_row_empty_row3() {
        let row0 = vec![s("Botschaften")];
        let row1 = vec![s("Botschaft")];
        let row2 = vec![None];
        let row3: Vec<Option<String>> = vec![None]; // row 3 empty → start at 4

        let col_map = build_column_map(&[row0, row1, row2, row3]);
        assert_eq!(col_map.data_start_row, 4);
    }

    // ── map_column_name (message-level) ─────────────────────────────

    #[test]
    fn message_name_variants() {
        assert_eq!(map_column_name("Botschaft", "", false), MappedField::MessageName);
        assert_eq!(map_column_name("Botschaften", "", false), MappedField::MessageName);
        assert_eq!(map_column_name("PDU", "", false), MappedField::MessageName);
        assert_eq!(map_column_name("Frame", "", false), MappedField::MessageName);
    }

    #[test]
    fn identifier_hex_variants() {
        assert_eq!(map_column_name("Identifier [hex]", "", false), MappedField::IdentifierHex);
        assert_eq!(map_column_name("PDU-ID [hex]", "", false), MappedField::IdentifierHex);
    }

    // ── map_column_name (signal-level) ──────────────────────────────

    #[test]
    fn signal_field_names() {
        assert_eq!(map_column_name("Signal", "", false), MappedField::SignalName);
        assert_eq!(map_column_name("Signale", "", false), MappedField::SignalName);
        assert_eq!(map_column_name("StartBit", "", false), MappedField::StartBit);
        assert_eq!(map_column_name("Signal Länge [Bits]", "", false), MappedField::BitLength);
        assert_eq!(map_column_name("InitWert roh [dez]", "", false), MappedField::InitValue);
        assert_eq!(map_column_name("FehlerWert roh [dez]", "", false), MappedField::ErrorValue);
        assert_eq!(map_column_name("Signalkommentar", "", false), MappedField::SignalComment);
    }

    #[test]
    fn unknown_column_is_ignored() {
        assert_eq!(map_column_name("SomeRandomColumn", "", false), MappedField::Ignored);
    }

    // ── map_value_range_sub_header (Wertebereich) ───────────────────

    #[test]
    fn wertebereich_sub_headers() {
        assert_eq!(map_value_range_sub_header("Min Rohwert [dez]", ""), MappedField::MinRaw);
        assert_eq!(map_value_range_sub_header("Max Rohwert [dez]", ""), MappedField::MaxRaw);
        assert_eq!(map_value_range_sub_header("phy Werte [dez]", ""), MappedField::PhysicalValue);
        assert_eq!(map_value_range_sub_header("phy. Werte [dez]", ""), MappedField::PhysicalValue);
        assert_eq!(map_value_range_sub_header("Einheit", ""), MappedField::Unit);
        assert_eq!(map_value_range_sub_header("Offset", ""), MappedField::Offset);
        assert_eq!(map_value_range_sub_header("Skalierung", ""), MappedField::Scaling);
        assert_eq!(map_value_range_sub_header("Rohwert [dez]", ""), MappedField::RawValue);
        assert_eq!(map_value_range_sub_header("Beschreibung", ""), MappedField::Description);
    }

    #[test]
    fn wertebereich_falls_back_to_col_name() {
        // When sub_name is empty, uses col_name instead
        assert_eq!(map_value_range_sub_header("", "Min Rohwert [dez]"), MappedField::MinRaw);
        assert_eq!(map_value_range_sub_header("", "Einheit"), MappedField::Unit);
    }

    #[test]
    fn wertebereich_unknown_is_ignored() {
        assert_eq!(map_value_range_sub_header("SomethingElse", ""), MappedField::Ignored);
    }

    // ── map_column_name delegates to Wertebereich ───────────────────

    #[test]
    fn in_wertebereich_uses_sub_header() {
        // When in_wertebereich=true, sub_name is used for mapping
        assert_eq!(
            map_column_name("SomeGroup", "Min Rohwert [dez]", true),
            MappedField::MinRaw
        );
        assert_eq!(
            map_column_name("SomeGroup", "Einheit", true),
            MappedField::Unit
        );
    }

    // ── Direct min/max rohwert outside Wertebereich ─────────────────

    #[test]
    fn min_max_rohwert_outside_wertebereich() {
        assert_eq!(
            map_column_name("Min Rohwert [dez]", "", false),
            MappedField::MinRaw
        );
        assert_eq!(
            map_column_name("Max Rohwert [dez]", "", false),
            MappedField::MaxRaw
        );
        // Also via sub_name
        assert_eq!(
            map_column_name("SomeCol", "Min Rohwert [dez]", false),
            MappedField::MinRaw
        );
    }

    // ── ECU column detection in build_column_map ────────────────────

    #[test]
    fn ecu_columns_only_in_sender_receiver_group() {
        let row0 = vec![s("Sender - Empfänger"), None, None];
        let row1 = vec![s("ECU_A"), s("ECU_B"), s("ECU_C")];
        let row2: Vec<Option<String>> = vec![None, None, None];
        let row3: Vec<Option<String>> = vec![None, None, None];

        let col_map = build_column_map(&[row0, row1, row2, row3]);

        assert_eq!(col_map.mapping[&0], MappedField::EcuColumn("ECU_A".into()));
        assert_eq!(col_map.mapping[&1], MappedField::EcuColumn("ECU_B".into()));
        assert_eq!(col_map.mapping[&2], MappedField::EcuColumn("ECU_C".into()));
    }

    #[test]
    fn empty_ecu_column_name_is_ignored() {
        // If an ECU column has no name in row1, it should NOT become EcuColumn("")
        let row0 = vec![s("Sender - Empfänger"), None];
        let row1 = vec![s("ECU_A"), None]; // second col has no name
        let row2: Vec<Option<String>> = vec![None, None];
        let row3: Vec<Option<String>> = vec![None, None];

        let col_map = build_column_map(&[row0, row1, row2, row3]);

        assert_eq!(col_map.mapping[&0], MappedField::EcuColumn("ECU_A".into()));
        // col 1 should be Ignored (empty col_name in sender-receiver group)
        assert_eq!(col_map.mapping[&1], MappedField::Ignored);
    }
}
