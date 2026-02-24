#!/bin/bash
export SQLX_OFFLINE=1
cargo run -p login-server > server.log 2>&1 &
LOG_PID=$!
sleep 2
python3 integration_test.py > test.log 2>&1
kill $LOG_PID
cat test.log
echo "--- Server Log ---"
cat server.log
