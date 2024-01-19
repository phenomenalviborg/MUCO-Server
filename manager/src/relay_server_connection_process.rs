use msgs::dequeue::dequeue_msg;
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

pub fn spawn_relay_server_connection_process(server_to_main: tokio::sync::mpsc::Sender<Vec<u8>>) -> tokio::sync::mpsc::Sender<Vec<u8>> {
    let (main_to_server, mut server_from_main) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();

        let mut stream = TcpStream::connect("localhost:1302").await.unwrap();

        loop {
            tokio::select! {
                result = stream.read(&mut static_buffer) => {
                    let len = match result {
                        Ok(len) => len,
                        Err(e) => {
                            println!("error while reading from socket: {e}");
                            return;
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

                    stream.write_all(&msg).await.unwrap();
                }
            }
        }
    });
    main_to_server
}
