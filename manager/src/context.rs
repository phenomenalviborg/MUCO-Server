use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use msgs::client_server_msg::{ClientServerMsg, Address};
use tokio::sync::{RwLock, mpsc};
use warp::filters::ws::Message;

use crate::{connection_status::ConnectionStatus, headset_data::{HeadsetData, DEFAULT_ENVIRONMENT_CODE}, inter_client_msg::InterClientMsg, player_data::PlayerAttributeTag, player_data_msg::PlayerDataMsg, status::{DeviceId, Status}};

pub struct MucoContext {
    pub to_relay_server_process: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub to_frontend_senders: HashMap<String, mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
    pub connection_id_to_player: HashMap<u32, DeviceId>,
    pub status: Status,
    pub status_generation: usize,
    pub unknown_connections: Vec<u32>,
}

pub type MucoContextRef = Arc<RwLock<MucoContext>>;

impl MucoContext {
    pub fn get_headset_mut(&mut self, unique_device_id: DeviceId) -> anyhow::Result<&mut HeadsetData> {
        let headset = self.status.headsets.get_mut(&unique_device_id).context("could not find headset with unique device id {unique_device_id}")?;
        Ok(headset)
    }

    pub fn get_environment_code_string(&self, name: &str) -> Box<str> {
        match self.status.environment_codes.get(name) {
            Some(code) => code.to_owned(),
            None => {
                println!("could not find environment code {name}, returning default");
                DEFAULT_ENVIRONMENT_CODE.into()
            },
        }
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

    pub async fn send_msg_to_player(&mut self, connection_id: u32, inter_client_msg: InterClientMsg) {
        let mut inter_client_msg_bytes = Vec::new();
        inter_client_msg.pack(&mut inter_client_msg_bytes);
        let client_server_msg = ClientServerMsg::BinaryMessageTo (Address::Client(connection_id), &inter_client_msg_bytes);
        let mut client_server_msg_bytes = Vec::new();
        client_server_msg.pack(&mut client_server_msg_bytes);
        self.to_relay_server_process.send(client_server_msg_bytes).await.unwrap();
    }
    
    pub fn get_or_request_unique_device_id(&mut self, connection_id: u32) -> Option<u32> {
        if let Some(unique_device_id) = self.connection_id_to_player.get(&connection_id) {
            return Some(*unique_device_id)
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
