use std::path::Path;

use super::enums::{BusType, Platform};

/// Extract platform and bus metadata from a file path.
pub fn extract_metadata(path: &Path) -> (Option<Platform>, BusType, String) {
    let path_str = path.to_string_lossy();
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let platform = detect_platform(&path_str);
    let bus_type = detect_bus_type(&filename);
    let bus_name = extract_bus_name(&filename, &bus_type);

    (platform, bus_type, bus_name)
}

fn detect_platform(path: &str) -> Option<Platform> {
    let lower = path.to_lowercase();
    if lower.contains("e3_1_2") || lower.contains("e³ 1.2") || lower.contains("e3 1.2") {
        Some(Platform::E3_1_2)
    } else if lower.contains("mlbevo") {
        Some(Platform::MLBevo2)
    } else {
        None
    }
}

fn detect_bus_type(filename: &str) -> BusType {
    let lower = filename.to_lowercase();
    if lower.contains("canfd") {
        BusType::CANFD
    } else if lower.contains("vlan") || lower.contains("ethernet") {
        BusType::Ethernet
    } else if lower.contains("flexray") {
        BusType::FlexRay
    } else if lower.contains("lin") {
        BusType::LIN
    } else if lower.contains("most") {
        BusType::MOST
    } else if lower.contains("can") {
        // "CAN" after CANFD check — classical CAN
        BusType::CAN
    } else {
        BusType::Unknown(String::new())
    }
}

/// Extract bus name from filename.
/// E.g. "E3_1_2_Premium_HCP1_CANFD01_KMatrix_Module_V12.xlsx" → "HCP1_CANFD01"
/// E.g. "MLBevo_Gen2_MLBevo_KCAN_KMatrix_V8.xlsx" → "MLBevo_KCAN"
fn extract_bus_name(filename: &str, _bus_type: &BusType) -> String {
    let name = filename
        .strip_suffix(".xlsx")
        .or_else(|| filename.strip_suffix(".dbc"))
        .or_else(|| filename.strip_suffix(".ldf"))
        .unwrap_or(filename);

    // Find "KMatrix" in the filename and take the two segments before it
    let parts: Vec<&str> = name.split('_').collect();
    if let Some(km_idx) = parts.iter().position(|p| p.to_lowercase() == "kmatrix") {
        if km_idx >= 2 {
            return format!("{}_{}", parts[km_idx - 2], parts[km_idx - 1]);
        } else if km_idx >= 1 {
            return parts[km_idx - 1].to_string();
        }
    }

    // For DBC files: "MLBevo_Gen2_Konzern_KCAN.dbc" → use filename stem
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e3_canfd() {
        let path = Path::new("/data/api_nip_v12_e3_1_2/K-Matrix/E3_1_2_Premium_HCP1_CANFD01_KMatrix_Module_V12.08.05.00F.xlsx");
        let (platform, bus_type, bus_name) = extract_metadata(path);
        assert_eq!(platform, Some(Platform::E3_1_2));
        assert_eq!(bus_type, BusType::CANFD);
        assert_eq!(bus_name, "HCP1_CANFD01");
    }

    #[test]
    fn test_mlbevo_can() {
        let path = Path::new("/data/MLBevo 2/K-Matrix/CAN/MLBevo_Gen2_MLBevo_KCAN_KMatrix_V8.29.01F.xlsx");
        let (platform, bus_type, bus_name) = extract_metadata(path);
        assert_eq!(platform, Some(Platform::MLBevo2));
        assert_eq!(bus_type, BusType::CAN);
        assert_eq!(bus_name, "MLBevo_KCAN");
    }

    #[test]
    fn test_mlbevo_vlan() {
        let path = Path::new("/data/MLBevo 2/K-Matrix/Ethernet/MLBevo_Gen2_MLBevo_VLAN_Infotainment_D5_KMatrix_V8.26.03F.xlsx");
        let (platform, bus_type, bus_name) = extract_metadata(path);
        assert_eq!(platform, Some(Platform::MLBevo2));
        assert_eq!(bus_type, BusType::Ethernet);
        assert_eq!(bus_name, "Infotainment_D5");
    }
}
