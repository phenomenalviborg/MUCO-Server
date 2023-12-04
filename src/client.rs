use crate::msgs::{ServerClientMsg, ClientType};

pub struct Client {
    pub user_id: usize,
    pub client_type: Option<ClientType>,
    pub main_to_client: tokio::sync::mpsc::Sender<ServerClientMsg>,
}
