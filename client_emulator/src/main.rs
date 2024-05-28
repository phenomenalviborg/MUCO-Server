use msgs::relay_server_connection_process::spawn_relay_server_connection_process;

#[tokio::main]
async fn main() {
    let (server_to_main, mut _main_from_server) = tokio::sync::mpsc::channel(100);
    let _to_relay_server_process = spawn_relay_server_connection_process(server_to_main);
    loop {
        
    }
}
