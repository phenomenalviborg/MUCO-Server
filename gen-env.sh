#!/usr/bin/env bash
# Write .env with SSLIP_HOST derived from this box's public IP, used by the
# Caddy service for its automatic Let's Encrypt cert. Idempotent. Run before
# `docker compose up`:  ./gen-env.sh && docker compose up -d --build
set -euo pipefail

IP=$(curl -fsS https://api.ipify.org)
HOST="${IP//./-}.sslip.io"
echo "SSLIP_HOST=${HOST}" > .env
echo "Wrote .env: SSLIP_HOST=${HOST}"
