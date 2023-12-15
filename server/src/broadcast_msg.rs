use msgs::{client_server_msg::Address, server_client_msg::ServerClientMsg};

#[derive(Debug, Clone)]
pub enum BroadcastMsg {
    Send(Address, ServerClientMsg),
    Kick(u32),
}
