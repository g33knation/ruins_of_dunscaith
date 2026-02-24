#!/bin/bash
# Kill existing instances
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
pkill -f "target/debug/login-server"
pkill -f "target/debug/world-server"
pkill -f "target/debug/zone-server"

# Start Servers
echo "Starting Login Server..."
export RUST_LOG=debug
export PUBLIC_IP=192.168.1.24
nohup cargo run -q -p login-server > login.log 2>&1 &

echo "Starting World Server..."
nohup cargo run -q -p world-server > world.log 2>&1 &

echo "Starting Zone Server..."
nohup cargo run -q -p zone-server > zone.log 2>&1 &

echo "Servers spinned up! Check login.log, world.log, and zone.log."
