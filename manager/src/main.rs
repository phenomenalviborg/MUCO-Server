use std::{collections::HashMap, convert::Infallible, sync::Arc};

use console_input::console_input_thread;
use context::{MucoContextRef, MucoContext};
use msgs::{client_server_msg::ClientServerMsg, client_type::ClientType, relay_server_connection_process::spawn_relay_server_connection_process, server_client_msg::ServerClientMsg};
use process_server_client_msg::process_server_client_msg;
use status::Status;
use tokio::sync::RwLock;
use warp::{reject::Rejection, Filter};
use connection_info::get_public_ip;
use discoverable_service::register_msdn;

// mod acme; // Disabled for now - too complex for this version
mod connection_info;
mod connection_status;
mod console_input;
mod context;
mod discovery;
mod handler;
mod headset_data;
mod process_server_client_msg;
mod status;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;

const SAVE_DATA_PATH: &str = "server_data.txt";
const DEFAULT_SESSION_DURATION: i64 = 30 * 60;
const PORT: u16 = 9080;

#[tokio::main]
async fn main() {
    let status = match Status::load(SAVE_DATA_PATH) {
        Ok(status) => status,
        Err(e) => {
            println!("error while loading headset data at startup: {e}");
            Status::new()
        }
    };

    let (server_to_main, mut main_from_server) = tokio::sync::mpsc::channel(100);
    let to_relay_server_process = spawn_relay_server_connection_process(server_to_main, true, 888);

    {
        let msg = ClientServerMsg::SetClientType (ClientType::Manager);
        let mut bytes = Vec::new();
        msg.pack(&mut bytes);
        to_relay_server_process.send(bytes).await.unwrap();
    }

    // Collect all local IP addresses to filter out self-discovery
    let mut local_ips = vec![
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), // localhost
    ];

    // Add the primary local network IP
    if let Ok(local_ip) = local_ip_address::local_ip() {
        local_ips.push(local_ip);
    }

    // Add all network interface IPs to be thorough
    if let Ok(all_ips) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in all_ips {
            if !local_ips.contains(&ip) {
                local_ips.push(ip);
            }
        }
    }

    println!("🔍 Filtering self-discovery for IPs: {:?}", local_ips);

    // Initialize discovery service with local IPs to filter out
    let (discovery_service, _discovery_rx) = discovery::DiscoveryService::new(local_ips);
    let discovery_service = Arc::new(discovery_service);

    let context = MucoContext {
        to_relay_server_process,
        connection_id_to_player: HashMap::new(),
        to_frontend_senders: HashMap::new(),
        status,
        status_generation: 0,
        unknown_connections: Vec::new(),
        discovery_service: discovery_service.clone(),
    };

    let context_ref = Arc::new(RwLock::new(context));

    console_input_thread(context_ref.clone());

    // API routes with /api prefix for proxy
    let api_routes = warp::path("api").and(
        warp::path("health").and_then(handler::health_handler)
            .or(warp::path("ws")
                .and(warp::ws())
                .and(with_context(context_ref.clone()))
                .and_then(handler::ws_handler))
    );

    // Root level trust endpoint for SSL certificate verification
    let trust_route = warp::path("trust").and_then(handler::trust_handler);

    let routes = trust_route.or(api_routes)
        .with(warp::cors().allow_any_origin());

    // Start the periodic status update task
    update_clients_periodically(context_ref.clone());

    // Print connection information
    println!("🚀 Starting MUCO Manager backend");
    println!("   Local:   http://127.0.0.1:{}", PORT);

    // Try to get local network IP and register on mDNS
    let _mdns = if let Ok(local_ip) = local_ip_address::local_ip() {
        println!("   Network: http://{}:{}", local_ip, PORT);

        // Register manager on mDNS for discovery by other managers
        let mdns = register_msdn(local_ip, PORT, "muco-manager");
        println!("   Announced via mDNS as _muco-manager._tcp.local.");
        Some(mdns)
    } else {
        None
    };

    // Try to get public IP (non-blocking, with timeout)
    let public_ip_future = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        get_public_ip()
    );

    if let Ok(Some(public_ip)) = public_ip_future.await {
        println!("   Public:  http://{}:{} (requires port forwarding)", public_ip, PORT);
    }

    println!();

    // Check if port is available before starting server
    if let Err(e) = std::net::TcpListener::bind(("0.0.0.0", PORT)) {
        eprintln!("\n❌ ERROR: Failed to bind to port {}", PORT);
        eprintln!("   Reason: {}", e);
        eprintln!("\n💡 Port {} is already in use. This usually means:", PORT);
        eprintln!("   • Another manager is already running");
        eprintln!("   • Another process is using port {}", PORT);
        eprintln!("\n🔍 Check what's using the port:");
        eprintln!("   lsof -i :{}", PORT);
        eprintln!("\n🛑 Kill the existing process:");
        eprintln!("   pkill -f manager");
        eprintln!();
        std::process::exit(1);
    }
    println!("✅ Port {} is available", PORT);

    // Start HTTP server in a separate task so it doesn't block
    tokio::spawn(async move {
        warp::serve(routes)
            .run(([0, 0, 0, 0], PORT))
            .await;
    });

    loop {
        let Some(msg_bytes) = main_from_server.recv().await else { break };
        let result = ServerClientMsg::decode(&msg_bytes);
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                println!("error while decoding server client msg: {e}");
                continue;
            }
        };
        process_server_client_msg(msg, &context_ref).await;
    }
}

fn update_clients_periodically(context_ref: MucoContextRef) {
    let mut frontend_status_generation = 0;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            {
                let context = context_ref.read().await;
                if context.status_generation != frontend_status_generation {
                    context.update_clients().await;
                    context.status.save(SAVE_DATA_PATH).unwrap();
                    frontend_status_generation = context.status_generation;
                }
                if context.unknown_connections.is_empty() {
                    continue;
                }
            }
            context_ref.write().await.request_unknown_device_ids().await;
        }
    });
}

fn with_context(context_ref: MucoContextRef) -> impl Filter<Extract = (MucoContextRef,), Error = Infallible> + Clone {
    warp::any().map(move || context_ref.clone())
}
