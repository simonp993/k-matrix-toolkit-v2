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
