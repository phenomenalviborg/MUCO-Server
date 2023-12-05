use std::time::{UNIX_EPOCH, SystemTime};

use crate::{Client, context::MucoContextRef, color::Color, connection_status::ConnectionStatus, DEFAULT_SESSION_DURATION, headset_data::SessionState, inter_client_msg::InterClientMsg, player_data_msg::PlayerDataMsg, player_data::{PlayerAttribute, Language}, SAVE_DATA_PATH};
use anyhow::Context;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn client_connection(ws: WebSocket, context_ref: MucoContextRef) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    let client = Client {
        sender: Some(client_sender),
    };

    let id = Uuid::new_v4().as_simple().to_string();

    context_ref.write().await.clients.insert(id.clone(), client);

    println!("{} connected", id);

    context_ref.read().await.update_clients().await;

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        match client_msg(&id, msg, &context_ref).await {
            Ok(_) => {}
            Err(e) => println!("error: {e}"),
        }
    }

    context_ref.write().await.clients.remove(&id);
    context_ref.read().await.update_clients().await;
    println!("{} disconnected", id);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClientMsg {
    Ping,
    Echo(String),
    Forget(String),
    Kick(String),
    SetColor(String, Color),
    SetName(String, String),
    SetLanguage(String, Language),
    StartSession(String),
    ExtendSession(String, i64),
    Pause(String),
    Unpause(String),
}

pub enum ServerResponse {
    Reply(String),
    UpdateClients,
    Nothing,
}

pub async fn process_client_msg(client_msg: ClientMsg, context_ref: &MucoContextRef) -> anyhow::Result<ServerResponse> {
    use ClientMsg::*;
    use ServerResponse::*;
    Ok(match client_msg {
        Ping => Reply("pong".to_string()),
        Echo(echo_string) => Reply(echo_string),
        Forget(unique_device_id) => {
            let mut context = context_ref.write().await;
            context.status.headsets.remove(&unique_device_id);
            context.status.save(SAVE_DATA_PATH)?;
            UpdateClients
        }
        Kick(unique_device_id) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.temp.connection_status = ConnectionStatus::Disconnected;
            UpdateClients
        }
        SetColor(unique_device_id, color) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.color = color;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Color(color)));
                context.send_msg_to_player(session_id, msg).await;
                context.status.save(SAVE_DATA_PATH)?;
            }
            UpdateClients
        }
        SetLanguage(unique_device_id, language) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.language = language;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Language(language)));
                context.send_msg_to_player(session_id, msg).await;
                context.status.save(SAVE_DATA_PATH)?;
            }
            UpdateClients
        }
        SetName(unique_device_id, name) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.name = name;
            context.status.save(SAVE_DATA_PATH)?;
            UpdateClients
        }
        StartSession(unique_device_id) => {
            let duration_since_unix_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?;
            let session_start_time = duration_since_unix_epoch.as_secs() as i64;
            let mut context = context_ref.write().await;
            let headset = context.get_headset_mut(unique_device_id)?;
            headset.temp.session_duration = DEFAULT_SESSION_DURATION;
            headset.temp.session_state = SessionState::Running(session_start_time);
            UpdateClients
        } 
        ExtendSession(unique_device_id, added_seconds) => {
            let mut context = context_ref.write().await;
            let headset = context.get_headset_mut(unique_device_id)?;
            headset.temp.session_duration += added_seconds;
            UpdateClients
        }
        Pause(unique_device_id) => {
            let mut context = context_ref.write().await;
            let headset = context.get_headset_mut(unique_device_id)?;
            match headset.temp.session_state {
                SessionState::Running(start_time) => {
                    let duration_since_unix_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?;
                    let now = duration_since_unix_epoch.as_secs() as i64;
                    let elapsed_time = now - start_time;
                    headset.temp.session_state = SessionState::Paused(elapsed_time);
                    UpdateClients
                }
                SessionState::Paused(_) => Nothing
            }
        }
        Unpause(unique_device_id) => {
            let mut context = context_ref.write().await;
            let headset = context.get_headset_mut(unique_device_id)?;
            match headset.temp.session_state {
                SessionState::Running(_) => Nothing,
                SessionState::Paused(elapsed_time) => {
                    let duration_since_unix_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?;
                    let now = duration_since_unix_epoch.as_secs() as i64;
                    let start_time = now - elapsed_time;
                    headset.temp.session_state = SessionState::Running(start_time);
                    UpdateClients
                }
            }
        }
    })
}

async fn client_msg(id: &str, msg: Message, context_ref: &MucoContextRef) -> anyhow::Result<()> {
    println!("received message from {}: {:?}", id, msg);
    let message = msg.to_str().ok().context("could not get message")?.trim();

    let client_msg = serde_json::from_str::<ClientMsg>(message)?;

    let response = process_client_msg(client_msg, context_ref).await?;

    match response {
        ServerResponse::Reply(reply) => {
            let context = context_ref.read().await;
            let client = context.clients.get(id).context("could not find client with id: {id}")?;
            let sender = client.sender.as_ref().context("client has no sender")?;
            let _ = sender.send(Ok(Message::text(reply)));
        }
        ServerResponse::UpdateClients => {
            context_ref.read().await.update_clients().await;
        }
        ServerResponse::Nothing => {}
    }

    Ok(())
}
