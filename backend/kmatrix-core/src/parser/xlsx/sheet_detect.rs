/// Detect the correct data sheet in an Excel workbook.
///
/// K-Matrix Excel files typically have:
/// - "Deckblatt" (cover sheet)
/// - "Inhalt" (table of contents)
/// - The actual data sheet (named after the bus, e.g. "HCP1_CANFD01 ")
///
/// NOTE: Sheet names often have trailing spaces.

/// Returns the names of sheets that likely contain K-Matrix data.
pub fn detect_data_sheets(sheet_names: &[String]) -> Vec<String> {
    let skip = ["deckblatt", "inhalt", "inhalte", "inhalteid"];

    sheet_names
        .iter()
        .filter(|name| {
            let lower = name.trim().to_lowercase();
            // Skip known non-data sheets
            if skip.iter().any(|s| lower == *s) {
                return false;
            }
            // Skip schedule table and node attribute sheets (LIN)
            if lower.ends_with(" st") || lower.ends_with(" na") {
                return false;
            }
            // Skip "Inhalt (Kanal X)" sheets (FlexRay TOC)
            if lower.starts_with("inhalt") {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canfd_sheets() {
        let sheets = vec![
            "Deckblatt".to_string(),
            "Inhalt".to_string(),
            "HCP1_CANFD01 ".to_string(),
        ];
        let data = detect_data_sheets(&sheets);
        assert_eq!(data, vec!["HCP1_CANFD01 "]);
    }

    #[test]
    fn test_flexray_sheets() {
        let sheets = vec![
            "Deckblatt".to_string(),
            "Inhalt (Kanal A)".to_string(),
            "HCP1_FlexRay_A ".to_string(),
            "Inhalt (Kanal B)".to_string(),
            "HCP1_FlexRay_B ".to_string(),
        ];
        let data = detect_data_sheets(&sheets);
        assert_eq!(data.len(), 2);
        assert!(data.contains(&"HCP1_FlexRay_A ".to_string()));
        assert!(data.contains(&"HCP1_FlexRay_B ".to_string()));
    }

    #[test]
    fn test_lin_sheets() {
        let sheets = vec![
            "Deckblatt".to_string(),
            "Inhalt".to_string(),
            "HCP1_LIN01 ".to_string(),
            "HCP1_LIN01 ST".to_string(),
            "HCP1_LIN01 NA".to_string(),
        ];
        let data = detect_data_sheets(&sheets);
        assert_eq!(data, vec!["HCP1_LIN01 "]);
    }
}
