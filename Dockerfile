# Build stage
FROM rust:1.83 as builder

WORKDIR /app
COPY . .

# Build the manager binary
RUN cargo build --release --bin manager

# Runtime stage
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/manager /app/manager

# Copy any required data files
COPY --from=builder /app/server_data.txt /app/server_data.txt

# Expose the HTTP port
EXPOSE 9080

# Run the manager
CMD ["./manager"]