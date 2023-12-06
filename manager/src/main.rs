use std::{sync::Arc, collections::HashMap, convert::Infallible};

use console_input::console_input_thread;
use context::{MucoContextRef, MucoContext};
use msgs::{client_server_msg::ClientServerMsg, client_type::ClientType, server_client_msg::ServerClientMsg};
use process_server_client_msg::process_server_client_msg;
use relay_server_connection_process::spawn_relay_server_connection_process;
use status::Status;
use tokio::sync::{mpsc::Receiver, RwLock};
use warp::{reject::Rejection, Filter};

mod color;
mod connection_status;
mod console_input;
mod context;
mod handler;
mod headset_data;
mod inter_client_msg;
mod player_data_msg;
mod player_data;
mod process_server_client_msg;
mod relay_server_connection_process;
mod status;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;

const SAVE_DATA_PATH: &str = "server_data.txt";
const DEFAULT_SESSION_DURATION: i64 = 30 * 60;

#[tokio::main]
async fn main() {
    let status = Status::load(SAVE_DATA_PATH).unwrap_or(Status::new());

    let (server_to_main, main_from_server) = tokio::sync::mpsc::channel(100);
    let to_relay_server_process = spawn_relay_server_connection_process(server_to_main);

    to_relay_server_process.send(ClientServerMsg::SetClientType (ClientType::Manager)).await.unwrap();
    
    let context = MucoContext {
        to_relay_server_process,
        connection_id_to_player: HashMap::new(),
        to_frontend_senders: HashMap::new(),
        status,
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

    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
    });

    relay_server(main_from_server, context_ref).await;
}

pub async fn relay_server(mut main_from_server: Receiver<ServerClientMsg>, context_ref: MucoContextRef) {
    loop {
        let Some(msg) = main_from_server.recv().await else { break };
        process_server_client_msg(msg, &context_ref).await;
    }
}

fn with_context(context_ref: MucoContextRef) -> impl Filter<Extract = (MucoContextRef,), Error = Infallible> + Clone {
    warp::any().map(move || context_ref.clone())
}
