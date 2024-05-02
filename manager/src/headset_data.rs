use crate::{color::Color, connection_status::ConnectionStatus, DEFAULT_SESSION_DURATION, player_data::Language};

pub const DEFAULT_ENVIRONMENT_CODE: &str = "AntilatencyAltEnvironmentHorizontalGrid~AgASIrhTiT_cRqA-r45jvZqZmT4AAAAAAAAAAACamRk_AVgBDwMDCgMNAgIIGQEOEgIICAAFEAMMGgEMEAIOAAAEBgANFQEPEQMRBwMDAQIHHwIEEwMKGwEBAwECGQEIBQIEIAALBQALFQEJFgMFFgIBBwMHEgMIHQEQGwILBgIDHwINCwAHFAIDHAAGDgECCQMQAwEDDQACDwMCBQMBHgMKCwAOHwALGAIBGAMPFQMIDQMQHgIGAQEQDAIKAAIOGAANCQALIAMHIAAKAwMODgMOIAIQFwMLDQANAwAKEwAEAgILCgICFQEFBAINGgMPBQICEgIKHgEHFwEHCgIBFQMNHQARDgEPBwIFDAAKEQABGwMHBAAREwEFHQIJDwIGGgEEGQMBCgEGCAI";
pub const DEFAULT_ENVIRONMENT_NAME: &str = "DefaultEnvironment";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentHeadsetData {
    pub unique_device_id: u32,
    pub name: String,
    pub color: Color,
    pub language: Language,
    pub environment_name: String,
}

impl PersistentHeadsetData {
    pub fn new(unique_device_id: u32) -> PersistentHeadsetData {
        PersistentHeadsetData {
            unique_device_id,
            name: "New Headset".to_string(),
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
            language: Language::EnGB,
            environment_name: DEFAULT_ENVIRONMENT_NAME.to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SessionState {
    Running (i64), //start time in seconds since UNIX-EPOCH
    Paused (i64), // time elapsed in seconds
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TempHeadsetData {
    pub connection_status: ConnectionStatus,
    pub session_state: SessionState, 
    pub session_duration: i64, //in seconds
}

impl TempHeadsetData {
    pub fn new() -> TempHeadsetData {
        TempHeadsetData {
            connection_status: ConnectionStatus::Disconnected,
            session_state: SessionState::Paused(0),
            session_duration: DEFAULT_SESSION_DURATION,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadsetData {
    pub persistent: PersistentHeadsetData,
    pub temp: TempHeadsetData,
}

impl HeadsetData {
    pub fn new(unique_device_id: u32) -> HeadsetData {
        HeadsetData {
            persistent: PersistentHeadsetData::new(unique_device_id),
            temp: TempHeadsetData::new(),
        }
    }
}
