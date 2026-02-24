import socket
import struct
import sys

# Configuration - TWEAK THESE VALUES TO FIX THE LOOP
LISTEN_IP = "0.0.0.0"
LISTEN_PORT = 5998

# --- VARIABLES TO FUZZ ---
# 1. OpCode: Try 0x1000 (Current), 0x0002 (Echo Request), 0x2000
RESPONSE_OPCODE = 0x0002 

# 2. Header Padding: The client sends 14 bytes before "Everquest". 
#    We are currently sending 4 bytes (SessionID) + 2 bytes (OpCode) = 6 bytes.
#    Try True to pad this to 14 bytes to match the client's structure.
USE_14_BYTE_PADDING = True 

# 3. Status: Try 1 (Up), 2 (Locked?), -1 (0xFFFF), 0
STATUS = -1 

# 4. Load: Try 0, 10, 100, -1
LOAD = 0 
# -------------------------

def start_server():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((LISTEN_IP, LISTEN_PORT))
    print(f"[*] Fuzzer Login Server listening on {LISTEN_IP}:{LISTEN_PORT}")
    print(f"[*] Configuration: OpCode=0x{RESPONSE_OPCODE:04X}, Padding={USE_14_BYTE_PADDING}, Status={STATUS}")

    while True:
        data, addr = sock.recvfrom(1024)
        
        # 1. Parse Request
        if len(data) < 20: continue
        
        # RoF2 Request Structure (from your tcpdump):
        # [Header: 14 bytes] [Magic: "Everquest\0"]
        
        # Extract SessionID (First 4 bytes of header)
        session_id = data[0:4]
        
        # Extract Magic Word (Skip 14 bytes)
        try:
            magic_part = data[14:]
            null_idx = magic_part.find(b'\x00')
            if null_idx == -1: continue
            magic_bytes = magic_part[:null_idx]
            magic_str = magic_bytes.decode('utf-8', errors='ignore')
        except:
            continue

        if "verquest" in magic_str or "verQuest" in magic_str: # Loose match
            print(f"[+] Client Request from {addr} | Magic: '{magic_str}'")
            
            # 2. Build Response
            response = bytearray()
            
            # --- SESSION ID (4 Bytes) ---
            response.extend(session_id)
            
            # --- PADDING/OPCODE LOGIC ---
            if USE_14_BYTE_PADDING:
                # If Client expects 14 byte header, we need 8 bytes padding + 2 byte opcode?
                # Or just echo the full header? 
                # Let's try: Session(4) + Padding(8) + OpCode(2) = 14 Bytes
                response.extend(b'\x00' * 8)
                response.extend(struct.pack('<H', RESPONSE_OPCODE))
            else:
                # Current Rust Logic: Session(4) + OpCode(2) = 6 Bytes
                response.extend(struct.pack('<H', RESPONSE_OPCODE))

            # --- MAGIC (Echoed Exactly) ---
            response.extend(magic_bytes + b'\x00')
            
            # --- IP (127.0.0.1 Big Endian) ---
            response.extend(b'\x7F\x00\x00\x01')
            
            # --- PORT (5998 Big Endian) ---
            response.extend(b'\x17\x6E')
            
            # --- STATUS (Little Endian) ---
            response.extend(struct.pack('<H', STATUS))
            
            # --- LOAD (Little Endian) ---
            response.extend(struct.pack('<I', LOAD))
            
            # --- STRINGS ---
            response.extend(b'RustLogin\x00')
            response.extend(b'Rust EQEmu Fuzzer\x00')
            
            sock.sendto(response, addr)
            print(f"    -> Sent Response ({len(response)} bytes). Waiting for TCP...")

if __name__ == "__main__":
    try:
        start_server()
    except KeyboardInterrupt:
        print("\nStopping Fuzzer.")
