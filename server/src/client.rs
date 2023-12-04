use msgs::{client_type::ClientType, server_client_msg::ServerClientMsg};

pub struct Client {
    pub session_id: u32,
    pub client_type: Option<ClientType>,
    pub main_to_client: tokio::sync::mpsc::Sender<ServerClientMsg>,
}
