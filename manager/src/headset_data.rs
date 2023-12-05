use crate::{color::Color, connection_status::ConnectionStatus, DEFAULT_SESSION_DURATION};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentHeadsetData {
    pub name: String,
    pub color: Color,
}

impl PersistentHeadsetData {
    pub fn new() -> PersistentHeadsetData {
        PersistentHeadsetData { name: "New Headset".to_string(), color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } }
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
    pub fn new() -> HeadsetData {
        HeadsetData {
            persistent: PersistentHeadsetData::new(),
            temp: TempHeadsetData::new(),
        }
    }
}
