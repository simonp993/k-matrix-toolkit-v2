use serde::{Deserialize, Serialize};
use std::fmt;

/// Vehicle platform identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    E3_1_2,
    MLBevo2,
    Unknown(String),
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::E3_1_2 => write!(f, "E³ 1.2"),
            Platform::MLBevo2 => write!(f, "MLBevo 2"),
            Platform::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Automotive bus type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusType {
    CAN,
    CANFD,
    LIN,
    Ethernet,
    FlexRay,
    MOST,
    Unknown(String),
}

impl fmt::Display for BusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusType::CAN => write!(f, "CAN"),
            BusType::CANFD => write!(f, "CAN FD"),
            BusType::LIN => write!(f, "LIN"),
            BusType::Ethernet => write!(f, "Ethernet"),
            BusType::FlexRay => write!(f, "FlexRay"),
            BusType::MOST => write!(f, "MOST"),
            BusType::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Source file format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileFormat {
    XLSX,
    DBC,
    LDF,
    JSON,
    XML,
}

/// Role of an ECU in a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EcuRole {
    /// "S" in K-Matrix — sends the signal
    Sender,
    /// "E" in K-Matrix — receives the signal
    Receiver,
    /// "S*" in K-Matrix — conditional sender / router
    Router,
}
