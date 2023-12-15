use msgs::server_client_msg::ServerClientMsg;

use crate::{headset_data::HeadsetData, context::MucoContextRef, inter_client_msg::InterClientMsg, player_data_msg::PlayerDataMsg, player_data::PlayerAttribute, connection_status::ConnectionStatus};

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
        ServerClientMsg::InterClient(sender, input_buffer) => {
            let result = InterClientMsg::decode(&input_buffer, sender);
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
                            match player_data {
                                PlayerAttribute::DeviceId(device_id) => {
                                    let device_id_string = device_id.to_string();
                                    let mut context = context_ref.write().await;
                                    if !context.status.headsets.contains_key(&device_id_string) {
                                        let new_player_data = HeadsetData::new(device_id);
                                        context.status.headsets.insert(device_id_string.clone(), new_player_data);
                                    }
                                    let headset = context.status.headsets.get_mut(&device_id_string).unwrap();
                                    headset.temp.connection_status = ConnectionStatus::Connected (sender);
                                    let headset_color = headset.persistent.color;
                                    let headset_language = headset.persistent.language;
                                    let headset_environment_code = headset.persistent.environment_code.clone();
                                    context.connection_id_to_player.insert(sender, device_id_string);
                                    context.status_generation += 1;
                                    context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Color (headset_color)))).await;
                                    context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::Language (headset_language)))).await;
                                    context.send_msg_to_player(sender, InterClientMsg::PlayerData(PlayerDataMsg::Set(PlayerAttribute::EnvironmentCode (headset_environment_code)))).await;
                                }
                                _ => {}
                            }
                        }
                        msg => println!("unhandeled player data msg: {msg:?}")
                    }
                }
                InterClientMsg::_Ping => {}
            }

            context_ref.write().await.get_or_request_unique_device_id(sender);
        }
    }
}
