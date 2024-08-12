use msgs::{color::Color, player_data::{BatteryStatus, Language}};

use crate::{connection_status::ConnectionStatus, status::EnvCodeName, DEFAULT_SESSION_DURATION};

pub const DEFAULT_ENVIRONMENT_CODE: &str = "AntilatencyAltEnvironmentHorizontalGrid~AgACBLhTiT_cRqA-r45jvZqZmT4AAAAAAAAAAACamRk_AQEAAgM";
pub const DEFAULT_ENVIRONMENT_NAME: &str = "NoEnvironment";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentHeadsetData {
    pub unique_device_id: u32,
    pub name: String,
    pub color: Color,
    pub language: Language,
    pub environment_name: EnvCodeName,
}

impl PersistentHeadsetData {
    pub fn new(unique_device_id: u32) -> PersistentHeadsetData {
        PersistentHeadsetData {
            unique_device_id,
            name: "New Headset".to_string(),
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
            language: Language::EnGB,
            environment_name: DEFAULT_ENVIRONMENT_NAME.into(),
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
    pub in_dev_mode: bool,
    pub is_visible: bool,
    pub battery_status: BatteryStatus,
    pub battery_level: f32,
    pub data_buffer: Option<Vec<u8>>,
    pub level: f32,
    pub audio_volume: f32,
}

impl TempHeadsetData {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
            session_state: SessionState::Paused(0),
            session_duration: DEFAULT_SESSION_DURATION,
            in_dev_mode: false,
            is_visible: true,
            data_buffer: None,
            battery_status: BatteryStatus::Unknown,
            battery_level: 0.0,
            level: 0.0,
            audio_volume:0.5,
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
