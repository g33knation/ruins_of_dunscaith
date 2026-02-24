#!/bin/bash
export SQLX_OFFLINE=1
cargo run -p login-server &
LOG_PID=$!
sleep 5 # Wait for compilation and startup
python3 integration_test.py
kill $LOG_PID
