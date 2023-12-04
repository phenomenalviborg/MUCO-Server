use std::{sync::Arc, collections::HashMap, convert::Infallible};

use console_input::console_input_thread;
use context::{MucoContextRef, MucoContext};
use msgs::{client_server_msg::ClientServerMsg, client_type::ClientType};
use server::Server;
use status::Status;
use tokio::sync::{mpsc, RwLock};
use warp::{filters::ws::Message, reject::Rejection, Filter};

mod color;
mod connection_status;
mod console_input;
mod context;
mod handler;
mod headset_data;
mod server;
mod status;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;

const SAVE_DATA_PATH: &str = "server_data.txt";
const DEFAULT_SESSION_DURATION: i64 = 30 * 60;

#[derive(Debug, Clone)]
pub struct Client {
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[tokio::main]
async fn main() {
    let status = Status::load(SAVE_DATA_PATH).unwrap_or(Status::new());
    
    let context = MucoContext {
        clients: HashMap::new(),
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

    let (server_to_main, mut main_from_server) = tokio::sync::mpsc::channel(100);
    let server = Server::new(server_to_main);

    server.main_to_server.send(ClientServerMsg::SetClientType (ClientType::Manager)).await.unwrap();

    loop {
        let Some(msg) = main_from_server.recv().await else { break };
        dbg!(msg);
    }
}

fn with_context(context_ref: MucoContextRef) -> impl Filter<Extract = (MucoContextRef,), Error = Infallible> + Clone {
    warp::any().map(move || context_ref.clone())
}

