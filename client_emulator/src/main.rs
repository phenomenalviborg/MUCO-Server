use std::{fs, time::Duration};

use byteorder::{LittleEndian, ReadBytesExt};
use console_cmd::ConsoleCmd;
use console_input::console_input_thread;
use msgs::{client_server_msg::ClientServerMsg, dequeue::dequeue_msg, inter_client_msg::InterClientMsg, relay_server_connection_process::spawn_relay_server_connection_process};
use tokio::sync::mpsc::error::TryRecvError;

mod console_cmd;
mod console_input;

#[tokio::main]
async fn main() {
    // let (server_to_main, mut _main_from_server) = tokio::sync::mpsc::channel(100);
    // let _to_relay_server_process = spawn_relay_server_connection_process(server_to_main);
    let mut console_receiver = console_input_thread();
    loop {
        if let Some(console_str) = console_receiver.recv().await {
            let parse_result = ConsoleCmd::parse(console_str.trim()).await;
            match parse_result {
                Ok(cmd) => {
                    match cmd {
                        ConsoleCmd::Display(path) => {
                            let log_bytes = fs::read(path).unwrap();
                            display(&log_bytes);
                        }
                        ConsoleCmd::Play(path) => {
                            let log_bytes = fs::read(path).unwrap();
                            play(&log_bytes).await;
                        }
                    }
                }
                Err(err) => println!("err: {err}")
            }
        }
    }
}

fn display(log_bytes: &[u8]) {
    let mut rdr = &log_bytes[..];
    while rdr.len() > 0 {
        let timestamp = rdr.read_u32::<LittleEndian>().unwrap();
        print!("{timestamp} ");
        let (begin, end) = dequeue_msg(rdr).unwrap();
        let decode_result = ClientServerMsg::decode(&rdr[begin..end], 0);
        let msg = decode_result.unwrap();
        match msg {
            ClientServerMsg::BinaryMessageTo(_, msg_bytes) => {
                let decode_result = InterClientMsg::decode(msg_bytes, 0);
                let msg = decode_result.unwrap();
                print!("{msg:?}");
            }
            _ => print!("{msg:?}")
        }
        println!();
        rdr = &rdr[end..];
    }
}

fn get_first_timestamp(log_bytes: &[u8]) -> u32 {
    let mut rdr = &log_bytes[..];
    rdr.read_u32::<LittleEndian>().unwrap()
}

async fn play(log_bytes: &[u8]) {
    let (server_to_main, mut main_from_server) = tokio::sync::mpsc::channel(100);
    let to_relay_server_process = spawn_relay_server_connection_process(server_to_main, false);
    let start_time = std::time::SystemTime::now().checked_sub(Duration::from_millis(get_first_timestamp(log_bytes) as u64)).unwrap();
    let mut rdr = &log_bytes[..];
    while rdr.len() > 0 {
        let recv_result = main_from_server.try_recv();
        match recv_result {
            Ok(_) => {
                // println!("received msg");
            }
            Err(err) => {
                match err {
                    TryRecvError::Empty => {
                        // println!("empty");
                    }
                    TryRecvError::Disconnected => {
                        // println!("disconnected");
                    }
                }
            }
        }
        let timestamp = rdr.read_u32::<LittleEndian>().unwrap() as i64;
        let since_start = start_time.elapsed().unwrap().as_millis() as i64;
        let delay = timestamp - since_start;
        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;
        }
        let (begin, end) = dequeue_msg(rdr).unwrap();
        // dbg!(&rdr[..end]);
        to_relay_server_process.send(rdr[..end].to_owned()).await.unwrap();
        // println!("sent msg");
        rdr = &rdr[end..];
    }
}
