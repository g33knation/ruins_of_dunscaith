#!/usr/bin/env python3
"""
Capture and analyze zone handoff packets on port 9000 (world) and 9001 (zone).
Run as: sudo python3 capture_zone_handoff.py
"""

import socket
import struct
import sys
from datetime import datetime

def hexdump(data, prefix=""):
    """Print hex dump of data."""
    for i in range(0, len(data), 16):
        hex_part = ' '.join(f'{b:02x}' for b in data[i:i+16])
        ascii_part = ''.join(chr(b) if 32 <= b < 127 else '.' for b in data[i:i+16])
        print(f"{prefix}{i:04x}: {hex_part:<48} {ascii_part}")

def analyze_zone_server_info(data, offset=0):
    """Analyze OP_ZoneServerInfo packet (OpCode 0x4C44)."""
    print("\n=== OP_ZoneServerInfo Analysis ===")
    
    # Find IP string (null-terminated, 128 bytes)
    ip_end = data.find(b'\x00', offset)
    if ip_end == -1:
        ip_end = offset + 127
    ip = data[offset:ip_end].decode('utf-8', errors='replace')
    print(f"IP Address: '{ip}'")
    
    # Port is at offset 128 from start of payload
    if len(data) >= offset + 130:
        port_bytes = data[offset+128:offset+130]
        port_le = struct.unpack('<H', port_bytes)[0]  # Little Endian
        port_be = struct.unpack('>H', port_bytes)[0]  # Big Endian
        print(f"Port bytes: {port_bytes.hex()}")
        print(f"Port (Little Endian): {port_le}")
        print(f"Port (Big Endian): {port_be}")
        print(f"Expected: 9001 (0x2329)")
    
    hexdump(data[offset:offset+140], "  ")

def main():
    # Monitor both world (9000) and zone (9001)
    print("Zone Handoff Packet Analyzer")
    print("============================")
    print("Monitoring UDP traffic on ports 9000 and 9001...")
    print("Press Ctrl+C to stop.\n")
    
    # Use raw socket to capture all UDP
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_RAW, socket.IPPROTO_UDP)
    except PermissionError:
        print("ERROR: Need root privileges. Run with sudo.")
        sys.exit(1)
    
    sock.bind(('0.0.0.0', 0))
    
    OP_ZONE_SERVER_INFO = 0x4C44
    
    while True:
        try:
            data, addr = sock.recvfrom(65535)
            
            # Parse IP header (20 bytes minimum)
            ip_header = data[:20]
            iph = struct.unpack('!BBHHHBBH4s4s', ip_header)
            src_ip = socket.inet_ntoa(iph[8])
            dst_ip = socket.inet_ntoa(iph[9])
            
            # Parse UDP header (8 bytes)
            ip_header_len = (iph[0] & 0xF) * 4
            udp_header = data[ip_header_len:ip_header_len+8]
            src_port, dst_port, length, checksum = struct.unpack('!HHHH', udp_header)
            
            # Filter for our ports
            if dst_port not in (9000, 9001) and src_port not in (9000, 9001):
                continue
            
            payload = data[ip_header_len+8:]
            
            ts = datetime.now().strftime('%H:%M:%S.%f')[:-3]
            print(f"\n[{ts}] {src_ip}:{src_port} -> {dst_ip}:{dst_port} (len={len(payload)})")
            
            hexdump(payload[:64], "  ")
            
            # Look for OP_ZoneServerInfo in payload (it would be embedded in EQStream)
            if len(payload) > 10:
                # Check for combined packet structure
                for i in range(len(payload) - 2):
                    opcode = struct.unpack('<H', payload[i:i+2])[0]
                    if opcode == OP_ZONE_SERVER_INFO and i + 132 < len(payload):
                        print(f"\n>>> Found OP_ZoneServerInfo at offset {i} <<<")
                        analyze_zone_server_info(payload, i+2)
                        break
            
        except KeyboardInterrupt:
            print("\nStopping...")
            break

if __name__ == '__main__':
    main()
