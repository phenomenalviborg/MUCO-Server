#!/usr/bin/env bash
# provision-tls.sh — set up the MUCO backend TLS proxy on this box. Idempotent.
#
# The dashboard is served over HTTPS, so browsers block ws:// to the plain-HTTP
# manager (:9080). Caddy terminates TLS on :9443 with a real Lets Encrypt cert
# for the sslip.io hostname derived from this box public IP, and reverse-proxies
# to the local manager. The frontend toSecureHost() maps <ip>:9080 ->
# <dashed-ip>.sslip.io:9443 automatically, so NO frontend change is needed.
#
# Run as root on each new backend/VR box:  bash provision-tls.sh
set -euo pipefail

IP=$(curl -fsS https://api.ipify.org)
HOST="${IP//./-}.sslip.io"

cat > /etc/caddy/Caddyfile <<CADDY
# MUCO backend TLS proxy — managed by provision-tls.sh. Do not hand-edit.
# Real Lets Encrypt cert for the sslip.io host; reverse-proxy to local manager.
${HOST}:9443 {
	reverse_proxy 127.0.0.1:9080
}
CADDY

caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
systemctl reload caddy
echo "Provisioned ${HOST}:9443 -> 127.0.0.1:9080"
