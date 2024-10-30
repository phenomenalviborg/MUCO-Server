use std::{fs::File, io::Write, net::SocketAddr, sync::Arc, time::SystemTime};

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use chrono::Local;
use msgs::{client_server_msg::{Address, ClientServerMsg}, client_type::ClientType, dequeue::dequeue_msg, model::SharedData, network_version::NETWORK_VERSION_NUMBER, server_client_msg::ServerClientMsg};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::{broadcast, RwLock}};

use crate::broadcast_msg::BroadcastMsg;

pub struct ClientDb {
    pub session_id_counter: u16,
}

impl ClientDb {
    pub fn new() -> ClientDb {
        ClientDb {
            session_id_counter: 0,
        }
    }

    pub async fn new_client(&mut self, socket: TcpStream, addr: SocketAddr, tx: broadcast::Sender<BroadcastMsg>, log_folder_path: Option<&str>, server_start_time: SystemTime, shared_data: Arc<RwLock<SharedData>>) {
        socket.set_nodelay(true).unwrap();
        let session_id = self.session_id_counter;
        self.session_id_counter += 1;

        let mut log_file = None;
        if let Some(path) = log_folder_path {
            let file_path = format!("{path}/{session_id}.muco_log");
            log_file = Some(File::create_new(file_path).unwrap());
        }
        spawn_client_process(socket, tx, session_id, server_start_time, log_file, shared_data);
        print_message_preamble_no_device_id(session_id);
        println!("accepted new connection from {addr}");
    }
}

pub fn print_message_preamble(session_id: u16, device_id: u32) {
    print_message_preamble_no_device_id(session_id);
    print!("{device_id} ");
}

pub fn print_timestamp() {
    let date = Local::now();
    print!("{} ", date.format("%Y-%m-%d %H:%M:%S"));
}

pub fn print_message_preamble_no_device_id(session_id: u16) {
    print_timestamp();
    print!("{session_id} ");
}


pub fn spawn_client_process(mut socket: TcpStream, tx: broadcast::Sender<BroadcastMsg>, session_id: u16, server_start_time: SystemTime, mut log_file: Option<File>, shared_data: Arc<RwLock<SharedData>>) {
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();

        let network_version_len = NETWORK_VERSION_NUMBER.len();
        let device_id_len = 4;
        let initial_message_len = network_version_len + device_id_len;
        while input_buffer.len() < initial_message_len {
            let result = socket.read(&mut static_buffer).await;
            let len = match result {
                Ok(len) => len,
                Err(e) => {
                    print_message_preamble_no_device_id(session_id);
                    println!("error while reading from socker: {e}");
                    return;
                }
            };
            if len == 0 {
                print_message_preamble_no_device_id(session_id);
                println!("client died");
                return;
            }
            input_buffer.extend(&static_buffer[..len]);
        }

        {
            let network_version_number = &input_buffer[..network_version_len];
            if network_version_number != NETWORK_VERSION_NUMBER {
                print_message_preamble_no_device_id(session_id);
                println!("rejecting client because of network version number, expected: {NETWORK_VERSION_NUMBER:?}, got: {network_version_number:?}");
                return;
            }
            input_buffer.drain(..network_version_len);
        }

        let device_id = LittleEndian::read_u32(&input_buffer);
        input_buffer.drain(..device_id_len);
        print_message_preamble(session_id, device_id);
        println!("received initial message");

        {
            let mut output_buffer = Vec::new();
            let model = shared_data.read().await.model.clone();
            let msg = ServerClientMsg::Hello {
                session_id,
                model,
            };
            msg.pack(&mut output_buffer);
            match socket.write_all(&output_buffer).await {
                Ok(_) => {
                    let flush_result = socket.flush().await;
                    match flush_result {
                        Ok(_) => {},
                        Err(err) => {
                            print_message_preamble(session_id, device_id);
                            println!("error while flushing data: {err}");
                            return;
                        }
                    }
                },
                Err(e) => {
                    print_message_preamble(session_id, device_id);
                    println!("disconnecting because of error while writing to client: {e}");
                    return;
                }
            }
        }
        
        let mut rx = tx.subscribe();
        let mut should_disconnect = false;
        while !should_disconnect {
            tokio::select! {
                biased;
                result = rx.recv() => {
                    let broadcast_msg = match result {
                        Ok(msg) => msg,
                        Err(e) => {
                            print_message_preamble(session_id, device_id);
                            println!("error while receiving: {e}");
                            print_message_preamble(session_id, device_id);
                            println!("client disconnected");
                            break;
                        }
                    };

                    process_broadcast_msg(broadcast_msg, session_id, device_id, &mut socket, &mut should_disconnect).await;
                }
                result = socket.read(&mut static_buffer) => {
                    let len = match result {
                        Ok(len) => len,
                        Err(e) => {
                            print_message_preamble(session_id, device_id);
                            println!("error while reading from socket: {e}");
                            break;
                        }
                    };
                    if len == 0 {
                        print_message_preamble(session_id, device_id);
                        println!("client died");
                        break;
                    }
                    input_buffer.extend(&static_buffer[..len]);

                    while let Some((begin, end)) = dequeue_msg(&input_buffer) {
                        if let Some(file) = &mut log_file {
                            let since_server_start = std::time::SystemTime::now()
                                .duration_since(server_start_time)
                                .expect("Time went backwards").as_millis() as u32;
                            file.write_u32::<LittleEndian>(since_server_start).unwrap();
                            file.write_all(&input_buffer[..end]).unwrap();
                        }

                        let decode_result = ClientServerMsg::decode(&input_buffer[begin..end], session_id);

                        let msg = match decode_result {
                            Ok(msg) => msg,
                            Err(e) => {
                                print_message_preamble(session_id, device_id);
                                println!("error while decode msg: {e}");
                                break;
                            }
                        };

                        if let Some(response) = process_msg(msg, session_id, &shared_data, &mut should_disconnect).await {
                            match tx.send(response) {
                                Ok(_) => {}
                                Err(e) => {
                                    print_message_preamble(session_id, device_id);
                                    println!("error while trying to broadcast msg: {e}");
                                }
                            }
                        }

                        input_buffer.drain(..end);
                    }
                }
            }
        }
        {
            let msg = ServerClientMsg::ClientDisconnected (session_id);
            let mut output_buffer: Vec<u8> = Vec::new();
            msg.pack(&mut output_buffer);
            match tx.send(BroadcastMsg::Send(Address::All, output_buffer)) {
                Ok(_) => {}
                Err(e) => {
                    print_message_preamble(session_id, device_id);
                    println!("error while trying to broadcast exit msg: {e}");
                }
            }
        }
    });
}

pub async fn process_broadcast_msg(broadcast_msg: BroadcastMsg, session_id: u16, device_id: u32, socket: &mut TcpStream, should_disconnect: &mut bool) {
    match broadcast_msg {
        BroadcastMsg::Send(address, output_buffer) => {
            if address.includes(session_id) {
                match socket.write_all(&output_buffer).await {
                    Ok(_) => {},
                    Err(e) => {
                        print_message_preamble(session_id, device_id);
                        println!("disconnecting because of error while writing to socket: {e}");
                        *should_disconnect = true;
                    }
                }
            }
        }
        BroadcastMsg::Kick(to_kick) => {
            if to_kick == session_id {
                *should_disconnect = true;
            }
        }
    }
}

pub async fn process_msg<'a>(msg: ClientServerMsg<'a>, session_id: u16, shared_data: &RwLock<SharedData>, should_disconnect: &mut bool) -> Option<BroadcastMsg> {
    match msg {
        ClientServerMsg::Disconnect => {
            *should_disconnect = true;
            None
        }
        ClientServerMsg::BinaryMessageTo (address, content) => {
            let msg = ServerClientMsg::InterClient(session_id, content);
            let mut output_buffer: Vec<u8> = Vec::new();
            msg.pack(&mut output_buffer);
            Some(BroadcastMsg::Send (address, output_buffer))
        }
        ClientServerMsg::SetClientType (client_type) => {
            if client_type != ClientType::Player {
                None
            }
            else {
                let address = Address::Other (session_id);
                let msg = ServerClientMsg::ClientConnected (session_id);
                let mut output_buffer: Vec<u8> = Vec::new();
                msg.pack(&mut output_buffer);
                Some(BroadcastMsg::Send (address, output_buffer))
            }
        }
        ClientServerMsg::Kick (to_kick) => Some(BroadcastMsg::Kick (to_kick)),
        ClientServerMsg::SetData { room, creator_id, index, data } => {
            let mut lock = shared_data.write().await;
            if let Some(data_owner) = lock.data_owners.get(&(room, creator_id, index)) {
                if *data_owner != session_id as u16 {
                    return None;
                }
            }
            lock.model.facts.insert((room, creator_id, index), data.into());
            let address = Address::Other (session_id);
            let msg = ServerClientMsg::DataNotify { room, creator_id, index, data };
            let mut output_buffer: Vec<u8> = Vec::new();
            msg.pack(&mut output_buffer);
            Some(BroadcastMsg::Send (address, output_buffer))
        }
        ClientServerMsg::ClaimData { room, creator_id, index } => {
            let mut lock = shared_data.write().await;
            lock.data_owners.insert((room, creator_id, index), session_id as u16);
            let address = Address::Other (session_id);
            let msg = ServerClientMsg::DataOwner { room, creator_id, index, owner_id: session_id as u16 };
            let mut output_buffer: Vec<u8> = Vec::new();
            msg.pack(&mut output_buffer);
            Some(BroadcastMsg::Send (address, output_buffer))
        }
    }
}
