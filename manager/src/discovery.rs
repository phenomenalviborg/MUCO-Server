use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const SERVICE_TYPE: &str = "_muco-manager._tcp.local.";
const STALE_THRESHOLD: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredServer {
    pub host: String,
    pub name: String,
    pub last_seen: Instant,
}

pub type DiscoveryEvent = (DiscoveryEventType, DiscoveredServer);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEventType {
    ServerDiscovered,
    ServerLost,
}

pub struct DiscoveryService {
    servers: Arc<RwLock<HashMap<String, DiscoveredServer>>>,
    tx: broadcast::Sender<DiscoveryEvent>,
}

impl DiscoveryService {
    /// Create a new discovery service
    ///
    /// # Arguments
    /// * `local_ips` - List of IP addresses that belong to this manager (to filter out self-discovery)
    pub fn new(local_ips: Vec<IpAddr>) -> (Self, broadcast::Receiver<DiscoveryEvent>) {
        let servers = Arc::new(RwLock::new(HashMap::new()));
        let (tx, rx) = broadcast::channel(100);

        let service = DiscoveryService {
            servers: servers.clone(),
            tx: tx.clone(),
        };

        // Spawn discovery thread
        let servers_clone = servers.clone();
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            run_discovery_loop(servers_clone, tx_clone, local_ips);
        });

        (service, rx)
    }

    pub fn get_all_servers(&self) -> Vec<DiscoveredServer> {
        let servers = self.servers.read().unwrap();
        servers.values().cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.tx.subscribe()
    }
}

fn run_discovery_loop(
    servers: Arc<RwLock<HashMap<String, DiscoveredServer>>>,
    tx: broadcast::Sender<DiscoveryEvent>,
    local_ips: Vec<IpAddr>,
) {
    let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");
    let receiver = mdns.browse(SERVICE_TYPE).expect("Failed to browse for services");

    loop {
        // Try to discover new servers
        match receiver.try_recv() {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let addresses = info.get_addresses();
                if let Some(addr) = addresses.iter().next() {
                    // Skip if this is one of our own IP addresses
                    if local_ips.contains(addr) {
                        continue;
                    }

                    let port = info.get_port();
                    let host = format!("{addr}:{port}");
                    let now = Instant::now();

                    let mut servers_write = servers.write().unwrap();
                    let is_new = !servers_write.contains_key(&host);

                    let server = DiscoveredServer {
                        host: host.clone(),
                        name: format!("MUCO Manager ({})", host),
                        last_seen: now,
                    };

                    servers_write.insert(host.clone(), server.clone());

                    if is_new {
                        drop(servers_write); // Release lock before broadcasting
                        let _ = tx.send((DiscoveryEventType::ServerDiscovered, server));
                    }
                }
            }
            Ok(_other_event) => {
                // Ignore other mDNS events (ServiceFound, ServiceRemoved, etc.)
            }
            Err(_) => {
                // No events available right now
            }
        }

        // Clean up stale servers
        {
            let mut servers_write = servers.write().unwrap();
            let now = Instant::now();
            let mut to_remove = Vec::new();

            for (host, server) in servers_write.iter() {
                if now.duration_since(server.last_seen) > STALE_THRESHOLD {
                    to_remove.push((host.clone(), server.clone()));
                }
            }

            for (host, server) in to_remove {
                servers_write.remove(&host);
                drop(servers_write); // Release lock before broadcasting
                let _ = tx.send((DiscoveryEventType::ServerLost, server.clone()));
                servers_write = servers.write().unwrap(); // Re-acquire lock
            }
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}
