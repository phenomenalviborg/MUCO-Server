use crate::msgs::ServerClientMsg;

pub struct Client {
    pub user_id: usize,
    pub main_to_client: tokio::sync::mpsc::Sender<ServerClientMsg>,
}
