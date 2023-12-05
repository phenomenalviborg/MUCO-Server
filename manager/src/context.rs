use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use tokio::sync::RwLock;
use warp::filters::ws::Message;

use crate::{Client, status::Status, headset_data::HeadsetData};

pub struct MucoContext {
    pub clients: HashMap<String, Client>,
    pub connection_id_to_player: HashMap<u32, String>,
    pub status: Status,
}

pub type MucoContextRef = Arc<RwLock<MucoContext>>;

impl MucoContext {
    pub fn get_headset_mut(&mut self, unique_device_id: String) -> anyhow::Result<&mut HeadsetData> {
        let headset = self.status.headsets.get_mut(&unique_device_id).context("could not find headset with unique device id {unique_device_id}")?;
        Ok(headset)
    }

    pub async fn update_clients(&self) {
        let json = serde_json::to_string(&self.status).unwrap();

        for (_id, client) in self.clients.iter() {
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(Message::text(json.clone())));
            }
        }
    }
}

