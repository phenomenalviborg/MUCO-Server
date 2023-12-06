use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use msgs::client_server_msg::{ClientServerMsg, Address};
use tokio::sync::{RwLock, mpsc};
use warp::filters::ws::Message;

use crate::{status::Status, headset_data::HeadsetData, connection_status::ConnectionStatus, inter_client_msg::InterClientMsg, player_data_msg::PlayerDataMsg, player_data::PlayerAttributeTag};

pub struct MucoContext {
    pub to_relay_server_process: tokio::sync::mpsc::Sender<ClientServerMsg>,
    pub to_frontend_senders: HashMap<String, mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
    pub connection_id_to_player: HashMap<u32, String>,
    pub status: Status,
    pub status_generation: usize,
    pub unknown_connections: Vec<u32>,
}

pub type MucoContextRef = Arc<RwLock<MucoContext>>;

impl MucoContext {
    pub fn get_headset_mut(&mut self, unique_device_id: String) -> anyhow::Result<&mut HeadsetData> {
        let headset = self.status.headsets.get_mut(&unique_device_id).context("could not find headset with unique device id {unique_device_id}")?;
        Ok(headset)
    }

    pub async fn update_clients(&self) {
        let json = serde_json::to_string(&self.status).unwrap();

        for (_id, to_frontend_sender) in self.to_frontend_senders.iter() {
            to_frontend_sender.send(Ok(Message::text(json.clone()))).unwrap();
        }
    }

    pub async fn disconnect(&mut self, connection_id: u32) {
        let Some(device_id) = self.connection_id_to_player.get(&connection_id) else { return };
        let Some(headset) = self.status.headsets.get_mut(device_id) else { return };
        headset.temp.connection_status = ConnectionStatus::Disconnected;
        println!("client disconnected: {device_id}");
        self.status_generation += 1;
    }

    pub async fn send_msg_to_player(&mut self, connection_id: u32, msg: InterClientMsg) {
        let mut bytes = Vec::new();
        msg.pack(&mut bytes);
        self.to_relay_server_process.send(ClientServerMsg::BinaryMessageTo (Address::Client(connection_id), bytes)).await.unwrap();
    }
    
    pub fn get_or_request_unique_device_id(&mut self, connection_id: u32) -> Option<&str> {
        if let Some(unique_device_id) = self.connection_id_to_player.get(&connection_id) {
            return Some(unique_device_id)
        }
        if !self.unknown_connections.contains(&connection_id) {
            self.unknown_connections.push(connection_id)
        }
        None
    }

    pub async fn request_unknown_device_ids(&mut self) {
        while let Some(connection_id) = self.unknown_connections.pop() {
            let msg = InterClientMsg::PlayerData(PlayerDataMsg::Request (PlayerAttributeTag::DeviceId));
            self.send_msg_to_player(connection_id, msg).await;
        }
    }
}
