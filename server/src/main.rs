use std::net::{Ipv4Addr, SocketAddr, IpAddr};

use local_ip_address::local_ip;
use msgs::client_server_msg::ClientServerMsg;
use tokio::net::TcpListener;
use crate::client_db::ClientDb;

mod client_db;
mod client;

#[tokio::main]
async fn main() {
    let port = 1302;
    let addr = &SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);
    let listener = TcpListener::bind(addr).await.unwrap();
    
    let my_local_ip = local_ip().unwrap();
    println!("Server Started at ip: {my_local_ip}:{port}");

    let (client_to_main, mut main_from_client) = tokio::sync::mpsc::channel::<(u32, ClientServerMsg)>(100);

    let mut client_db = ClientDb::new();

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (socket, addr) = result.unwrap();
                client_db.new_client(socket, addr, client_to_main.clone()).await;
            }
            result = main_from_client.recv() => {
                let (session_id, msg) = result.unwrap();
                client_db.process_message(msg, session_id).await;
            }
        }
    }
}
