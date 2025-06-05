#!/bin/bash

# Test common ports that might be open
TARGET_IP="172.225.176.137"
PORTS=(80 443 8080 8443 9000 9080 3000 5000 8000 22222 33333 44444 8888 9999)

echo "Testing ports on $TARGET_IP..."

for port in "${PORTS[@]}"; do
    echo -n "Port $port: "
    timeout 3 bash -c "</dev/tcp/$TARGET_IP/$port" 2>/dev/null
    if [ $? -eq 0 ]; then
        echo "OPEN"
    else
        echo "CLOSED/FILTERED"
    fi
done