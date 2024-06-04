use std::{fs::File, io::Write, net::SocketAddr, time::SystemTime};

use byteorder::{LittleEndian, WriteBytesExt};
use msgs::{client_server_msg::{Address, ClientServerMsg}, client_type::ClientType, dequeue::dequeue_msg, network_version::NETWORK_VERSION_NUMBER, server_client_msg::ServerClientMsg};
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}, sync::broadcast};

use crate::broadcast_msg::BroadcastMsg;

pub struct ClientDb {
    pub session_id_counter: u32,
}

impl ClientDb {
    pub fn new() -> ClientDb {
        ClientDb {
            session_id_counter: 0,
        }
    }

    pub async fn new_client(&mut self, socket: TcpStream, addr: SocketAddr, tx: broadcast::Sender<BroadcastMsg>, log_folder_path: Option<&str>, server_start_time: SystemTime) {
        socket.set_nodelay(true).unwrap();
        let session_id = self.session_id_counter;
        self.session_id_counter += 1;

        let mut log_file = None;
        if let Some(path) = log_folder_path {
            let file_path = format!("{path}/{session_id}.muco_log");
            log_file = Some(File::create_new(file_path).unwrap());
        }
        spawn_client_process(socket, tx, session_id, addr, server_start_time, log_file);
        println!("accepted client: {session_id} {addr}");
    }
}

pub fn spawn_client_process(mut socket: TcpStream, tx: broadcast::Sender<BroadcastMsg>, session_id: u32, addr: SocketAddr, server_start_time: SystemTime, mut log_file: Option<File>) {
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();

        while input_buffer.len() < 4
        {
            let result = socket.read(&mut static_buffer).await;
            let len = match result {
                Ok(len) => len,
                Err(e) => {
                    println!("error while reading from socker: {e}");
                    return;
                }
            };
            if len == 0 {
                println!("client died: {session_id} {addr}");
                return;
            }
            input_buffer.extend(&static_buffer[..len]);
        }

        {
            let network_version_number = &input_buffer[..4];
            if network_version_number != NETWORK_VERSION_NUMBER {
                println!("rejecting client because of network version number, expected: {NETWORK_VERSION_NUMBER:?}, got: {network_version_number:?}");
                return;
            }
            input_buffer.drain(..4);
        }

        {
            let mut output_buffer = Vec::new();
            let msg = ServerClientMsg::AssignSessionId(session_id);
            msg.pack(&mut output_buffer);
            match socket.write_all(&output_buffer).await {
                Ok(_) => {},
                Err(e) => {
                    println!("disconnecting because of error while writing to client: {e}");
                    return;
                }
            }
        }
        
        let mut rx = tx.subscribe();
        'outer: loop {
            tokio::select! {
                biased;
                result = rx.recv() => {
                    let broadcast_msg = match result {
                        Ok(msg) => msg,
                        Err(e) => {
                            println!("error while receiving: {e}");
                            println!("client disconnected: {session_id} {addr}");
                            break;
                        }
                    };

                    match broadcast_msg {
                        BroadcastMsg::Send(address, output_buffer) => {
                            if address.includes(session_id) {
                                match socket.write_all(&output_buffer).await {
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("disconnecting because of error while writing to socket: {e}");
                                        break;
                                    }
                                }
                            }
                        }
                        BroadcastMsg::Kick(to_kick) => {
                            if to_kick == session_id {
                                break;
                            }
                        }
                    }
                }
                result = socket.read(&mut static_buffer) => {
                    let len = match result {
                        Ok(len) => len,
                        Err(e) => {
                            println!("error while reading from socket: {e}");
                            break;
                        }
                    };
                    if len == 0 {
                        println!("client died: {session_id} {addr}");
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
                                println!("error while decode msg: {e}");
                                break;
                            }
                        };

                        if let Some(response) = match msg {
                            ClientServerMsg::Disconnect => break 'outer,
                            ClientServerMsg::BinaryMessageTo(address, content) => {
                                let msg = ServerClientMsg::InterClient(session_id, content);
                                let mut output_buffer: Vec<u8> = Vec::new();
                                msg.pack(&mut output_buffer);
                                Some(BroadcastMsg::Send(address, output_buffer))
                            }
                            ClientServerMsg::SetClientType(client_type) => {
                                if client_type != ClientType::Player {
                                    None
                                }
                                else {
                                    let address = Address::Other(session_id);
                                    let msg = ServerClientMsg::ClientConnected(session_id);
                                    let mut output_buffer: Vec<u8> = Vec::new();
                                    msg.pack(&mut output_buffer);
                                    Some(BroadcastMsg::Send(address, output_buffer))
                                }
                            }
                            ClientServerMsg::Kick(to_kick) => Some(BroadcastMsg::Kick(to_kick)),
                        } {
                            match tx.send(response) {
                                Ok(_) => {}
                                Err(e) => println!("error while trying to broadcast msg: {e}"),
                            }
                        }

                        input_buffer.drain(..end);
                    }
                }
            }
        }
        {
            let msg = ServerClientMsg::ClientDisconnected(session_id);
            let mut output_buffer: Vec<u8> = Vec::new();
            msg.pack(&mut output_buffer);
            match tx.send(BroadcastMsg::Send(Address::All, output_buffer)) {
                Ok(_) => {}
                Err(e) => println!("error while trying to broadcast exit msg: {e}"),
            }
        }
    });
}
