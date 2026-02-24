import socket
import struct
import time
import sys

# --- THE CONFIGURATIONS TO BRUTE FORCE ---
CONFIGS = [
    # 1. The "Green Light" (Echo OpCode + Open Status + Padding) - Most Likely
    {"opcode": 0x0002, "padding": True,  "status": -1, "load": 0,  "name": "Echo OpCode (0x02) + Open Status (-1)"},
    
    # 2. Standard + Open Status
    {"opcode": 0x1000, "padding": True,  "status": -1, "load": 0,  "name": "Standard OpCode (0x1000) + Open Status (-1)"},
    
    # 3. Echo OpCode + Standard Status
    {"opcode": 0x0002, "padding": True,  "status": 1,  "load": 10, "name": "Echo OpCode (0x02) + Status 1"},
    
    # 4. No Padding Variants
    {"opcode": 0x0002, "padding": False, "status": -1, "load": 0,  "name": "Echo OpCode (0x02) + No Padding"},
    
    # 5. The "Legacy" packet (OpCode 0x0004 used by some Titans clients)
    {"opcode": 0x0004, "padding": True,  "status": -1, "load": 0,  "name": "Legacy OpCode (0x04)"},
]

def start_auto_fuzzer():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", 5998))
    
    current_idx = 0
    consecutive_failures = 0
    last_req_time = 0
    
    print(f"[*] AUTO-FUZZER ENGAGED on Port 5998")
    print(f"[*] Launch your RoF2 Client now. I will cycle configs until it connects.")
    print(f"[*] Testing Config #{current_idx+1}: {CONFIGS[current_idx]['name']}")

    while True:
        try:
            data, addr = sock.recvfrom(2048)
            now = time.time()
            
            # --- LOOP DETECTION ---
            # If requests come faster than 1.5 seconds, the client rejected us.
            if now - last_req_time < 1.5:
                consecutive_failures += 1
            else:
                # If there was a long pause, the previous config MIGHT have worked, 
                # or the client just started. We don't switch yet.
                consecutive_failures = 0
            
            last_req_time = now

            # If we failed 3 times in a row, SWITCH CONFIG
            if consecutive_failures >= 3:
                current_idx = (current_idx + 1) % len(CONFIGS)
                consecutive_failures = 0
                print(f"\n[!] REJECTED. Switching to Config #{current_idx+1}:")
                print(f"    >>> {CONFIGS[current_idx]['name']} <<<")

            # --- PARSE REQUEST ---
            if len(data) < 20: continue
            
            # Extract Session ID (Bytes 0-4)
            session_id = data[0:4]
            
            # Extract Magic (Scan for "Everquest" or "EverQuest")
            # We scan the whole packet to be safe
            try:
                magic_bytes = b'EverQuest'
                if b'Everquest' in data: magic_bytes = b'Everquest'
                elif b'EverQuest' in data: magic_bytes = b'EverQuest'
            except:
                magic_bytes = b'EverQuest'

            # --- BUILD RESPONSE ---
            cfg = CONFIGS[current_idx]
            response = bytearray()
            
            # 1. Header
            response.extend(session_id)
            if cfg['padding']:
                # RoF2 expects 14 byte header? 4 (Session) + 8 (Pad) + 2 (OpCode)
                response.extend(b'\x00' * 8)
                response.extend(struct.pack('<H', cfg['opcode']))
            else:
                # Simple header: 4 (Session) + 2 (OpCode)
                response.extend(struct.pack('<H', cfg['opcode']))

            # 2. Magic (Echo)
            response.extend(magic_bytes + b'\x00')
            
            # 3. IP (127.0.0.1 Big Endian)
            response.extend(b'\x7F\x00\x00\x01')
            
            # 4. Port (5998 Big Endian)
            response.extend(b'\x17\x6E')
            
            # 5. Status & Load (Little Endian)
            # Status -1 (0xFFFF) is standard for "Open"
            response.extend(struct.pack('<h', cfg['status'])) 
            response.extend(struct.pack('<I', cfg['load']))
            
            # 6. Strings
            response.extend(b'RustLogin\x00')
            response.extend(b'AutoFuzzer\x00')
            
            sock.sendto(response, addr)
            # print(f".", end="", flush=True) # Minimal output to reduce spam

        except KeyboardInterrupt:
            print("\n[*] Fuzzer Stopped.")
            break
        except Exception as e:
            print(f"[!] Error: {e}")

if __name__ == "__main__":
    start_auto_fuzzer()
