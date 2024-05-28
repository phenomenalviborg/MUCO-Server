use std::io::stdin;
use std::thread;

use tokio::sync::mpsc::{self, Receiver};

pub fn console_input_thread() -> Receiver<String>{
    let (sender, receiver) = mpsc::channel(100);
    thread::spawn(move || {
        pollster::block_on(console_input_loop(sender))
    });
    receiver
}

pub async fn console_input_loop(sender: mpsc::Sender<String>) {
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        sender.send(input).await.unwrap();
    }
}
