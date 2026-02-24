#!/bin/bash
# Kill any existing server instances
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
fuser -k 5998/udp 5998/tcp 5999/udp 5999/tcp 9000/udp 9000/tcp 7000/udp 7000/tcp 2>/dev/null

echo "Starting Servers..."
export RUST_LOG=info
export PUBLIC_IP=192.168.1.24
cargo run --bin login-server > login.log 2>&1 &
cargo run --bin world-server > world.log 2>&1 &
cargo run --bin zone-server > zone.log 2>&1 &
echo "Servers Running."
