#!/usr/bin/env python3
"""
TCP Doctor V2 - Size-First Verification
Parses TCP greeting with [Size BE][OpCode BE] order
"""
import socket
import binascii
import struct

# Configuration
SERVER_IP = "127.0.0.1"
SERVER_PORT = 5998

def diagnose_tcp_v2():
    print(f"[*] Connecting to TCP {SERVER_IP}:{SERVER_PORT}...")
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(3.0)
        sock.connect((SERVER_IP, SERVER_PORT))
        print("[+] Connection Established! Reading Packet...")

        # Read Header (First 4 bytes: Size + OpCode)
        header = sock.recv(4)
        
        if len(header) < 4:
            print("[-] Server sent less than 4 bytes. Connection closed.")
            return

        # Parse as Big Endian [Size, OpCode]
        size, opcode = struct.unpack('>HH', header)
        
        print(f"\n[+] Header Received: {binascii.hexlify(header).decode('utf-8')}")
        print(f"    Parsed Size:   {size}")
        print(f"    Parsed OpCode: 0x{opcode:04X}")

        # Diagnosis
        if opcode == 0x0001:
            print("\n[+] DIAGNOSIS: SUCCESS (SIZE-FIRST CONFIRMED)")
            print("    The server is correctly sending [Size] then [OpCode 1].")
            print("    The RoF2 client should accept this.")
            
            # Try to read body if size > 0
            if size > 0:
                body = sock.recv(size)
                print(f"\n[+] Body ({size} bytes): {binascii.hexlify(body).decode('utf-8')}")
        elif size == 0x0001:
            print("\n[!] DIAGNOSIS: FAILURE (OPCODE-FIRST DETECTED)")
            print("    The server sent OpCode 1 in the Size field.")
            print("    You are still sending [OpCode] [Size]. Apply the Rust fix!")
        else:
            print("\n[?] DIAGNOSIS: Unclear.")
            print(f"    Neither Size=0x0001 nor OpCode=0x0001 detected.")

    except Exception as e:
        print(f"[-] Error: {e}")
    finally:
        sock.close()

if __name__ == "__main__":
    diagnose_tcp_v2()
