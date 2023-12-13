use std::{net::SocketAddr, collections::VecDeque};

use msgs::{client_server_msg::{ClientServerMsg, Address}, server_client_msg::ServerClientMsg, client_type::ClientType};
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

use crate::client::Client;

pub struct ClientDb {
    pub session_id_counter: u32,
    pub clients: Vec<Client>,
}

impl ClientDb {
    pub fn new() -> ClientDb {
        ClientDb {
            session_id_counter: 0,
            clients: Vec::new(),
        }
    }

    pub async fn new_client(&mut self, socket: TcpStream, addr: SocketAddr, client_to_main: tokio::sync::mpsc::Sender<(u32, ClientServerMsg)>) {
        let session_id = self.session_id_counter;
        self.session_id_counter += 1;
        let main_to_client = spawn_client_process(socket, client_to_main.clone(), session_id, addr);
        let client = Client { session_id, main_to_client, client_type: None };
        client.main_to_client.send(ServerClientMsg::AssignSessionId(session_id)).await.unwrap();
        self.clients.push(client);
        println!("accepted client: {session_id} {addr}");
    }

    pub fn get_mut(&mut self, session_id: u32) -> Option<&mut Client> {
        self.clients.iter_mut().find(|client| client.session_id == session_id)
    }
    
    pub fn remove(&mut self, session_id: u32) {
        self.clients.retain(|client| client.session_id != session_id)
    }

    pub async fn send_server_client_msg(&mut self, address: Address, msg: ServerClientMsg, disconnected_client_queue: &mut VecDeque<u32>) {
        let mut i = 0;
        while i < self.clients.len() {
            let client = &self.clients[i];
            if address.includes(client.session_id) {
                let result = client.main_to_client.send(msg.clone()).await;
                match result {
                    Ok(_) => {},
                    Err(e) => {
                        println!("error sending message: {e}");
                        println!("removing client: {}", client.session_id);
                        disconnected_client_queue.push_back(client.session_id);
                        self.clients.remove(i);
                        continue;
                    }
                }
            }
            i += 1;
        }
    }

    pub async fn process_message(&mut self, msg: ClientServerMsg, session_id: u32, disconnected_client_queue: &mut VecDeque<u32>) {
        match msg {
            ClientServerMsg::Disconnect => {
                self.remove(session_id);
                self.send_server_client_msg(Address::All, ServerClientMsg::ClientDisconnected(session_id), disconnected_client_queue).await;
            }
            ClientServerMsg::BinaryMessageTo(address, bytes) => {
                self.send_server_client_msg(address, ServerClientMsg::InterClient(session_id, bytes), disconnected_client_queue).await;
            }
            ClientServerMsg::SetClientType(client_type) => {
                let client = self.get_mut(session_id).unwrap();
                client.client_type = Some(client_type);

                match client_type {
                    ClientType::Player => {
                        self.send_server_client_msg(Address::Other (session_id), ServerClientMsg::ClientConnected(session_id), disconnected_client_queue).await;
                    }
                    ClientType::Manager => {}
                }
            }
        }
    }
}

pub fn spawn_client_process(mut socket: TcpStream, client_to_main: tokio::sync::mpsc::Sender<(u32, ClientServerMsg)>, session_id: u32, addr: SocketAddr) -> tokio::sync::mpsc::Sender<ServerClientMsg> {
    let (main_to_client, mut client_from_main) = tokio::sync::mpsc::channel::<ServerClientMsg>(100);
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();
        let mut output_buffer = Vec::new();

        loop {
            tokio::select! {
                result = socket.read(&mut static_buffer) => {
                    let len = match result {
                        Ok(len) => len,
                        Err(e) => {
                            println!("error while reading from socket: {e}");
                            return;
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
                                client_to_main.send((session_id, msg)).await.unwrap();
                            }
                            Err(e) => println!("error while decode msg: {e}")
                        }
                    }
                }
                result = client_from_main.recv() => {
                    let msg = match result {
                        Some(msg) => msg,
                        None => {
                            println!("client disconnected: {session_id} {addr}");
                        break;
                        }
                    };

                    match send_client_msg(msg, &mut socket, &mut output_buffer).await {
                        Ok(_) => {},
                        Err(e) => {
                            println!("disconnecting because of error while writing to client: {e}");
                            return;
                        }
                    }
                }
            }
        }
    });
    main_to_client
}

pub async fn send_client_msg(msg: ServerClientMsg, socket: &mut TcpStream, output_buffer: &mut Vec<u8>) -> anyhow::Result<()> {
    output_buffer.clear();
    msg.pack(output_buffer);
    socket.write_all(&output_buffer).await?;
    Ok(())
}
