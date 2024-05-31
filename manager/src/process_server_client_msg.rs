use msgs::{inter_client_msg::InterClientMsg, player_data::{PlayerAttribute, PlayerAttributeTag}, player_data_msg::PlayerDataMsg, server_client_msg::ServerClientMsg};

use crate::{connection_status::ConnectionStatus, context::MucoContextRef, headset_data::HeadsetData};

pub async fn process_player_attribute(player_data: PlayerAttribute, sender: u32, context_ref: &MucoContextRef) {
    match player_data {
        PlayerAttribute::DeviceId(device_id) => {
            let mut context = context_ref.write().await;
            if !context.status.headsets.contains_key(&device_id) {
                let new_player_data = HeadsetData::new(device_id);
                context.status.headsets.insert(device_id, new_player_data);
            }
            let headset = context.status.headsets.get_mut(&device_id).unwrap();
            headset.temp.connection_status = ConnectionStatus::Connected (sender);
            let color = headset.persistent.color;
            let language = headset.persistent.language;
            let environment_name = headset.persistent.environment_name.clone();
            let environment_code = context.get_environment_code_string(&environment_name);
            context.connection_id_to_player.insert(sender, device_id);
            context.status_generation += 1;
            context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Color (color)))).await;
            context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Language (language)))).await;
            context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::EnvironmentCode (environment_code.to_owned())))).await;
        }
        PlayerAttribute::DevMode(in_dev_mode) => {
            let mut context = context_ref.write().await;
            let device_id = match context.connection_id_to_player.get(&sender) {
                Some(id) => *id,
                None => {
                    println!("could not find device id for sender: {sender}");
                    return;
                }
            };
            let headset = context.status.headsets.get_mut(&device_id).unwrap();
            headset.temp.in_dev_mode = in_dev_mode;
            context.status_generation += 1;
        }
        _ => {}
    }
}

pub async fn process_server_client_msg(msg: ServerClientMsg<'_>, context_ref: &MucoContextRef) {
    match msg {
        ServerClientMsg::AssignSessionId(session_id) => {
            println!("session id: {session_id}");
        }
        ServerClientMsg::ClientConnected(session_id) => {
            println!("client connected: {session_id}");
        }
        ServerClientMsg::ClientDisconnected(session_id) => {
            let mut context = context_ref.write().await;
            context.disconnect(session_id).await;
        }
        ServerClientMsg::InterClient(sender, mut input_buffer) => {
            let result = InterClientMsg::decode(&mut input_buffer);
            let inter_client_msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    println!("error while decodeing msg: {e}");
                    return;
                }
            };

            match inter_client_msg {
                InterClientMsg::_Interaction => {}
                InterClientMsg::PlayerData (player_data_msg) => {
                    match player_data_msg {
                        PlayerDataMsg::Notify (player_data) => {
                            process_player_attribute(player_data, sender, context_ref).await;
                        }
                        msg => println!("unhandeled player data msg: {msg:?}")
                    }
                }
                InterClientMsg::_Ping => {}
                InterClientMsg::AllPlayerData (data) => {
                    let mut rdr = &data[..];
                    for tag in PlayerAttributeTag::ALL_TAGS {
                        let decode_result = PlayerAttribute::decode_(&mut rdr, *tag);
                        match decode_result {
                            Ok(att) => {
                                process_player_attribute(att, sender, context_ref).await;
                            }
                            Err(err) => {
                                dbg!(err);
                                todo!()
                            }
                        }
                    }
                }
                InterClientMsg::Diff (_diff) => {

                }
            }

            context_ref.write().await.get_or_request_unique_device_id(sender);
        }
    }
}
