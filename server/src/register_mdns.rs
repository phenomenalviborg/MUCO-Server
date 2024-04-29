use std::net::IpAddr;

use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn register_msdn(ip: IpAddr, port: u16) -> ServiceDaemon {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    let service_type = "_muco-server._tcp.local.";
    let instance_name = "muco_server";
    let host_name = format!("{ip}.local.");
    let properties = [("property_1", "test"), ("property_2", "1234")];

    let my_service = ServiceInfo::new(
        service_type,
        instance_name,
        &host_name,
        ip,
        port,
        &properties[..],
    ).unwrap();

    mdns.register(my_service).expect("Failed to register our service");
    mdns
}
