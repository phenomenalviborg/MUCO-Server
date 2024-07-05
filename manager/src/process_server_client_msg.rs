use msgs::{inter_client_msg::InterClientMsg, player_data::{PlayerAttribute, PlayerAttributeTag}, player_data_msg::PlayerDataMsg, server_client_msg::ServerClientMsg};

use crate::{connection_status::ConnectionStatus, context::{get_or_request_device_id, MucoContextRef}, headset_data::HeadsetData};

pub async fn process_player_attribute(player_attribute: PlayerAttribute, sender: u32, context_ref: &MucoContextRef) {
    match player_attribute {
        PlayerAttribute::DeviceId(device_id) => {
            {
                let read = context_ref.read().await;
                if let Some(current_device_id) = read.connection_id_to_player.get(&sender) {
                    if *current_device_id == device_id {
                        return;
                    }
                }
            }

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
        _ => {
            if let Some(device_id) = get_or_request_device_id(sender, context_ref).await {
                let update = {
                    let read = context_ref.read().await;
                    let headset = read.status.headsets.get(&device_id).unwrap();
                    match &player_attribute {
                        PlayerAttribute::DevMode(in_dev_mode) => headset.temp.in_dev_mode != *in_dev_mode,
                        PlayerAttribute::Battery(status, level) => headset.temp.battery_status != *status || headset.temp.battery_level != *level,
                        PlayerAttribute::Level(level) => headset.temp.level != *level,
                        PlayerAttribute::AudioVolume(audio_volume) => headset.temp.audio_volume != *audio_volume,
                        _ => false
                    }
                };
                if update {
                    let mut write = context_ref.write().await;
                    let headset = write.status.headsets.get_mut(&device_id).unwrap();
                    match player_attribute {
                        PlayerAttribute::DevMode(in_dev_mode) => headset.temp.in_dev_mode = in_dev_mode,
                        PlayerAttribute::Battery(status, level) => {
                            headset.temp.battery_status = status;
                            headset.temp.battery_level = level;
                        }
                        PlayerAttribute::Level(level) => {
                            headset.temp.level = level;
                        }
                        PlayerAttribute::AudioVolume(audio_volume) => {
                            headset.temp.audio_volume = audio_volume;
                        }
                        _ => {}
                    }
                    write.status_generation += 1;
                }
            }
        }
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
                    process_data_buffer(data, sender, context_ref).await;
                }
                InterClientMsg::Diff (diff) => {
                    let Some(devide_id) = get_or_request_device_id(sender, context_ref).await else { return };
                    let data = {
                        let mut write = context_ref.write().await;
                        write.status.headsets.get_mut(&devide_id).unwrap().temp.data_buffer.take()
                    };
                    if let Some(mut data) = data {
                        let mut rdr = &diff[..];
                        apply_diff(&mut data, &mut rdr).unwrap();
                        process_data_buffer(data, sender, context_ref).await;
                    }
                }
            }

            context_ref.write().await.get_or_request_unique_device_id(sender);
        }
    }
}


pub async fn process_data_buffer(data: Vec<u8>, sender: u32, context_ref: &MucoContextRef) {
    let mut rdr = &data[..];
    for tag in PlayerAttributeTag::ALL_TAGS {
        let decode_result = PlayerAttribute::decode_(&mut rdr, *tag);
        match decode_result {
            Ok(player_attribute) => {
                process_player_attribute(player_attribute, sender, context_ref).await;
            }
            Err(err) => {
                dbg!(err);
                // todo!()
            }
        }
    }
    let mut write = context_ref.write().await;
    let device_id = *write.connection_id_to_player.get(&sender).unwrap();
    write.status.headsets.get_mut(&device_id).unwrap().temp.data_buffer = Some(data);
}

fn apply_diff(a: &mut Vec<u8>, diff: &[u8]) -> Option<()> {
    let mut diff_cursor = 0;
    let mut  buffer_cursor = 0;
    let len = decode_vlq(&mut diff_cursor, diff)?;
    
    while a.len() < len {
        a.push(0);
    }
    
    while buffer_cursor < len {
        let same = decode_vlq(&mut diff_cursor, diff)?;
        buffer_cursor += same;
        
        if buffer_cursor == len {
            break;
        }
        
        let different = decode_vlq(&mut diff_cursor, diff)?;
        for _ in 0..different {
            a[buffer_cursor] = diff[diff_cursor];
            buffer_cursor += 1;
            diff_cursor += 1;
        }
    }

    Some(())
}

pub fn decode_vlq(cursor: &mut usize, buffer: &[u8]) -> Option<usize> {
    let mut acc = 0;
    let mut shift = 0;
    loop {
        let b = *buffer.get(*cursor)?;
        *cursor += 1;
        acc += ((b & 0b1111111) as usize) << shift;
        if b & 0b10000000 == 0 { break }
        shift += 7;
    }

    Some(acc)
}
