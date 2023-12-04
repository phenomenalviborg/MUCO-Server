use crate::{color::Color, connection_status::ConnectionStatus};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentHeadsetData {
    pub name: String,
    pub color: Color,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadsetData {
    pub persistent: PersistentHeadsetData,
    pub temp: TempHeadsetData,
}

