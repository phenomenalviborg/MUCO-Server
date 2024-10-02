use std::{env, fs::create_dir, net::{IpAddr, Ipv4Addr, SocketAddr}};

use client_db::print_timestamp;
use discoverable_service::register_msdn;
use local_ip_address::local_ip;
use tokio::{net::TcpListener, sync::broadcast};
use crate::{broadcast_msg::BroadcastMsg, client_db::ClientDb};

mod client_db;
mod broadcast_msg;

#[tokio::main]
async fn main() {
    let server_start_time = std::time::SystemTime::now();
    let since_the_epoch = server_start_time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards").as_secs();

    let mut enable_logging: bool = false;
    let args: Vec<String> = env::args().collect();
    if let Some(arg) = args.get(1) {
        if arg == "log" {
            enable_logging = true;
        }
    }

    let path = format!("log_{since_the_epoch}");
    let log_folder_path = if enable_logging {
        println!("logging enabled");
        create_dir(&path).unwrap();
        Some(&path[..])
    }
    else {
        None
    };

    let port = 1302;
    let my_local_ip = local_ip().unwrap();

    let _mdns = register_msdn(my_local_ip, port, "muco-server");

    let addr = &SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);
    let listener = TcpListener::bind(addr).await.unwrap();
    

    print_timestamp();
    println!("Server Started at ip: {my_local_ip}:{port}");

    let mut client_db = ClientDb::new();

    let (tx, _) = broadcast::channel::<BroadcastMsg>(100);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        client_db.new_client(socket, addr, tx.clone(), log_folder_path, server_start_time).await;
    }

    //TODO shut down propperly
    // mdns.shutdown().unwrap();
}
