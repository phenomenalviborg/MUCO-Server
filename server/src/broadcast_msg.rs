use msgs::client_server_msg::ClientServerMsg;

#[derive(Debug, Clone)]
pub enum BroadcastMsg {
    ClientServerMsg (u32, ClientServerMsg),
    ClientDisconnected (u32),
}
