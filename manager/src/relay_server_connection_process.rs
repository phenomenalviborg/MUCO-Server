use msgs::{server_client_msg::ServerClientMsg, client_server_msg::ClientServerMsg};
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

pub fn spawn_relay_server_connection_process(server_to_main: tokio::sync::mpsc::Sender<ServerClientMsg>) -> tokio::sync::mpsc::Sender<ClientServerMsg> {
    let (main_to_server, mut server_from_main) = tokio::sync::mpsc::channel::<ClientServerMsg>(100);
    tokio::spawn(async move {
        let mut static_buffer = [0; 1024];
        let mut input_buffer = Vec::new();
        let mut output_buffer = Vec::new();

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
                    
                    while let Some(msg) = ServerClientMsg::dequeue_and_decode(&mut input_buffer) {
                        match msg {
                            Ok(msg) => {
                                server_to_main.send(msg).await.unwrap();
                            }
                            Err(e) => println!("error while decode msg: {e}")
                        }
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

                    output_buffer.clear();
                    msg.pack(&mut output_buffer);
                    stream.write_all(&output_buffer).await.unwrap();
                }
            }
        }
    });
    main_to_server
}
