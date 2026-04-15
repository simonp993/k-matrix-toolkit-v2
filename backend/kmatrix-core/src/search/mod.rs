//! Search engine — full-text search across all loaded K-Matrices.
//!
//! Builds a flat index of search hits (one per signal per message per matrix)
//! and performs case-insensitive substring matching across multiple fields.

use serde::{Deserialize, Serialize};

use crate::{EcuRole, KMatrix};

/// A single flat search result row — one signal with its context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub matrix_id: String,
    pub source_file: String,
    pub platform: Option<String>,
    pub bus_type: String,
    pub bus_name: String,
    pub message_name: String,
    pub identifier: Option<String>,
    pub signal_name: String,
    pub comment: Option<String>,
    pub description: Option<String>,
    pub init_value: Option<String>,
    pub error_value: Option<String>,
    pub min_raw: Option<String>,
    pub max_raw: Option<String>,
    pub physical_value: Option<String>,
    pub unit: Option<String>,
    pub offset: Option<String>,
    pub scaling: Option<String>,
    pub start_bit: Option<u32>,
    pub bit_length: Option<u32>,
    pub ecu_sender: Option<String>,
    pub ecu_receivers: Vec<String>,
}

/// Optional filters for narrowing search results.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchFilter {
    pub platform: Option<String>,
    pub bus_type: Option<String>,
    pub bus_name: Option<String>,
    pub file_type: Option<String>,
}

/// Build a flat index of all signals from all loaded K-Matrices.
pub fn build_index(matrices: &[KMatrix]) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    for km in matrices {
        let platform_str = km.platform.as_ref().map(|p: &crate::Platform| p.to_string());
        let bus_type_str = km.bus_type.to_string();
        for msg in &km.messages {
            // Find sender and receivers from ECU assignments
            let ecu_sender = msg
                .ecu_assignments
                .iter()
                .find(|a| {
                    matches!(a.role, EcuRole::Sender | EcuRole::Router)
                })
                .map(|a| a.ecu_name.clone());

            let ecu_receivers: Vec<String> = msg
                .ecu_assignments
                .iter()
                .filter(|a| a.role == EcuRole::Receiver)
                .map(|a| a.ecu_name.clone())
                .collect();

            for sig in &msg.signals {
                hits.push(SearchHit {
                    matrix_id: km.id.to_string(),
                    source_file: km.source_file.clone(),
                    platform: platform_str.clone(),
                    bus_type: bus_type_str.clone(),
                    bus_name: km.bus_name.clone(),
                    message_name: msg.name.clone(),
                    identifier: msg.identifier.clone(),
                    signal_name: sig.name.clone(),
                    comment: sig.comment.clone(),
                    description: sig.description.clone(),
                    init_value: sig.init_value.clone(),
                    error_value: sig.error_value.clone(),
                    min_raw: sig.min_raw.clone(),
                    max_raw: sig.max_raw.clone(),
                    physical_value: sig.physical_value.clone(),
                    unit: sig.unit.clone(),
                    offset: sig.offset.clone(),
                    scaling: sig.scaling.clone(),
                    start_bit: sig.start_bit,
                    bit_length: sig.bit_length,
                    ecu_sender: ecu_sender.clone(),
                    ecu_receivers: ecu_receivers.clone(),
                });
            }
        }
    }
    hits
}

/// Search the flat index for a query string with optional filters.
///
/// Matches case-insensitively against: signal name, message name, comment,
/// description, identifier, ECU names, bus name.
pub fn search(index: &[SearchHit], query: &str, filter: &SearchFilter) -> Vec<SearchHit> {
    let q = query.to_lowercase();

    index
        .iter()
        .filter(|hit| {
            // Apply filters first
            if let Some(ref pf) = filter.platform {
                if let Some(ref hp) = hit.platform {
                    if !hp.eq_ignore_ascii_case(pf) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Some(ref bf) = filter.bus_type {
                if !hit.bus_type.eq_ignore_ascii_case(bf) {
                    return false;
                }
            }
            if let Some(ref bn) = filter.bus_name {
                if !hit.bus_name.eq_ignore_ascii_case(bn) {
                    return false;
                }
            }
            if let Some(ref ft) = filter.file_type {
                let ext = hit.source_file.rsplit('.').next().unwrap_or("").to_lowercase();
                if !ext.eq_ignore_ascii_case(ft) {
                    return false;
                }
            }
            // Empty query matches everything (after filters)
            if q.is_empty() {
                return true;
            }

            // Match query against multiple fields
            let fields: Vec<&str> = [
                Some(hit.signal_name.as_str()),
                Some(hit.message_name.as_str()),
                hit.comment.as_deref(),
                hit.description.as_deref(),
                hit.identifier.as_deref(),
                hit.ecu_sender.as_deref(),
                Some(hit.bus_name.as_str()),
                Some(hit.source_file.as_str()),
            ]
            .into_iter()
            .flatten()
            .collect();

            // Also match ECU receivers
            let receiver_match = hit
                .ecu_receivers
                .iter()
                .any(|r| r.to_lowercase().contains(&q));

            fields
                .iter()
                .any(|f| f.to_lowercase().contains(&q))
                || receiver_match
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_test_matrix() -> KMatrix {
        KMatrix {
            id: Uuid::new_v4(),
            source_file: "test_can.dbc".into(),
            source_path: "/tmp/test_can.dbc".into(),
            platform: Some(Platform::MLBevo2),
            bus_type: BusType::CAN,
            bus_name: "EFP_SUBCAN02".into(),
            format: FileFormat::DBC,
            messages: vec![
                Message {
                    name: "CIMU_01".into(),
                    identifier: Some("0x66".into()),
                    signals: vec![
                        Signal {
                            name: "CIMU_01_CRC".into(),
                            comment: Some("CRC checksum".into()),
                            description: None,
                            init_value: None,
                            error_value: None,
                            min_raw: None,
                            max_raw: None,
                            physical_value: None,
                            unit: None,
                            offset: Some("0".into()),
                            scaling: Some("1".into()),
                            raw_value: None,
                            start_bit: Some(0),
                            bit_length: Some(8),
                        },
                        Signal {
                            name: "CIMU_01_BZ".into(),
                            comment: Some("Botschaftszähler".into()),
                            description: None,
                            init_value: None,
                            error_value: None,
                            min_raw: None,
                            max_raw: None,
                            physical_value: None,
                            unit: None,
                            offset: Some("0".into()),
                            scaling: Some("1".into()),
                            raw_value: None,
                            start_bit: Some(8),
                            bit_length: Some(4),
                        },
                    ],
                    ecu_assignments: vec![
                        EcuAssignment {
                            ecu_name: "CIMU".into(),
                            role: EcuRole::Sender,
                        },
                        EcuAssignment {
                            ecu_name: "EFP_PAG".into(),
                            role: EcuRole::Receiver,
                        },
                    ],
                },
            ],
            parsed_at: Utc::now(),
        }
    }

    #[test]
    fn search_signal_name() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "CRC", &SearchFilter::default());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_name, "CIMU_01_CRC");
    }

    #[test]
    fn search_case_insensitive() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "cimu", &SearchFilter::default());
        assert_eq!(results.len(), 2); // both signals are in CIMU_01
    }

    #[test]
    fn search_message_name() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "CIMU_01", &SearchFilter::default());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_identifier() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "0x66", &SearchFilter::default());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_ecu_receiver() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "EFP_PAG", &SearchFilter::default());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_comment() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "checksum", &SearchFilter::default());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_name, "CIMU_01_CRC");
    }

    #[test]
    fn search_no_results() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "NONEXISTENT_SIGNAL_XYZ", &SearchFilter::default());
        assert!(results.is_empty());
    }

    #[test]
    fn search_empty_query_returns_all() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let results = search(&index, "", &SearchFilter::default());
        assert!(!results.is_empty());
    }

    #[test]
    fn search_with_bus_filter() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        // Filter for LIN — should return nothing since our test data is CAN
        let filter = SearchFilter {
            bus_type: Some("LIN".into()),
            ..Default::default()
        };
        let results = search(&index, "CIMU", &filter);
        assert!(results.is_empty());

        // Filter for CAN — should return results
        let filter = SearchFilter {
            bus_type: Some("CAN".into()),
            ..Default::default()
        };
        let results = search(&index, "CIMU", &filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_with_platform_filter() {
        let km = make_test_matrix();
        let index = build_index(&[km]);
        let filter = SearchFilter {
            platform: Some("E³ 1.2".into()),
            ..Default::default()
        };
        let results = search(&index, "CIMU", &filter);
        assert!(results.is_empty()); // fixture is MLBevo2

        let filter = SearchFilter {
            platform: Some("MLBevo 2".into()),
            ..Default::default()
        };
        let results = search(&index, "CIMU", &filter);
        assert_eq!(results.len(), 2);
    }
}
