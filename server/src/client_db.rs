use std::net::SocketAddr;

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

    pub fn get(&self, session_id: u32) -> Option<&Client> {
        self.clients.iter().find(|client| client.session_id == session_id)
    }
    
    pub fn get_mut(&mut self, session_id: u32) -> Option<&mut Client> {
        self.clients.iter_mut().find(|client| client.session_id == session_id)
    }
    
    pub fn all_clients(&self) -> impl Iterator<Item = &Client> {
        self.clients.iter()
    }

    pub fn other_clients(&self, session_id: u32) -> impl Iterator<Item = &Client> {
        self.clients.iter().filter(move |client| client.session_id != session_id)
    }

    pub fn remove(&mut self, session_id: u32) {
        self.clients.retain(|client| client.session_id != session_id)
    }

    pub async fn process_message(&mut self, msg: ClientServerMsg, session_id: u32) {
        match msg {
            ClientServerMsg::Disconnect => {
                self.remove(session_id);
                for client in self.all_clients() {
                    client.main_to_client.send(ServerClientMsg::ClientDisconnected(session_id)).await.unwrap();
                }
            }
            ClientServerMsg::BinaryMessageTo(addresse, bytes) => {
                match addresse {
                    Address::Client (session_id) => {
                        let client = self.get(session_id).unwrap();
                        client.main_to_client.send(ServerClientMsg::BinaryMessageFrom(session_id, bytes.clone())).await.unwrap();
                    }
                    Address::All => {
                        for client in self.all_clients() {
                            client.main_to_client.send(ServerClientMsg::BinaryMessageFrom(session_id, bytes.clone())).await.unwrap();
                        }
                    }
                    Address::Other => {
                        for client in self.other_clients(session_id) {
                            client.main_to_client.send(ServerClientMsg::BinaryMessageFrom(session_id, bytes.clone())).await.unwrap();
                        }
                    }
                }
                
            }
            ClientServerMsg::SetClientType(client_type) => {
                let client = self.get_mut(session_id).unwrap();
                client.client_type = Some(client_type);

                match client_type {
                    ClientType::Player => {
                        for client in &mut self.other_clients(session_id) {
                            client.main_to_client.send(ServerClientMsg::ClientConnected(session_id)).await.unwrap();
                        }
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
                    
                    while let Some(msg) = ClientServerMsg::dequeue_and_decode(&mut input_buffer) {
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

                    send_client_msg(msg, &mut socket, &mut output_buffer).await;
                }
            }
        }
    });
    main_to_client
}

pub async fn send_client_msg(msg: ServerClientMsg, socket: &mut TcpStream, output_buffer: &mut Vec<u8>) {
    output_buffer.clear();
    msg.pack(output_buffer);
    socket.write_all(&output_buffer).await.unwrap();
}
