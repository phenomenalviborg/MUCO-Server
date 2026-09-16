use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::env;

/// Discover the relay server address.
///
/// Priority:
/// 1. `MUCO_RELAY_HOST` environment variable (e.g. "172.234.96.146:1302")
/// 2. mDNS browsing on `_muco-server._tcp.local.` (with graceful fallback on failure)
pub fn find_local_server_ip() -> Option<String> {
    // Check env var first – this bypasses mDNS entirely, which is needed in
    // environments where the mdns-sd crate's internal UDP signal sockets fail
    // (EPERM on send_to, e.g. on Linode and other cloud VMs).
    if let Ok(host) = env::var("MUCO_RELAY_HOST") {
        let host = host.trim().to_string();
        if !host.is_empty() {
            println!("using MUCO_RELAY_HOST: {host}");
            return Some(host);
        }
    }

    // Fall back to mDNS, handling errors gracefully.
    let mdns = match ServiceDaemon::new() {
        Ok(mdns) => mdns,
        Err(e) => {
            println!("mDNS daemon creation failed: {e}");
            return None;
        }
    };

    let receiver = match mdns.browse("_muco-server._tcp.local.") {
        Ok(receiver) => receiver,
        Err(e) => {
            println!("mDNS browse failed: {e}");
            let _ = mdns.shutdown();
            return None;
        }
    };

    // Block for a resolution (relay is local, so this should arrive quickly).
    match receiver.recv() {
        Ok(ServiceEvent::ServiceResolved(info)) => {
            let addr = match info.get_addresses().iter().next() {
                Some(addr) => addr.clone(),
                None => {
                    let _ = mdns.shutdown();
                    return None;
                }
            };
            let port = info.get_port();
            let _ = mdns.shutdown();
            return Some(format!("{addr}:{port}"));
        }
        Ok(_) => {
            // Non-resolved event (e.g. ServiceFound without address yet) –
            // treat it as not-found; the outer loop in the caller will retry.
            let _ = mdns.shutdown();
            None
        }
        Err(e) => {
            println!("mDNS receiver error: {e}");
            let _ = mdns.shutdown();
            None
        }
    }
}