use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::enums::{BusType, EcuRole, FileFormat, Platform};

/// A parsed K-Matrix file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KMatrix {
    pub id: Uuid,
    /// Original filename (e.g. "E3_1_2_Premium_HCP1_CANFD01_KMatrix_Module_V12.xlsx")
    pub source_file: String,
    /// Full path to the source file
    pub source_path: PathBuf,
    /// Detected vehicle platform
    pub platform: Option<Platform>,
    /// Bus type (CAN, CAN FD, LIN, Ethernet, FlexRay, etc.)
    pub bus_type: BusType,
    /// Bus name extracted from filename/sheet (e.g. "HCP1_CANFD01")
    pub bus_name: String,
    /// Source file format
    pub format: FileFormat,
    /// All messages in this K-Matrix
    pub messages: Vec<Message>,
    /// When this file was parsed
    pub parsed_at: DateTime<Utc>,
}

/// A message (Botschaft) or PDU on the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Botschaft / PDU / Frame name
    pub name: String,
    /// Identifier [hex] or PDU-ID [hex] — e.g. "0x123"
    pub identifier: Option<String>,
    /// Signals within this message
    pub signals: Vec<Signal>,
    /// ECU sender/receiver assignments for this message
    pub ecu_assignments: Vec<EcuAssignment>,
}

/// A signal within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Signal name (column: "Signal" or "Signale")
    pub name: String,
    /// Signalkommentar — free-text description
    pub comment: Option<String>,
    /// Beschreibung — value description text
    pub description: Option<String>,
    /// InitWert roh [dez]
    pub init_value: Option<String>,
    /// FehlerWert roh [dez]
    pub error_value: Option<String>,
    /// Min Rohwert [dez]
    pub min_raw: Option<String>,
    /// Max Rohwert [dez]
    pub max_raw: Option<String>,
    /// phy Werte [dez]
    pub physical_value: Option<String>,
    /// Einheit (unit, e.g. "km/h", "°C")
    pub unit: Option<String>,
    /// Offset
    pub offset: Option<String>,
    /// Skalierung (scaling factor)
    pub scaling: Option<String>,
    /// Rohwert [dez] — raw value
    pub raw_value: Option<String>,
    /// StartBit
    pub start_bit: Option<u32>,
    /// Signal Länge [Bits]
    pub bit_length: Option<u32>,
}

/// ECU assignment for a message — indicates which ECUs send/receive/route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcuAssignment {
    /// ECU name (e.g. "HCP1", "Gateway", "BCM2")
    pub ecu_name: String,
    /// Role: Sender ("S"), Receiver ("E"), or Router ("S*")
    pub role: EcuRole,
}
