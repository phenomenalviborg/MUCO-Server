use serde::{Deserialize, Serialize};

/// Messages sent from the manager backend to the manager frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ManagerClientMsg {
    /// A new server was discovered on the network
    #[serde(rename = "server_discovered")]
    ServerDiscovered {
        host: String,
        name: String,
    },

    /// A previously discovered server is no longer available
    #[serde(rename = "server_lost")]
    ServerLost {
        host: String,
    },

    /// Initial list of currently discovered servers (sent on connection)
    #[serde(rename = "discovered_servers")]
    DiscoveredServers {
        servers: Vec<DiscoveredServerInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServerInfo {
    pub host: String,
    pub name: String,
}
