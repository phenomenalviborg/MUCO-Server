use crate::msgs::ServerClientMsg;

#[derive(Debug, Clone, Copy)]
pub enum ClientType {
    Player,
    Manager,
}

impl ClientType {
    pub fn from_u32(index: u32) -> Option<ClientType> {
        match index {
            0 => Some(ClientType::Player),
            1 => Some(ClientType::Manager),
            _ => None,
        }
    }
}

pub struct Client {
    pub user_id: usize,
    pub client_type: Option<ClientType>,
    pub main_to_client: tokio::sync::mpsc::Sender<ServerClientMsg>,
}
