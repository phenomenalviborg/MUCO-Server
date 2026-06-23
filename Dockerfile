# Build stage — compile both service binaries in one pass.
FROM rust:1.83 as builder

WORKDIR /app
COPY . .

# manager = HTTP dashboard API (:9080); server = relay for VR clients (:1302).
RUN cargo build --release --bin manager --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Both binaries ship in one image; each compose service picks which to run via
# its `command`. Single build, single image.
COPY --from=builder /app/target/release/manager /app/manager
COPY --from=builder /app/target/release/server /app/server

# Copy any required data files
COPY --from=builder /app/server_data.txt /app/server_data.txt

# manager HTTP API, relay server
EXPOSE 9080
EXPOSE 1302

# Default command; overridden per service in docker-compose.yml.
CMD ["./manager"]
