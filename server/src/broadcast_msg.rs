use msgs::client_server_msg::Address;

#[derive(Debug, Clone)]
pub enum BroadcastMsg {
    Send (Address, Vec<u8>),
    Kick (u16),
}
