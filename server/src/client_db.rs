use std::net::SocketAddr;

use msgs::{client_server_msg::ClientServerMsg, server_client_msg::ServerClientMsg, client_type::ClientType};
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

    pub async fn new_client(&mut self, socket: TcpStream, addr: SocketAddr, tx: broadcast::Sender<BroadcastMsg>) {
        let session_id = self.session_id_counter;
        spawn_client_process(socket, tx, session_id, addr);
        self.session_id_counter += 1;
        println!("accepted client: {session_id} {addr}");
    }
}

pub fn spawn_client_process(mut socket: TcpStream, tx: broadcast::Sender<BroadcastMsg>, session_id: u32, addr: SocketAddr) {
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();
        let mut output_buffer: Vec<u8> = Vec::new();

        match send_client_msg(ServerClientMsg::AssignSessionId(session_id), &mut socket, &mut output_buffer).await {
            Ok(_) => {},
            Err(e) => {
                println!("disconnecting because of error while writing to client: {e}");
                return;
            }
        }
        
        let mut rx = tx.subscribe();
        loop {
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

                    let response = match broadcast_msg {
                        BroadcastMsg::ClientDisconnected(sender) => {
                            ServerClientMsg::ClientDisconnected(sender)
                        }
                        BroadcastMsg::ClientServerMsg(sender, msg) => {
                            match msg {
                                ClientServerMsg::Disconnect => {
                                    if sender == session_id {
                                        break;
                                    }
                                    continue;
                                }
                                ClientServerMsg::BinaryMessageTo(to, content) => {
                                    if !to.includes(session_id) {
                                        continue
                                    }
                                    ServerClientMsg::InterClient(sender, content)
                                }
                                ClientServerMsg::SetClientType(new_client_typ) => {
                                    if sender == session_id {
                                        continue;
                                    }
                                    else if new_client_typ == ClientType::Player {
                                        ServerClientMsg::ClientConnected(sender)
                                    }
                                    else {
                                        continue;
                                    }
                                }
                                ClientServerMsg::Kick(to_kick) => {
                                    if session_id == to_kick {
                                        break;
                                    }
                                    continue;
                                }
                            }
                        }
                    };
                    match send_client_msg(response, &mut socket, &mut output_buffer).await {
                        Ok(_) => {},
                        Err(e) => {
                            println!("disconnecting because of error while writing to client: {e}");
                            break;
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
                    
                    while let Some(msg) = ClientServerMsg::dequeue_and_decode(&mut input_buffer, session_id) {
                        match msg {
                            Ok(msg) => {
                                match tx.send(BroadcastMsg::ClientServerMsg(session_id, msg)) {
                                    Ok(_) => {}
                                    Err(e) => println!("error while sending inter msg: {e}")
                                };
                            }
                            Err(e) => println!("error while decode msg: {e}")
                        }
                    }
                }
            }
        }
        match tx.send(BroadcastMsg::ClientDisconnected(session_id)) {
            Ok(_) => {}
            Err(e) => println!("error while sending disconnect msg: {e}")
        };
    });
}

pub async fn send_client_msg(msg: ServerClientMsg, socket: &mut TcpStream, output_buffer: &mut Vec<u8>) -> anyhow::Result<()> {
    output_buffer.clear();
    msg.pack(output_buffer);
    socket.write_all(&output_buffer).await?;
    Ok(())
}
