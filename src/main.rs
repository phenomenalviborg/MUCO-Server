use std::net::{Ipv4Addr, SocketAddr, IpAddr};

use local_ip_address::local_ip;
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}, sync::broadcast};
use crate::msgs::{ClientServerMsg, ServerClientMsg, IntercomMsg};

mod msgs;

#[tokio::main]
async fn main() {
    let port = 1302;
    let addr = &SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);
    let listener = TcpListener::bind(addr).await.unwrap();
    
    let my_local_ip = local_ip().unwrap();
    println!("Server Started at ip: {my_local_ip}:{port}");

    let (tx, _) = broadcast::channel::<IntercomMsg>(100);
    let mut user_id_counter = 0;

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        let user_id = user_id_counter;
        user_id_counter += 1;

        println!("accepted client: {user_id} {addr}");

        tx.send(IntercomMsg::NewClient (user_id)).unwrap();

        tokio::spawn(async move {
            let mut static_buffer = [0; 1024];
            let mut input_buffer = Vec::new();
            let mut output_buffer = Vec::new();

            loop {
                tokio::select! {
                    result = socket.read(&mut static_buffer) => {
                        let len = result.unwrap();
                        if len == 0 {
                            println!("client died: {user_id} {addr}");
                            break;
                        }
                        input_buffer.extend(&static_buffer[..len]);
                        
                        while let Some(msg) = ClientServerMsg::dequeue(&mut input_buffer) {
                            tx.send(IntercomMsg::ClientServerMsg (user_id,msg)).unwrap();
                        }
                    }
                    result = rx.recv() => {

                        let intercom_msg = match result {
                            Ok(msg) => msg,
                            Err(err) => {
                                println!("msg error: {err}");
                                continue;
                            }
                        };

                        match intercom_msg {
                            IntercomMsg::NewClient (new_user_id) => {
                                let msg = if user_id == new_user_id {
                                    ServerClientMsg::AssignClientId (user_id)
                                }
                                else {
                                    ServerClientMsg::ClientConnected (new_user_id)
                                };
                                send_client_msg(msg, &mut socket, &mut output_buffer).await;
                            }
                            IntercomMsg::ClientServerMsg(sender, received_msg) => {
                                match received_msg {
                                    ClientServerMsg::Disconnect => {
                                        if sender == user_id {
                                            println!("client disconnected: {user_id} {addr}");
                                            break;
                                        }
                                        else {
                                            let msg = ServerClientMsg::ClientDisconnected (sender);
                                            send_client_msg(msg, &mut socket, &mut output_buffer).await;
                                        }
                                    }
                                    ClientServerMsg::BroadcastBytesAll (bytes) => {
                                        let msg = ServerClientMsg::BroadcastBytes (sender, bytes);
                                        send_client_msg(msg, &mut socket, &mut output_buffer).await;
                                    }
                                    ClientServerMsg::BroadcastBytesOther (bytes) => {
                                        if sender != user_id {
                                            let msg = ServerClientMsg::BroadcastBytes (sender, bytes);
                                            send_client_msg(msg, &mut socket, &mut output_buffer).await;
                                        }
                                    }
                                    ClientServerMsg::BinaryMessageTo (address, bytes) => {
                                        if address == user_id {
                                            let msg = ServerClientMsg::BinaryMessageFrom (sender, bytes);
                                            send_client_msg(msg, &mut socket, &mut output_buffer).await;
                                        }
                                    }
                                    ClientServerMsg::Unsupported (msg_type_index) => {
                                        println!("message type index not supported: {msg_type_index}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

pub async fn send_client_msg(msg: ServerClientMsg, socket: &mut TcpStream, output_buffer: &mut Vec<u8>) {
    output_buffer.clear();
    msg.pack(output_buffer);
    socket.write_all(&output_buffer).await.unwrap();
}
