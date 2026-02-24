#!/usr/bin/env python3
"""Simple UDP listener to check if ANY traffic arrives on port 9001"""
import socket
import sys
from datetime import datetime

PORT = 9001

print(f"Listening on UDP port {PORT}...")
print("Waiting for any incoming packets...")
print("Press Ctrl+C to stop.\n")

# Use SO_REUSEPORT so we can run alongside the zone server
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
except AttributeError:
    pass  # SO_REUSEPORT not available on all systems
sock.settimeout(60.0)

# This will fail if zone server is already bound - which is fine, 
# we just need it to test BEFORE starting zone server
try:
    sock.bind(('0.0.0.0', PORT))
except OSError as e:
    print(f"Could not bind to port {PORT}: {e}")
    print("(This is expected if zone-server is already running)")
    sock.close()
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.settimeout(60.0)
    # Try to connect and recv - this won't work for UDP listen but let's bail
    print("Please stop zone-server first and run this script alone.")
    sys.exit(1)

try:
    while True:
        try:
            data, addr = sock.recvfrom(65535)
            ts = datetime.now().strftime('%H:%M:%S.%f')[:-3]
            print(f"[{ts}] RECEIVED {len(data)} bytes from {addr}")
            print(f"  Hex: {data[:64].hex()}")
            print()
        except socket.timeout:
            print("Timeout - no packets received in 60 seconds")
            break
except KeyboardInterrupt:
    print("\nStopping...")
finally:
    sock.close()
