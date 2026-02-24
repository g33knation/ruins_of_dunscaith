#!/bin/bash
set -e

echo "Setting up Postgres Database..."

# Fix Collation Version Mismatch Warning
sudo -u postgres psql -c "ALTER DATABASE postgres REFRESH COLLATION VERSION;" || true
sudo -u postgres psql -c "ALTER DATABASE template1 REFRESH COLLATION VERSION;" || true

# Create User 'eqemu' if not exists
sudo -u postgres psql -c "DO \$\$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'eqemu') THEN CREATE ROLE eqemu LOGIN PASSWORD 'eqemupass'; END IF; END \$\$;"

# Create Database 'peq' if not exists
if ! sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw peq; then
    echo "Creating database 'peq'..."
    sudo -u postgres createdb -O eqemu peq
else
    echo "Database 'peq' already exists."
fi

# Grant Privileges
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE peq TO eqemu;"

echo "Applying Migrations..."
export PGPASSWORD=eqemupass

# Apply 01
psql -h 127.0.0.1 -U eqemu -d peq -f migrations/20240121000001_create_tables.sql

# Apply 02
psql -h 127.0.0.1 -U eqemu -d peq -f migrations/20240121000002_world_redirection.sql

# Apply 03
psql -h 127.0.0.1 -U eqemu -d peq -f migrations/20240121000003_full_peq_schema.sql

echo "Database Setup Complete!"
