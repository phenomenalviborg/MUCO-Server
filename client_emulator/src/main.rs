use std::{fs, time::Duration};

use byteorder::{LittleEndian, ReadBytesExt};
use console_cmd::ConsoleCmd;
use console_input::console_input_thread;
use msgs::{client_server_msg::ClientServerMsg, dequeue::dequeue_msg, inter_client_msg::InterClientMsg, relay_server_connection_process::spawn_relay_server_connection_process};

mod console_cmd;
mod console_input;

#[tokio::main]
async fn main() {
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
                            tokio::spawn(play(log_bytes));
                        }
                        ConsoleCmd::Loop(path) => {
                            let log_bytes = fs::read(path).unwrap();
                            tokio::spawn(loop_play(log_bytes));
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
    let lines_to_print = 20;
    let mut line_nr = 0;
    let mut start_time = 0;
    let mut end_time = 0;
    let mut prev_timestamp = None;
    let mut time_stamp_buckets = Vec::new();
    while rdr.len() > 0 {
        let do_print = line_nr < lines_to_print;
        let timestamp = rdr.read_u32::<LittleEndian>().unwrap();
        if let Some(prev_timestamp) = prev_timestamp {
            let duration = timestamp - prev_timestamp;
            while time_stamp_buckets.len() <= duration as usize {
                time_stamp_buckets.push(0);
            }
            time_stamp_buckets[duration as usize] += 1;
        }
        if start_time == 0 {
            start_time = timestamp;
        }
        end_time = timestamp;
        let (begin, end) = dequeue_msg(rdr).unwrap();
        let decode_result = ClientServerMsg::decode(&rdr[begin..end], 0);
        let msg = decode_result.unwrap();
        if do_print {
            print!("{timestamp} ");
            let byte_count = end - begin;
            print!("{byte_count:4} ");
            match msg {
                ClientServerMsg::BinaryMessageTo(_, msg_bytes) => {
                    let decode_result = InterClientMsg::decode(msg_bytes, 0);
                    let msg = decode_result.unwrap();
                    print!("{msg:?}");
                }
                _ => print!("{msg:?}")
            }
            println!();
        }
        rdr = &rdr[end..];
        line_nr += 1;
        prev_timestamp = Some(timestamp);
    }

    let duration = end_time - start_time;
    let total_bytes = log_bytes.len() - line_nr * 4;
    let seconds = duration as f32 / 1000.0;
    let bytes_per_second = total_bytes as f32 / seconds;
    let kb_per_second = bytes_per_second / 1024.0;
    let msgs_per_second = line_nr as f32 / seconds;

    println!("duration: {seconds}");
    println!("msg count: {line_nr}");
    println!("total bytes: {total_bytes}");
    println!("kb per second: {kb_per_second}");
    println!("msgs per second: {msgs_per_second}");
    println!();

    // let duration_count = line_nr - 1;
    for (i, x) in time_stamp_buckets.iter().copied().enumerate() {
        // let frac_count = x as f32 / duration_count as f32;
        // let perc_count = frac_count * 100.0;
        let frac_time = (i as f32 * x as f32) / duration as f32;
        let perc_time = frac_time * 100.0;
        let fps = 1000.0 / i as f32;
        if perc_time > 1.0 {
            println!("{i:4}ms x {x:3} {perc_time:4.1}% {fps:5.1}fps");
        }
    }
}

fn get_first_timestamp(log_bytes: &[u8]) -> u32 {
    let mut rdr = &log_bytes[..];
    rdr.read_u32::<LittleEndian>().unwrap()
}

async fn play(log_bytes: Vec<u8>) {
    play_(&log_bytes).await;
}

async fn loop_play(log_bytes: Vec<u8>) {
    loop {
        play_(&log_bytes).await;
    }
}

async fn play_(log_bytes: &[u8]) {
    let (server_to_main, mut main_from_server) = tokio::sync::mpsc::channel(100);
    let to_relay_server_process = spawn_relay_server_connection_process(server_to_main, false);
    let start_time = std::time::SystemTime::now().checked_sub(Duration::from_millis(get_first_timestamp(&log_bytes) as u64)).unwrap();
    let mut rdr = &log_bytes[..];
    while rdr.len() > 0 {
        let _recv_result = main_from_server.try_recv();
        let timestamp = rdr.read_u32::<LittleEndian>().unwrap() as i64;
        let since_start = start_time.elapsed().unwrap().as_millis() as i64;
        let delay = timestamp - since_start;
        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;
        }
        let (_begin, end) = dequeue_msg(rdr).unwrap();
        to_relay_server_process.send(rdr[..end].to_owned()).await.unwrap();
        rdr = &rdr[end..];
    }
}
