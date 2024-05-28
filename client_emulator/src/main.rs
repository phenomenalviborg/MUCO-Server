use std::fs;

use byteorder::{LittleEndian, ReadBytesExt};
use console_cmd::ConsoleCmd;
use console_input::console_input_thread;
use msgs::{client_server_msg::ClientServerMsg, dequeue::dequeue_msg, inter_client_msg::InterClientMsg, relay_server_connection_process::spawn_relay_server_connection_process};

mod console_cmd;
mod console_input;

#[tokio::main]
async fn main() {
    let (server_to_main, mut _main_from_server) = tokio::sync::mpsc::channel(100);
    let _to_relay_server_process = spawn_relay_server_connection_process(server_to_main);
    let mut console_receiver = console_input_thread();
    loop {
        if let Some(console_str) = console_receiver.recv().await {
            let parse_result = ConsoleCmd::parse(console_str.trim()).await;
            match parse_result {
                Ok(cmd) => {
                    match cmd {
                        ConsoleCmd::Display(path) => {
                            let log_bytes = fs::read(path).unwrap();
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
                    }
                }
                Err(err) => println!("err: {err}")
            }
        }
    }
}
