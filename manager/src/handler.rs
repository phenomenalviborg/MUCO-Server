use crate::{ws, Result, context::MucoContextRef};
use warp::{http::StatusCode, Reply};

pub async fn ws_handler(ws: warp::ws::Ws, context_ref: MucoContextRef) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::frontend_connection_process(socket, context_ref)))
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

