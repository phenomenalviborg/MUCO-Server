use std::net::IpAddr;

use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn register_msdn(ip: IpAddr, port: u16, instance_name: &str) -> ServiceDaemon {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    let service_type = format!("_{instance_name}._tcp.local.");
    let host_name = format!("{ip}.local.");
    let properties = [("property_1", "test")];

    let my_service = ServiceInfo::new(
        &service_type,
        instance_name,
        &host_name,
        ip,
        port,
        &properties[..],
    ).unwrap();

    mdns.register(my_service).expect("Failed to register our service");
    mdns
}
