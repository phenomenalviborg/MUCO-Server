use std::{collections::HashMap, convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use console_input::console_input_thread;
use context::{MucoContextRef, MucoContext};
use msgs::{client_server_msg::ClientServerMsg, client_type::ClientType, server_client_msg::ServerClientMsg};
use process_server_client_msg::process_server_client_msg;
use relay_server_connection_process::spawn_relay_server_connection_process;
use status::Status;
use tokio::sync::RwLock;
use warp::{reject::Rejection, Filter};

mod connection_status;
mod console_input;
mod context;
mod discover_server;
mod handler;
mod headset_data;
mod process_server_client_msg;
mod relay_server_connection_process;
mod status;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;

const SAVE_DATA_PATH: &str = "server_data.txt";
const DEFAULT_SESSION_DURATION: i64 = 30 * 60;

#[tokio::main]
async fn main() {
    let status = match Status::load(SAVE_DATA_PATH) {
        Ok(status) => status,
        Err(e) => {
            println!("error while loading headset data at startup: {e}");
            Status::new()
        }
    };

    let (server_to_main, mut main_from_server) = tokio::sync::mpsc::channel(100);
    let to_relay_server_process = spawn_relay_server_connection_process(server_to_main);

    {
        let msg = ClientServerMsg::SetClientType (ClientType::Manager);
        let mut bytes = Vec::new();
        msg.pack(&mut bytes);
        to_relay_server_process.send(bytes).await.unwrap();
    }
    
    let context = MucoContext {
        to_relay_server_process,
        connection_id_to_player: HashMap::new(),
        to_frontend_senders: HashMap::new(),
        status,
        status_generation: 0,
        unknown_connections: Vec::new(),
    };

    let context_ref = Arc::new(RwLock::new(context));

    console_input_thread(context_ref.clone());

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_context(context_ref.clone()))
        .and_then(handler::ws_handler);

    let routes = health_route
        .or(ws_route)
        .with(warp::cors().allow_any_origin());

    let port = 8000;
    let addr = SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);

    tokio::spawn(async move {
        warp::serve(routes).run(addr).await;
    });

    update_clients_periodically(context_ref.clone()).await;

    loop {
        let Some(msg_bytes) = main_from_server.recv().await else { break };
        let result = ServerClientMsg::decode(&msg_bytes);
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                println!("error while decoding server client msg: {e}");
                continue;
            }
        };
        process_server_client_msg(msg, &context_ref).await;
    }
}

async fn update_clients_periodically(context_ref: MucoContextRef) {
    let mut frontend_status_generation = 0;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            {
                let context = context_ref.read().await;
                if context.status_generation != frontend_status_generation {
                    context.update_clients().await;
                    context.status.save(SAVE_DATA_PATH).unwrap();
                    frontend_status_generation = context.status_generation;
                }
                if context.unknown_connections.is_empty() {
                    continue;
                }
            }
            context_ref.write().await.request_unknown_device_ids().await;
        }
    });
}

fn with_context(context_ref: MucoContextRef) -> impl Filter<Extract = (MucoContextRef,), Error = Infallible> + Clone {
    warp::any().map(move || context_ref.clone())
}
