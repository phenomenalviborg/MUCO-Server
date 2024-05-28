use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

use crate::{dequeue::dequeue_msg, discover_server::find_local_server_ip};

pub fn spawn_relay_server_connection_process(server_to_main: tokio::sync::mpsc::Sender<Vec<u8>>) -> tokio::sync::mpsc::Sender<Vec<u8>> {
    let (main_to_server, mut server_from_main) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    tokio::spawn(async move {
        loop {
            let addr = find_local_server_ip().unwrap();
            println!("found server at address: {addr}");

            let mut static_buffer = [0; 1024];
            let mut input_buffer = Vec::new();

            let mut stream = TcpStream::connect(addr).await.unwrap();

            'connected: loop {
                tokio::select! {
                    result = stream.read(&mut static_buffer) => {
                        let len = match result {
                            Ok(len) => len,
                            Err(e) => {
                                println!("error while reading from socket: {e}, restarting connection");
                                break 'connected;
                            }
                        };
                        if len == 0 {
                            println!("server died");
                            break;
                        }
                        input_buffer.extend(&static_buffer[..len]);
                        
                        while let Some((begin, end)) = dequeue_msg(&mut input_buffer) {
                            let bytes = input_buffer[begin..end].to_vec();
                            server_to_main.send(bytes).await.unwrap();
                            input_buffer.drain(..end);
                        }
                    }
                    result = server_from_main.recv() => {
                        let msg = match result {
                            Some(msg) => msg,
                            None => {
                                println!("server disconnected");
                            break;
                            }
                        };

                        match stream.write_all(&msg).await {
                            Ok(_) => {},
                            Err(err) => {
                                println!("error while writing to stream: {err}, restarting connection proccess");
                                break 'connected;
                            },
                        }
                    }
                }
            }
        }
    });
    main_to_server
}
