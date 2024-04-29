use std::net::{Ipv4Addr, SocketAddr, IpAddr};

use local_ip_address::local_ip;
use tokio::{net::TcpListener, sync::broadcast};
use crate::{broadcast_msg::BroadcastMsg, client_db::ClientDb, register_mdns::register_msdn};

mod client_db;
mod broadcast_msg;
mod register_mdns;

#[tokio::main]
async fn main() {
    let port = 1302;
    let my_local_ip = local_ip().unwrap();

    let _mdns = register_msdn(my_local_ip, port);

    let addr = &SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);
    let listener = TcpListener::bind(addr).await.unwrap();
    
    println!("Server Started at ip: {my_local_ip}:{port}");

    let mut client_db = ClientDb::new();

    let (tx, _) = broadcast::channel::<BroadcastMsg>(100);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        client_db.new_client(socket, addr, tx.clone()).await;
    }

    //TODO shut down propperly
    // mdns.shutdown().unwrap();
}
