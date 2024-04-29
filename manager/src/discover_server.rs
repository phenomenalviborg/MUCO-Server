use mdns_sd::{ServiceDaemon, ServiceEvent};

pub fn find_local_server_ip() -> Option<String> {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    let service_type = "_muco-server._tcp.local.";
    let receiver = mdns.browse(service_type).expect("Failed to browse");

    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let addresses = info.get_addresses();
                let addr = addresses.iter().next().unwrap();
                mdns.shutdown().unwrap();
                let port = info.get_port();
                let s = format!("{addr}:{port}");
                return Some(s);
            }
            _ => {}
        }
    }

    mdns.shutdown().unwrap();
    None
}
