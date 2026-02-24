#!/bin/bash
export SQLX_OFFLINE=1
export DATABASE_URL="postgres://eqemu:eqemupass@127.0.0.1:5432/peq"
export PUBLIC_IP="127.0.0.1"

echo "Starting Login Server..."
cargo run -p login-server > login.log 2>&1 &
LOGIN_PID=$!

echo "Starting World Server..."
cargo run -p world-server > world.log 2>&1 &
WORLD_PID=$!

sleep 5

echo "Running Full Integration Test..."
python3 full_integration_test.py > test_full.log 2>&1
TEST_EXIT=$?

kill $LOGIN_PID
kill $WORLD_PID

cat test_full.log
echo "--- Login Server Log ---"
tail -n 20 login.log
echo "--- World Server Log ---"
tail -n 20 world.log

exit $TEST_EXIT
