use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;
use local_ip_address::local_ip;
use warp::Filter;

pub struct ConnectionConfig {
    pub http_port: u16,
    pub https_port: u16,
    pub has_certificates: bool,
}

impl ConnectionConfig {
    pub fn new() -> Self {
        Self {
            http_port: 8080,
            https_port: 9443,
            has_certificates: std::path::Path::new("cert.pem").exists() 
                && std::path::Path::new("key.pem").exists(),
        }
    }
    
    pub fn ensure_certificates() -> Self {
        let has_certs = std::path::Path::new("cert.pem").exists() 
            && std::path::Path::new("key.pem").exists();
            
        if !has_certs {
            println!("🔐 No SSL certificates found, generating dynamic self-signed certificate...");
            if let Err(e) = generate_dynamic_certificate() {
                println!("❌ Failed to generate certificate: {}", e);
                println!("   HTTPS will be disabled");
            } else {
                println!("✅ Dynamic certificate generated successfully");
            }
        }
        
        Self {
            http_port: 8080,
            https_port: 9443,
            has_certificates: std::path::Path::new("cert.pem").exists() 
                && std::path::Path::new("key.pem").exists(),
        }
    }
}

fn generate_dynamic_certificate() -> Result<(), Box<dyn std::error::Error>> {
    // Try to detect the server's external IP
    let server_ip = detect_server_ip()?;
    
    println!("🌐 Generating certificate for IP: {}", server_ip);
    
    // Create dynamic certificate configuration
    let cert_config = format!(r#"[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = State
L = City  
O = MUCO Server
CN = {}

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = {}
"#, server_ip, server_ip);
    
    // Write temporary config file
    std::fs::write("cert_config_temp.conf", cert_config)?;
    
    // Generate certificate using OpenSSL
    let output = Command::new("openssl")
        .args([
            "req", "-x509", "-newkey", "rsa:4096", 
            "-keyout", "key.pem", 
            "-out", "cert.pem", 
            "-days", "365", 
            "-nodes", 
            "-config", "cert_config_temp.conf"
        ])
        .output()?;
    
    // Clean up temporary config
    let _ = std::fs::remove_file("cert_config_temp.conf");
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("OpenSSL failed: {}", error).into());
    }
    
    println!("📝 Certificate valid for:");
    println!("   • localhost");
    println!("   • 127.0.0.1");
    println!("   • {}", server_ip);
    println!("🔗 HTTPS endpoint: https://{}:9443", server_ip);
    println!("🔌 WebSocket endpoint: wss://{}:9443/ws", server_ip);
    
    Ok(())
}

fn detect_server_ip() -> Result<String, Box<dyn std::error::Error>> {
    // Try multiple methods to detect the server's IP
    
    // Method 1: Try external IP detection services (IPv4 preferred)
    if let Ok(output) = Command::new("curl")
        .args(["-4", "-s", "--connect-timeout", "5", "ifconfig.me"])
        .output() 
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && ip.parse::<std::net::Ipv4Addr>().is_ok() {
            return Ok(ip);
        }
    }
    
    // Method 2: Try alternative service (IPv4 preferred)
    if let Ok(output) = Command::new("curl")
        .args(["-4", "-s", "--connect-timeout", "5", "ipinfo.io/ip"])
        .output() 
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && ip.parse::<std::net::Ipv4Addr>().is_ok() {
            return Ok(ip);
        }
    }
    
    // Method 3: Try local network interface detection
    if let Ok(output) = Command::new("sh")
        .args(["-c", "ip route get 1 2>/dev/null | awk '{print $NF;exit}' 2>/dev/null || echo ''"])
        .output() 
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && is_valid_ip(&ip) {
            return Ok(ip);
        }
    }
    
    // Fallback: Use localhost
    println!("⚠️  Could not detect external IP, using localhost");
    Ok("127.0.0.1".to_string())
}

fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok() || ip.parse::<std::net::Ipv6Addr>().is_ok()
}

pub async fn get_public_ip() -> Option<String> {
    // Try multiple services in case one is down
    let services = [
        "https://ipv4.icanhazip.com",
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
    ];
    
    for service in &services {
        if let Ok(response) = reqwest::get(*service).await {
            if let Ok(ip) = response.text().await {
                let ip = ip.trim();
                if !ip.is_empty() && ip.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

pub fn print_startup_banner() {
    println!("\n🚀 MUCO Manager Starting");
    println!("================================");
}

pub async fn detect_and_print_public_ip() -> String {
    print!("🌐 Detecting public IP... ");
    if let Some(ip) = get_public_ip().await {
        println!("✅ {}", ip);
        ip
    } else {
        println!("⚠️  Failed to detect");
        "YOUR_PUBLIC_IP".to_string()
    }
}

pub fn print_connection_info(config: &ConnectionConfig, local_ip: IpAddr, public_ip: &str) {
    println!("\n📡 Available Connections:");
    println!("Local Network:");
    println!("  • HTTP:  http://127.0.0.1:{}  (local machine)", config.http_port);
    println!("  • HTTP:  http://{}:{}  (local network)", local_ip, config.http_port);
    
    if config.has_certificates {
        println!("  • HTTPS: https://127.0.0.1:{}  (self-signed cert)", config.https_port);
        println!("  • HTTPS: https://{}:{}  (self-signed cert)", local_ip, config.https_port);
    }
    
    println!("\nExternal Access (requires port forwarding):");
    println!("  • HTTP:  http://{}:{}  (if port {} forwarded)", public_ip, config.http_port, config.http_port);
    if config.has_certificates {
        println!("  • HTTPS: https://{}:{}  (if port {} forwarded)", public_ip, config.https_port, config.https_port);
    }
    
    println!("\n💡 Troubleshooting: See CONNECTION-GUIDE.md for detailed help");
    println!("================================\n");
}

pub async fn start_servers<F>(routes: F, config: ConnectionConfig) 
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: warp::Reply,
{
    // Start HTTP server (always available)
    let routes_http = routes.clone();
    let http_port = config.http_port;
    let http_addr = SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), http_port);
    tokio::spawn(async move {
        println!("✅ HTTP Server running on port {}", http_port);
        warp::serve(routes_http).run(http_addr).await;
    });
    
    // Start HTTPS server (if certificates available)
    if config.has_certificates {
        let routes_https = routes.clone();
        let https_port = config.https_port;
        let https_addr = SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), https_port);
        tokio::spawn(async move {
            println!("✅ HTTPS Server running on port {}", https_port);
            warp::serve(routes_https)
                .tls()
                .cert_path("cert.pem")
                .key_path("key.pem")
                .run(https_addr)
                .await;
        });
    } else {
        println!("⚠️  HTTPS Server disabled (no certificates found)");
        println!("   Place cert.pem and key.pem in current directory to enable HTTPS");
    }
}

pub async fn setup_and_start_servers<F>(routes: F) 
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: warp::Reply,
{
    print_startup_banner();
    let public_ip = detect_and_print_public_ip().await;
    
    // Use the new ensure_certificates method to auto-generate if needed
    let config = ConnectionConfig::ensure_certificates();
    let local_ip = local_ip().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
    
    // Show SSL setup guidance if no certificates found (after generation attempt)
    if !config.has_certificates {
        print_ssl_setup_guidance();
    }
    
    let config_copy = ConnectionConfig {
        http_port: config.http_port,
        https_port: config.https_port,
        has_certificates: config.has_certificates,
    };
    start_servers(routes, config).await;
    print_connection_info(&config_copy, local_ip, &public_ip);
}

fn print_ssl_setup_guidance() {
    println!("\n🔐 SSL Certificate Setup");
    println!("================================");
    println!("No SSL certificates found. For HTTPS support:");
    println!("");
    println!("📋 Option 1: Generate self-signed certificates (for testing)");
    println!("  openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes");
    println!("");
    println!("📋 Option 2: Use Let's Encrypt (for production)");
    println!("  1. Install certbot: apt install certbot");
    println!("  2. Get certificate: certbot certonly --standalone -d yourdomain.com");
    println!("  3. Copy files: cp /etc/letsencrypt/live/yourdomain.com/{{fullchain,privkey}}.pem .");
    println!("  4. Rename: mv fullchain.pem cert.pem && mv privkey.pem key.pem");
    println!("");
    println!("📋 Option 3: Manual certificates");
    println!("  Place cert.pem and key.pem in current directory");
    println!("");
    println!("⚠️  Note: HTTPS frontends cannot connect to HTTP backends");
    println!("   due to browser mixed content policy.");
    println!("================================");
}