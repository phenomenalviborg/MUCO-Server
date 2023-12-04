use std::net::SocketAddr;

use msgs::{ClientServerMsg, ServerClientMsg, ClientType};
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

use crate::client::Client;

pub struct ClientDb {
    pub user_id_counter: usize,
    pub clients: Vec<Client>,
}

impl ClientDb {
    pub fn new() -> ClientDb {
        ClientDb {
            user_id_counter: 0,
            clients: Vec::new(),
        }
    }

    pub async fn new_client(&mut self, socket: TcpStream, addr: SocketAddr, client_to_main: tokio::sync::mpsc::Sender<(usize, ClientServerMsg)>) {
        let user_id = self.user_id_counter;
        self.user_id_counter += 1;
        let main_to_client = spawn_client_process(socket, client_to_main.clone(), user_id, addr);
        let client = Client { user_id, main_to_client, client_type: None };
        client.main_to_client.send(ServerClientMsg::AssignClientId(user_id)).await.unwrap();
        self.clients.push(client);
        println!("accepted client: {user_id} {addr}");
    }

    pub fn get(&self, client_id: usize) -> Option<&Client> {
        self.clients.iter().find(|client| client.user_id == client_id)
    }
    
    pub fn get_mut(&mut self, client_id: usize) -> Option<&mut Client> {
        self.clients.iter_mut().find(|client| client.user_id == client_id)
    }
    
    pub fn all_clients(&self) -> impl Iterator<Item = &Client> {
        self.clients.iter()
    }

    pub fn other_clients(&self, client_id: usize) -> impl Iterator<Item = &Client> {
        self.clients.iter().filter(move |client| client.user_id != client_id)
    }

    pub fn remove(&mut self, client_id: usize) {
        self.clients.retain(|client| client.user_id != client_id)
    }

    pub async fn process_message(&mut self, msg: ClientServerMsg, client_id: usize) {
        match msg {
            ClientServerMsg::Disconnect => {
                self.remove(client_id);
                for client in self.all_clients() {
                    client.main_to_client.send(ServerClientMsg::ClientDisconnected(client_id)).await.unwrap();
                }
            }
            ClientServerMsg::BroadcastBytesAll(bytes) => {
                for client in self.all_clients() {
                    client.main_to_client.send(ServerClientMsg::BroadcastBytes(client_id, bytes.clone())).await.unwrap();
                }
            }
            ClientServerMsg::BroadcastBytesOther(bytes) => {
                for client in self.other_clients(client_id) {
                    client.main_to_client.send(ServerClientMsg::BroadcastBytes(client_id, bytes.clone())).await.unwrap();
                }
            }
            ClientServerMsg::BinaryMessageTo(addressed, bytes) => {
                let client = self.get(addressed).unwrap();
                client.main_to_client.send(ServerClientMsg::BinaryMessageFrom(addressed, bytes.clone())).await.unwrap();
            }
            ClientServerMsg::SetClientType(client_type) => {
                let client = self.get_mut(client_id).unwrap();
                client.client_type = Some(client_type);

                match client_type {
                    ClientType::Player => {
                        for client in &mut self.other_clients(client_id) {
                            client.main_to_client.send(ServerClientMsg::ClientConnected(client_id)).await.unwrap();
                        }
                    }
                    ClientType::Manager => {}
                }
            }
        }
    }
}

pub fn spawn_client_process(mut socket: TcpStream, client_to_main: tokio::sync::mpsc::Sender<(usize, ClientServerMsg)>, user_id: usize, addr: SocketAddr) -> tokio::sync::mpsc::Sender<ServerClientMsg> {
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
                        println!("client died: {user_id} {addr}");
                        break;
                    }
                    input_buffer.extend(&static_buffer[..len]);
                    
                    while let Some(msg) = ClientServerMsg::dequeue_and_decode(&mut input_buffer) {
                        match msg {
                            Ok(msg) => {
                                client_to_main.send((user_id, msg)).await.unwrap();
                            }
                            Err(e) => println!("error while decode msg: {e}")
                        }
                    }
                }
                result = client_from_main.recv() => {
                    let msg = match result {
                        Some(msg) => msg,
                        None => {
                            println!("client disconnected: {user_id} {addr}");
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
