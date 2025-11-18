use std::time::{SystemTime, UNIX_EPOCH};

use crate::{connection_status::ConnectionStatus, context::MucoContextRef, discovery::DiscoveryEventType, headset_data::SessionState, status::{DeviceId, EnvCodeName}, DEFAULT_SESSION_DURATION};
use anyhow::Context;
use futures::{FutureExt, StreamExt};
use msgs::{client_server_msg::ClientServerMsg, color::Color, inter_client_msg::InterClientMsg, manager_client_msg::{DiscoveredServerInfo, ManagerClientMsg}, player_data::{EnvData, Language, PlayerAttribute}, player_data_msg::PlayerDataMsg};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn frontend_connection_process(ws: WebSocket, context_ref: MucoContextRef) {
    let (frontend_ws_sender, mut frontend_ws_rcv) = ws.split();
    let (to_frontend_connection_process, front_end_connection_process_rcv) = mpsc::unbounded_channel();

    let front_end_connection_rcv_unbounded_receiver_stream = UnboundedReceiverStream::new(front_end_connection_process_rcv);
    tokio::task::spawn(front_end_connection_rcv_unbounded_receiver_stream.forward(frontend_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    let id = Uuid::new_v4().as_simple().to_string();

    let to_frontend_sender_clone = to_frontend_connection_process.clone();
    context_ref.write().await.to_frontend_senders.insert(id.clone(), to_frontend_connection_process);

    println!("{} connected", id);

    // Send initial discovered servers list
    let discovery_service = {
        let context = context_ref.read().await;
        context.discovery_service.clone()
    };

    let discovered_servers = discovery_service.get_all_servers();
    let servers_info: Vec<DiscoveredServerInfo> = discovered_servers
        .into_iter()
        .map(|s| DiscoveredServerInfo {
            host: s.host,
            name: s.name,
        })
        .collect();

    let initial_msg = ManagerClientMsg::DiscoveredServers {
        servers: servers_info,
    };

    if let Ok(json) = serde_json::to_string(&initial_msg) {
        let _ = to_frontend_sender_clone.send(Ok(Message::text(json)));
    }

    // Spawn task to forward discovery events
    let to_frontend_sender_clone2 = to_frontend_sender_clone.clone();
    let mut discovery_rx = discovery_service.subscribe();
    tokio::spawn(async move {
        while let Ok((event_type, server)) = discovery_rx.recv().await {
            let msg = match event_type {
                DiscoveryEventType::ServerDiscovered => {
                    ManagerClientMsg::ServerDiscovered {
                        host: server.host,
                        name: server.name,
                    }
                }
                DiscoveryEventType::ServerLost => {
                    ManagerClientMsg::ServerLost { host: server.host }
                }
            };

            if let Ok(json) = serde_json::to_string(&msg) {
                if to_frontend_sender_clone2.send(Ok(Message::text(json))).is_err() {
                    break; // Frontend disconnected
                }
            }
        }
    });

    context_ref.write().await.status_generation += 1;

    while let Some(result) = frontend_ws_rcv.next().await {
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

    context_ref.write().await.to_frontend_senders.remove(&id);
    println!("{} disconnected", id);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClientMsg {
    Ping,
    Echo(String),
    Forget(DeviceId),
    Kick(DeviceId),
    SetColor(DeviceId, Color),
    SetLevel(DeviceId, f32),
    SetAudioVolume(DeviceId, f32),
    SetName(DeviceId, String),
    SetLanguage(DeviceId, Language),
    StartSession(DeviceId),
    ExtendSession(DeviceId, i64),
    Pause(DeviceId),
    Unpause(DeviceId),
    SetEnvironment(DeviceId, EnvCodeName),
    SetEnvironmentData(EnvCodeName, EnvData),
    RemoveEnvironment(EnvCodeName),
    RenameEnvironment(EnvCodeName, EnvCodeName),
    SetDevMode(DeviceId, bool),
    SetIsVisible(DeviceId, bool),
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
            if let Some(headset_data) = context.status.headsets.get(&unique_device_id) {
                if let ConnectionStatus::Connected(connection_id) = headset_data.temp.connection_status {
                    context.connection_id_to_player.remove(&connection_id);
                }
            }
            
            context.status.headsets.remove(&unique_device_id);
            UpdateClients
        }
        Kick(unique_device_id) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = ClientServerMsg::Kick(session_id);
                let mut bytes = Vec::new();
                msg.pack(&mut bytes);
                context.to_relay_server_process.send(bytes).await?;
            }
            
            Nothing
        }
        SetColor(unique_device_id, color) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.color = color;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Color(color)));
                context.send_msg_to_player(session_id, msg).await;
            }
            UpdateClients
        }
        SetLevel(unique_device_id, level) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.temp.level = level;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Level(level)));
                context.send_msg_to_player(session_id, msg).await;
            }
            UpdateClients
        }
        SetAudioVolume(unique_device_id,audio_volume) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.temp.audio_volume = audio_volume;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::AudioVolume(audio_volume)));
                context.send_msg_to_player(session_id, msg).await;
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
            }
            UpdateClients
        }
        SetName(unique_device_id, name) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.name = name;
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
        SetEnvironment(unique_device_id, name) => {
            let mut context = context_ref.write().await;
            let env_data = context.status.environment_data.get(&name).context("could not find environment")?.to_owned();
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.persistent.environment_name = name.clone();
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::EnvironmentData(name, env_data)));
                context.send_msg_to_player(session_id, msg).await;
            }
            UpdateClients
        }
        SetEnvironmentData(env_name, data) => {
            let mut context = context_ref.write().await;
            let environment_codes = &mut context.status.environment_data;
            environment_codes.insert(env_name.clone(), data.clone());
            let mut headsets_to_update = Vec::new();
            for (headset_name, headset) in &context.status.headsets {
                if headset.persistent.environment_name == env_name {
                    headsets_to_update.push(headset_name.clone());
                }
            }
            for headset_name in headsets_to_update {
                let headset = context.get_headset_mut(headset_name).unwrap();
                if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                    let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::EnvironmentData(env_name.clone(), data.clone())));
                    context.send_msg_to_player(session_id, msg).await;
                }
            }
            UpdateClients
        }
        RemoveEnvironment(name) => {
            let mut context = context_ref.write().await;
            let environment_codes = &mut context.status.environment_data;
            environment_codes.remove(&name);
            UpdateClients
        }
        RenameEnvironment(old_name, new_name) => {
            let mut context = context_ref.write().await;
            let environment_codes = &mut context.status.environment_data;
            let code = environment_codes.get(&old_name).unwrap();
            environment_codes.insert(new_name.clone(), code.clone());
            environment_codes.remove(&old_name);
            for (_, headset) in &mut context.status.headsets {
                if headset.persistent.environment_name == old_name {
                    headset.persistent.environment_name = new_name.clone();
                }
            }
            UpdateClients
        }
        SetDevMode(unique_device_id, in_dev_mode) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.temp.in_dev_mode = in_dev_mode;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::DevMode(in_dev_mode)));
                context.send_msg_to_player(session_id, msg).await;
            }
            UpdateClients
        }
        SetIsVisible(unique_device_id, is_visible) => {
            let mut context = context_ref.write().await;
            let headset = context.status.headsets.get_mut(&unique_device_id).context("could not find headset with id {unique_device_id}")?;
            headset.temp.is_visible = is_visible;
            if let ConnectionStatus::Connected(session_id) = headset.temp.connection_status {
                let msg = InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::IsVisible(is_visible)));
                context.send_msg_to_player(session_id, msg).await;
            }
            UpdateClients
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
            let sender = context.to_frontend_senders.get(id).context("could not find client with id: {id}")?;
            let _ = sender.send(Ok(Message::text(reply)));
        }
        ServerResponse::UpdateClients => {
            context_ref.write().await.status_generation += 1;
        }
        ServerResponse::Nothing => {}
    }

    Ok(())
}
