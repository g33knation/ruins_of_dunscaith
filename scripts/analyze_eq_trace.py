#!/usr/bin/env python3
"""
Parse EQEmu packet trace from tcpdump PCAP or hex dump.
This script analyzes UDP traffic to identify EQStream packets and OpCodes.

Usage: python3 analyze_eq_trace.py <input_file>
"""

import sys
import struct
from collections import defaultdict

# EQStream Transport OpCodes
TRANSPORT_OPS = {
    0x0001: "SessionRequest",
    0x0002: "SessionResponse",
    0x0003: "Combined",
    0x0005: "SessionDisconnect",
    0x0009: "Application",
    0x000d: "Fragment",
    0x0011: "Ack",
    0x0015: "KeepAlive",
    0x001d: "OutOfOrder",
}

# Known RoF2 Application OpCodes
APP_OPS = {
    0x7a09: "OP_SendLoginInfo",
    0x7499: "OP_ApproveWorld",
    0x7ceb: "OP_LogServer",
    0x00d2: "OP_SendCharInfo",
    0x590d: "OP_ExpansionInfo",
    0x507a: "OP_GuildsList",
    0x57c3: "OP_EnterWorld",
    0x7c94: "OP_PostEnterWorld",
    0x5475: "OP_SendMaxCharacters",
    0x7acc: "OP_SendMembership",
    0x057b: "OP_SendMembershipDetails",
    0x6bbf: "OP_CharacterCreate",
    0x6773: "OP_CharacterCreateRequest",
    0x1808: "OP_DeleteCharacter",
    0x56a2: "OP_ApproveName",
    0x0c22: "OP_MOTD",
    0x3234: "OP_SendZonePoints",
    0x4254: "OP_TributeInfo",
    0x5070: "OP_TimeOfDay",
    0x661e: "OP_Weather",
    0x4c44: "OP_ZoneServerInfo",
    0x4493: "OP_WorldComplete",
    0x1100: "OP_ClientReady (RoF2)",
    0x1500: "OP_Unknown1500 (RoF2)",
    0x0f13: "OP_World_Client_CRC1",
    0x4b8d: "OP_World_Client_CRC2",
    0x298d: "OP_World_Client_CRC3",
    # Zone Server OpCodes
    0x1900: "OP_ZoneEntry",
    0x5089: "OP_ZoneEntry (Response)",
    0x6506: "OP_PlayerProfile",
    0x7DFC: "OP_ClientUpdate",
    0x345D: "OP_ClientReady (Zone)",
    0x35FA: "OP_ReqClientSpawn",
    0x5f8e: "OP_SendExpZonein",
    0x43c8: "OP_SendAAStats",
    0x729b: "OP_SendTributes",
    0x1eec: "OP_LevelUpdate",
    0x2a79: "OP_Stamina",
    0x5ca6: "OP_CharInventory",
    0x69a4: "OP_ZonePoints",
}

def parse_hex_line(line):
    """Parse a hex dump line (tcpdump -xx format)."""
    parts = line.split()
    if len(parts) < 2:
        return []
    
    # Skip offset (first part like "0x0000:")
    if parts[0].startswith("0x") or parts[0].endswith(":"):
        parts = parts[1:]
    
    hex_bytes = []
    for p in parts:
        if len(p) == 2 or len(p) == 4:
            try:
                if len(p) == 4:
                    # Two bytes together
                    hex_bytes.append(int(p[:2], 16))
                    hex_bytes.append(int(p[2:], 16))
                else:
                    hex_bytes.append(int(p, 16))
            except ValueError:
                break
    return hex_bytes

def analyze_packet(data, direction, src_port, dst_port, pkt_num):
    """Analyze a single EQStream packet."""
    if len(data) < 2:
        return
    
    port_info = f"{src_port}->{dst_port}"
    
    # Transport OpCode (first 2 bytes, big endian)
    transport_op = struct.unpack(">H", bytes(data[0:2]))[0]
    transport_name = TRANSPORT_OPS.get(transport_op, f"Unknown({transport_op:#06x})")
    
    print(f"\n[PKT {pkt_num}] {direction} {port_info} ({len(data)} bytes)")
    print(f"  Transport: {transport_name} ({transport_op:#06x})")
    
    # Handle specific transport types
    if transport_op == 0x0009:  # Application
        if len(data) >= 4:
            app_op = struct.unpack("<H", bytes(data[2:4]))[0]
            app_name = APP_OPS.get(app_op, f"Unknown")
            print(f"  App OpCode: {app_name} ({app_op:#06x})")
            print(f"  Payload: {len(data) - 4} bytes")
            
            # Special analysis for ZoneServerInfo
            if app_op == 0x4c44:
                analyze_zone_server_info(data[4:])
                
    elif transport_op == 0x0003:  # Combined
        print(f"  [Combined Packet - Contains multiple sub-packets]")
        offset = 2
        sub_pkt = 0
        while offset < len(data):
            if len(data) - offset < 1:
                break
            sub_len = data[offset]
            offset += 1
            if sub_len == 0 or offset + sub_len > len(data):
                break
            sub_data = data[offset:offset+sub_len]
            if len(sub_data) >= 2:
                sub_op = struct.unpack("<H", bytes(sub_data[0:2]))[0]
                app_name = APP_OPS.get(sub_op, "Unknown")
                print(f"    Sub[{sub_pkt}]: {app_name} ({sub_op:#06x}), {sub_len} bytes")
                
                if sub_op == 0x4c44:
                    analyze_zone_server_info(sub_data[2:])
            offset += sub_len
            sub_pkt += 1
            
    elif transport_op == 0x000d:  # Fragment
        if len(data) >= 4:
            seq = struct.unpack(">H", bytes(data[2:4]))[0]
            print(f"  Fragment Seq: {seq}")
            print(f"  Fragment Data: {len(data) - 4} bytes")

def analyze_zone_server_info(payload):
    """Special analysis for OP_ZoneServerInfo packet."""
    if len(payload) < 130:
        print(f"    [ZoneServerInfo too short: {len(payload)} bytes]")
        return
        
    # IP: First 128 bytes, null-terminated
    ip_end = payload.find(0) if 0 in payload[:128] else 127
    ip = bytes(payload[:ip_end]).decode('ascii', errors='replace')
    
    # Port: Bytes 128-129, little endian
    port = struct.unpack("<H", bytes(payload[128:130]))[0]
    
    print(f"    [ZoneServerInfo] IP: '{ip}', Port: {port}")

def parse_tcpdump_file(filename):
    """Parse a tcpdump -xx output file."""
    print(f"Parsing tcpdump output: {filename}")
    print("=" * 60)
    
    packets = []
    current_packet = []
    current_meta = {}
    pkt_num = 0
    
    with open(filename, 'r') as f:
        for line in f:
            line = line.strip()
            
            # Packet header line (timestamp and metadata)
            if "UDP" in line or ">" in line:
                # Save previous packet
                if current_packet and current_meta:
                    pkt_num += 1
                    analyze_packet(current_packet, 
                                 current_meta.get('direction', '?'),
                                 current_meta.get('src_port', 0),
                                 current_meta.get('dst_port', 0),
                                 pkt_num)
                current_packet = []
                
                # Parse metadata
                parts = line.split()
                for i, p in enumerate(parts):
                    if '>' in p and i > 0:
                        src = parts[i-1].split('.')
                        dst = parts[i+1].split('.')
                        if len(src) >= 2 and len(dst) >= 2:
                            try:
                                current_meta['src_port'] = int(src[-1].rstrip(':'))
                                current_meta['dst_port'] = int(dst[-1].rstrip(':'))
                                current_meta['direction'] = 'TX' if current_meta['src_port'] in [9000, 7000] else 'RX'
                            except:
                                pass
                        break
                        
            # Hex data line
            elif line.startswith("0x") or (len(line) > 0 and line[0:4].replace(' ','').isalnum()):
                hex_bytes = parse_hex_line(line)
                # Skip IP/UDP headers (usually first 28 bytes for IPv4)
                if len(current_packet) == 0 and len(hex_bytes) >= 28:
                    # Check if this looks like an IP header
                    if hex_bytes[0] >> 4 == 4:  # IPv4
                        # Skip IP header (20 bytes) + UDP header (8 bytes)
                        hex_bytes = hex_bytes[28:]
                current_packet.extend(hex_bytes)
    
    # Final packet
    if current_packet and current_meta:
        pkt_num += 1
        analyze_packet(current_packet, 
                     current_meta.get('direction', '?'),
                     current_meta.get('src_port', 0),
                     current_meta.get('dst_port', 0),
                     pkt_num)
    
    print(f"\n{'='*60}")
    print(f"Total packets analyzed: {pkt_num}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 analyze_eq_trace.py <tcpdump_file>")
        print("\nGenerate trace with: tcpdump -i any -nn -xx udp port 9000 or udp port 7000 > trace.txt")
        sys.exit(1)
    
    parse_tcpdump_file(sys.argv[1])
