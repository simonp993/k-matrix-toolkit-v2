use std::path::Path;

use anyhow::{Context, Result};
use can_dbc::{Dbc, Transmitter};
use chrono::Utc;
use uuid::Uuid;

use crate::model::{
    EcuAssignment, EcuRole, FileFormat, KMatrix, Message, Signal, extract_metadata,
};

/// Parse a Vector CAN DBC file into a KMatrix.
///
/// Production VW DBC files often contain extensions that `can-dbc` does not
/// fully support (BA_DEF_REL_, BO_TX_BU_, etc.). When the library returns an
/// `Incomplete` error, it still provides the partially-parsed DBC with all
/// messages and signals — we accept that partial result.
pub fn parse_dbc(path: &Path) -> Result<Vec<KMatrix>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let dbc = match Dbc::try_from(content.as_str()) {
        Ok(dbc) => dbc,
        Err(can_dbc::Error::Incomplete(partial_dbc, _remaining)) => {
            tracing::warn!(
                "DBC file {} was only partially parsed (unsupported extensions), using partial result",
                path.display()
            );
            partial_dbc
        }
        Err(e) => {
            let dbg = format!("{:?}", e);
            let truncated = if dbg.len() > 500 { &dbg[..500] } else { &dbg };
            anyhow::bail!("Failed to parse DBC {}: {}", path.display(), truncated);
        }
    };

    let (platform, bus_type, bus_name) = extract_metadata(path);
    let source_file = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut messages = Vec::new();

    for msg in dbc.messages() {
        let msg_name = msg.name().to_string();
        let msg_id = msg.id().raw();
        let identifier = format!("0x{:X}", msg_id);

        let mut signals = Vec::new();
        let mut ecu_assignments = Vec::new();

        // Sender ECU
        if let Transmitter::NodeName(name) = msg.transmitter() {
            ecu_assignments.push(EcuAssignment {
                ecu_name: name.clone(),
                role: EcuRole::Sender,
            });
        }

        for sig in msg.signals() {
            let factor = sig.factor;
            let offset = sig.offset;
            let min = sig.min;
            let max = sig.max;
            let unit = sig.unit().to_string();

            // Get signal comment from the DBC comment section
            let comment = dbc.signal_comment(*msg.id(), sig.name())
                .map(|c| c.to_string());

            signals.push(Signal {
                name: sig.name().to_string(),
                comment,
                description: None,
                init_value: None,
                error_value: None,
                min_raw: Some(format!("{min}")),
                max_raw: Some(format!("{max}")),
                physical_value: None,
                unit: if unit.is_empty() { None } else { Some(unit) },
                offset: Some(format!("{offset}")),
                scaling: Some(format!("{factor}")),
                raw_value: None,
                start_bit: Some(sig.start_bit as u32),
                bit_length: Some(sig.size as u32),
            });

            // Receiver ECUs from signal
            for receiver in sig.receivers() {
                if receiver != "Vector__XXX"
                    && !receiver.is_empty()
                    && !ecu_assignments.iter().any(|a| a.ecu_name == *receiver && a.role == EcuRole::Receiver)
                {
                    ecu_assignments.push(EcuAssignment {
                        ecu_name: receiver.clone(),
                        role: EcuRole::Receiver,
                    });
                }
            }
        }

        messages.push(Message {
            name: msg_name,
            identifier: Some(identifier),
            signals,
            ecu_assignments,
        });
    }

    Ok(vec![KMatrix {
        id: Uuid::new_v4(),
        source_file,
        source_path: path.to_path_buf(),
        platform,
        bus_type,
        bus_name,
        format: FileFormat::DBC,
        messages,
        parsed_at: Utc::now(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Minimal valid DBC content with one message and two signals.
    const MINIMAL_DBC: &str = r#"
VERSION ""

NS_ :

BS_:

BU_: ECU1 ECU2

BO_ 256 TestMsg: 8 ECU1
 SG_ CRC_Signal : 0|8@1+ (1,0) [0|255] "unit" ECU2
 SG_ Counter_Signal : 8|4@1+ (1,0) [0|15] "" ECU1,ECU2

"#;

    /// DBC content with Vector__XXX placeholder receiver (should be filtered out).
    const DBC_WITH_VECTOR_XXX: &str = r#"
VERSION ""

NS_ :

BS_:

BU_: Sender1

BO_ 512 Msg_A: 8 Sender1
 SG_ Sig_X : 0|8@1+ (0.5,10) [0|127.5] "km/h" Vector__XXX

"#;

    fn write_temp_dbc(content: &str, name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        (dir, path)
    }

    #[test]
    fn parse_minimal_dbc() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "test.dbc");
        let matrices = parse_dbc(&path).expect("should parse minimal DBC");

        assert_eq!(matrices.len(), 1);
        let km = &matrices[0];
        assert_eq!(km.format, FileFormat::DBC);
        assert!(!km.messages.is_empty());
    }

    #[test]
    fn dbc_message_id_is_hex() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];

        assert_eq!(msg.name, "TestMsg");
        // 256 decimal = 0x100
        assert_eq!(msg.identifier, Some("0x100".into()));
    }

    #[test]
    fn dbc_signal_properties_extracted() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];

        let crc = msg.signals.iter().find(|s| s.name == "CRC_Signal").unwrap();
        assert_eq!(crc.start_bit, Some(0));
        assert_eq!(crc.bit_length, Some(8));
        assert_eq!(crc.scaling, Some("1".into()));
        assert_eq!(crc.offset, Some("0".into()));
        assert_eq!(crc.min_raw, Some("0".into()));
        assert_eq!(crc.max_raw, Some("255".into()));
        assert_eq!(crc.unit, Some("unit".into()));

        let counter = msg.signals.iter().find(|s| s.name == "Counter_Signal").unwrap();
        assert_eq!(counter.start_bit, Some(8));
        assert_eq!(counter.bit_length, Some(4));
    }

    #[test]
    fn dbc_sender_ecu_extracted() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];

        let sender = msg.ecu_assignments.iter().find(|a| a.role == EcuRole::Sender);
        assert!(sender.is_some());
        assert_eq!(sender.unwrap().ecu_name, "ECU1");
    }

    #[test]
    fn dbc_receiver_ecus_extracted() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];

        let receivers: Vec<&str> = msg
            .ecu_assignments
            .iter()
            .filter(|a| a.role == EcuRole::Receiver)
            .map(|a| a.ecu_name.as_str())
            .collect();
        assert!(receivers.contains(&"ECU2"));
    }

    #[test]
    fn dbc_skips_vector_xxx_receiver() {
        let (_dir, path) = write_temp_dbc(DBC_WITH_VECTOR_XXX, "vec_test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];

        let receivers: Vec<&str> = msg
            .ecu_assignments
            .iter()
            .filter(|a| a.role == EcuRole::Receiver)
            .map(|a| a.ecu_name.as_str())
            .collect();
        assert!(
            !receivers.contains(&"Vector__XXX"),
            "Vector__XXX placeholder should be filtered out"
        );
    }

    #[test]
    fn dbc_signal_with_factor_and_offset() {
        let (_dir, path) = write_temp_dbc(DBC_WITH_VECTOR_XXX, "factor_test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let sig = &matrices[0].messages[0].signals[0];

        assert_eq!(sig.name, "Sig_X");
        assert_eq!(sig.scaling, Some("0.5".into()));
        assert_eq!(sig.offset, Some("10".into()));
        assert_eq!(sig.unit, Some("km/h".into()));
    }

    #[test]
    fn dbc_empty_unit_becomes_none() {
        let (_dir, path) = write_temp_dbc(MINIMAL_DBC, "unit_test.dbc");
        let matrices = parse_dbc(&path).unwrap();
        let msg = &matrices[0].messages[0];
        let counter = msg.signals.iter().find(|s| s.name == "Counter_Signal").unwrap();
        assert_eq!(counter.unit, None); // empty string unit → None
    }

    #[test]
    fn dbc_nonexistent_file_returns_error() {
        let result = parse_dbc(Path::new("/nonexistent/file.dbc"));
        assert!(result.is_err());
    }

    #[test]
    fn dbc_metadata_from_path() {
        let dir = tempfile::tempdir().unwrap();
        // Path containing "MLBevo" and "CAN" should be detected
        let subdir = dir.path().join("MLBevo 2/K-Matrix/CAN");
        std::fs::create_dir_all(&subdir).unwrap();
        let path = subdir.join("MLBevo_Gen2_Test_KCAN_KMatrix.dbc");
        std::fs::write(&path, MINIMAL_DBC).unwrap();

        let matrices = parse_dbc(&path).unwrap();
        let km = &matrices[0];
        assert!(matches!(km.platform, Some(crate::model::Platform::MLBevo2)));
        assert_eq!(km.bus_type, crate::model::BusType::CAN);
    }
}
