use std::net::{Ipv4Addr, SocketAddr, IpAddr};

use local_ip_address::local_ip;
use tokio::{net::TcpListener, sync::broadcast};
use crate::{client_db::ClientDb, broadcast_msg::BroadcastMsg};

mod client_db;
mod broadcast_msg;

#[tokio::main]
async fn main() {
    let port = 1302;
    let addr = &SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);
    let listener = TcpListener::bind(addr).await.unwrap();
    
    let my_local_ip = local_ip().unwrap();
    println!("Server Started at ip: {my_local_ip}:{port}");

    let mut client_db = ClientDb::new();

    let (tx, _) = broadcast::channel::<BroadcastMsg>(100);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        client_db.new_client(socket, addr, tx.clone()).await;
    }
}
