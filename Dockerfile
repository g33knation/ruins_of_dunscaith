# Build Stage
FROM rust:latest as builder

WORKDIR /usr/src/app

# Install build dependencies (e.g. libssl/openssl if needed, though simple UDP might not)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY .sqlx ./.sqlx

# Build all binaries (Offline Mode)
ENV SQLX_OFFLINE=true
# We release mode for performance, though debug is fine for dev. Let's do release.
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /usr/local/bin

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /usr/src/app/target/release/login-server .
COPY --from=builder /usr/src/app/target/release/world-server .
COPY --from=builder /usr/src/app/target/release/zone-server .
COPY .env .

# Environment variables (Defaults, can be overridden by Compose)
ENV RUST_LOG=info

# Expose Ports (Documentation only)
EXPOSE 5999
EXPOSE 9000/udp
EXPOSE 9001/udp

# Entrypoint script or separate containers?
# We will use one image and override command in docker-compose.
