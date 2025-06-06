#!/bin/bash

# Dynamic Self-Signed Certificate Generator for MUCO Server
# Detects server IP and generates appropriate certificate

set -e

echo "🔍 Detecting server IP address..."

# Try multiple methods to get the external IP
SERVER_IP=""

# Method 1: Try ifconfig.me (IPv4 only)
if command -v curl >/dev/null 2>&1; then
    SERVER_IP=$(curl -4 -s --connect-timeout 5 ifconfig.me 2>/dev/null || echo "")
fi

# Method 2: Try ipinfo.io if first method failed (IPv4 only)
if [ -z "$SERVER_IP" ] && command -v curl >/dev/null 2>&1; then
    SERVER_IP=$(curl -4 -s --connect-timeout 5 ipinfo.io/ip 2>/dev/null || echo "")
fi

# Method 3: Try local network interface (fallback)
if [ -z "$SERVER_IP" ]; then
    if command -v ip >/dev/null 2>&1; then
        SERVER_IP=$(ip route get 1 2>/dev/null | awk '{print $NF;exit}' || echo "")
    elif command -v route >/dev/null 2>&1; then
        SERVER_IP=$(route get default 2>/dev/null | grep interface | awk '{print $2}' | xargs ifconfig 2>/dev/null | grep 'inet ' | grep -v 127.0.0.1 | awk '{print $2}' | head -1 || echo "")
    fi
fi

# Method 4: Final fallback to localhost
if [ -z "$SERVER_IP" ]; then
    SERVER_IP="127.0.0.1"
    echo "⚠️  Could not detect external IP, using localhost"
else
    echo "✅ Detected server IP: $SERVER_IP"
fi

# Create certificate configuration
cat > cert_config_dynamic.conf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = State
L = City  
O = MUCO Server
CN = $SERVER_IP

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = $SERVER_IP
EOF

echo "📝 Generated certificate configuration for IP: $SERVER_IP"

# Generate the certificate and key
echo "🔐 Generating self-signed certificate..."
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -config cert_config_dynamic.conf

if [ -f "cert.pem" ] && [ -f "key.pem" ]; then
    echo "✅ Certificate generated successfully!"
    echo "   Certificate: cert.pem"
    echo "   Private Key: key.pem"
    echo "   Valid for IP: $SERVER_IP"
    echo ""
    echo "🌐 HTTPS server will be available at: https://$SERVER_IP:9443"
    echo "🔌 WebSocket endpoint will be: wss://$SERVER_IP:9443/ws"
    echo ""
    echo "⚠️  Users will need to accept the security warning for this self-signed certificate."
else
    echo "❌ Failed to generate certificate!"
    exit 1
fi

# Clean up temporary config
rm -f cert_config_dynamic.conf

echo "🚀 Ready to start MUCO manager with HTTPS support!"