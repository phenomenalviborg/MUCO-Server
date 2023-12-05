#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConnectionStatus {
    Connected (u32),
    Disconnected,
}

