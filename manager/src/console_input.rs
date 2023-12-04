use std::io::stdin;
use std::thread;

use uuid::Uuid;

use crate::{SAVE_DATA_PATH, DEFAULT_SESSION_DURATION};
use crate::color::Color;
use crate::connection_status::ConnectionStatus;
use crate::context::MucoContextRef;
use crate::headset_data::{PersistentHeadsetData, TempHeadsetData, HeadsetData, SessionState};
use crate::status::Status;
use crate::ws::{process_client_msg, ServerResponse};

pub fn console_input_thread(context_ref: MucoContextRef) {
    thread::spawn(move || {
        pollster::block_on(console_input_loop(context_ref))
    });
}

pub async fn console_input_loop(context_ref: MucoContextRef) {
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        match process_console_input(&input.trim(), &context_ref).await {
            Ok(_) => {}
            Err(e) => println!("error: {e}")
        }
    }
}

pub async fn process_console_input(input: &str, context_ref: &MucoContextRef) -> anyhow::Result<()> {
    let (message_type, rem) = match input.find(" ") {
        Some(i) => (&input[..i], input[i+1..].trim()),
        None => (&input[..], ""),
    };

    match message_type {
        "add" => {
            let uuid = Uuid::new_v4().simple().to_string();
            let unique_device_id =uuid;
            let mut context = context_ref.write().await;
            let persistant = PersistentHeadsetData {
                color: Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },
                name: "HeadsetName".to_string(),
            };
            let temp = TempHeadsetData {
                connection_status: ConnectionStatus::Connected,
                session_state: SessionState::Paused(0),
                session_duration: DEFAULT_SESSION_DURATION,
            };

            let data = HeadsetData {
                persistent: persistant,
                temp,
            };

            context.status.headsets.insert(unique_device_id.to_string(), data);
            context.update_clients().await;
        }
        "save" => {
            let status = context_ref.read().await.status.clone();
            status.save(SAVE_DATA_PATH)?;
        }
        "load" => {
            let status = Status::load(SAVE_DATA_PATH)?;
            let mut context = context_ref.write().await;
            context.status = status;
            context.update_clients().await;
        }
        "status" => {
            let status = context_ref.read().await.status.clone();
            let json = serde_json::to_string_pretty(&status)?;
            println!("{json}");
        }
        ">" => {
            let client_msg = serde_json::from_str(rem)?;
            let response = process_client_msg(client_msg, context_ref).await?;
            match response {
                ServerResponse::Reply(reply) => println!("{reply}"),
                ServerResponse::UpdateClients => context_ref.read().await.update_clients().await,
                ServerResponse::Nothing => {}
            }
        }
        _ => println!("input not recognized"),
    }
    Ok(())
}

