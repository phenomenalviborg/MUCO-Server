use std::{sync::Arc, collections::HashMap, convert::Infallible};

use connection_status::ConnectionStatus;
use console_input::console_input_thread;
use context::{MucoContextRef, MucoContext};
use inter_client_msg::InterClientMsg;
use msgs::{client_server_msg::ClientServerMsg, client_type::ClientType, server_client_msg::ServerClientMsg};
use player_data::PlayerAttribute;
use player_data_msg::PlayerDataMsg;
use server::Server;
use status::Status;
use tokio::sync::{mpsc, RwLock};
use warp::{filters::ws::Message, reject::Rejection, Filter};

use crate::headset_data::HeadsetData;

mod color;
mod connection_status;
mod console_input;
mod context;
mod handler;
mod headset_data;
mod inter_client_msg;
mod player_data_msg;
mod player_data;
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
        connection_id_to_player: HashMap::new(),
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
        match msg {
            ServerClientMsg::AssignSessionId(session_id) => {
                println!("session id: {session_id}");
            }
            ServerClientMsg::ClientConnected(session_id) => {
                println!("client connected: {session_id}");
            }
            ServerClientMsg::ClientDisconnected(session_id) => {
                let mut context = context_ref.write().await;
                context.disconnect(session_id).await;
            }
            ServerClientMsg::InterClient(sender, input_buffer) => {
                let result = InterClientMsg::decode(&input_buffer, sender);
                let inter_client_msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        println!("error while decodeing msg: {e}");
                        return;
                    }
                };
                
                match inter_client_msg {
                    InterClientMsg::Interaction => {}
                    InterClientMsg::PlayerData (player_data_msg) => {
                        match player_data_msg {
                            PlayerDataMsg::Notify (player_data) => {
                                match player_data {
                                    PlayerAttribute::DeviceId(device_id) => {
                                        let device_id_string = device_id.to_string();
                                        let mut context = context_ref.write().await;
                                        if !context.status.headsets.contains_key(&device_id_string) {
                                            let new_player_data = HeadsetData::new();
                                            context.status.headsets.insert(device_id_string.clone(), new_player_data);
                                        }
                                        let headset = context.status.headsets.get_mut(&device_id_string).unwrap();
                                        headset.temp.connection_status = ConnectionStatus::Connected;
                                        context.connection_id_to_player.insert(sender, device_id_string);
                                        context.update_clients().await;
                                    }
                                    PlayerAttribute::Color => println!("received player color"),
                                    PlayerAttribute::Trans => println!("received player trans"),
                                    PlayerAttribute::Hands => println!("received player hands"),
                                }
                            }
                            PlayerDataMsg::Set(_) => todo!(),
                            PlayerDataMsg::Request => todo!(),
                        }
                    }
                    InterClientMsg::Ping => {}
                }
            }
        }
    }
}

fn with_context(context_ref: MucoContextRef) -> impl Filter<Extract = (MucoContextRef,), Error = Infallible> + Clone {
    warp::any().map(move || context_ref.clone())
}

