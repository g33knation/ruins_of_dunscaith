import socket
import struct
import sys
import time

# --- CONFIGURATION ---
UDP_IP = "0.0.0.0"
UDP_PORT = 5998
TCP_IP = "0.0.0.0"
TCP_PORT = 5998

# --- PACKET SETTINGS ---
# Port 5998 in Little Endian
PORT_BYTES = b'\x6e\x17' 
# Status 1 (Up) in Little Endian
STATUS_BYTES = b'\x01\x00'

def start_server():
    print(f"--- MOCK LOGIN SERVER STARTING ---")
    
    # 1. Setup UDP
    sock_udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock_udp.bind((UDP_IP, UDP_PORT))
    sock_udp.setblocking(False)
    print(f"[UDP] Listening on {UDP_IP}:{UDP_PORT}")

    # 2. Setup TCP
    sock_tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock_tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock_tcp.bind((TCP_IP, TCP_PORT))
    sock_tcp.listen(1)
    sock_tcp.setblocking(False)
    print(f"[TCP] Listening on {TCP_IP}:{TCP_PORT}")

    print("--- WAITING FOR CLIENT ---")

    while True:
        # --- UDP HANDLER ---
        try:
            data, addr = sock_udp.recvfrom(1024)
            # Case-insensitive check for EverQuest
            if b"everquest" in data.lower():
                print(f"[UDP] Client ping from {addr}")
                
                # Construct Response
                response = bytearray()
                # Locate end of 'EverQuest' to echo the header correctly
                magic_idx = data.lower().find(b"everquest")
                null_idx = data.find(b'\x00', magic_idx)
                
                response.extend(data[:null_idx+1]) # Echo raw header + magic
                response.extend(b'\x7F\x00\x00\x01') # IP 127.0.0.1
                response.extend(PORT_BYTES)          # Port 5998 (LE)
                response.extend(STATUS_BYTES)        # Status 1 (LE)
                response.extend(b'\x00\x00\x00\x00') # Load
                response.extend(b'MockLogin\x00')
                response.extend(b'Python Server\x00')
                
                sock_udp.sendto(response, addr)
        except BlockingIOError:
            pass

        # --- TCP HANDLER ---
        try:
            conn, tcp_addr = sock_tcp.accept()
            print(f"\n[TCP] !!! CONNECTION ACCEPTED from {tcp_addr} !!!")
            conn.setblocking(True)
            
            # 1. Send OP_SessionReady (0x0001)
            # Size = 14 (BE), OpCode = 1 (LE)
            packet = bytearray()
            packet.extend((14).to_bytes(2, 'big'))      # Size (BE)
            packet.extend((1).to_bytes(2, 'little'))    # OpCode (LE) <-- CRITICAL
            packet.extend(b'\x00' * 12)                 # Body
            conn.sendall(packet)
            print("[TCP] Sent OP_SessionReady")

            # 2. Wait for OP_Login (0x0002)
            data = conn.recv(1024)
            if data:
                # Parse Packet
                # First 2 bytes = Size (BE)
                # Next 2 bytes = OpCode (LE)
                size = int.from_bytes(data[0:2], 'big')
                opcode = int.from_bytes(data[2:4], 'little')
                
                print(f"[TCP] Received Packet: Size={size}, OpCode=0x{opcode:04X}")
                
                if opcode == 2:
                    print(">>> LOGIN REQUEST RECEIVED! <<<")
                    
                    # 3. Send OP_LoginResponse (0x0003) -> Success
                    # Body: AccountID(4) + SessionKey(10) + Unk(2) = 16 bytes
                    # Total Size = 16 (Body) + 2 (OpCode) = 18 bytes
                    resp_body = bytearray()
                    resp_body.extend((100).to_bytes(4, 'little')) # ID
                    resp_body.extend(b'PYTHONKEYX')               # Key
                    resp_body.extend(b'\x00\x00')                 # Unk
                    
                    packet = bytearray()
                    packet.extend((len(resp_body) + 2).to_bytes(2, 'big')) # Header Size
                    packet.extend((3).to_bytes(2, 'little'))               # OpCode 3 (LE)
                    packet.extend(resp_body)
                    
                    conn.sendall(packet)
                    print("[TCP] Sent OP_LoginResponse (Welcome!)")
                    
                    # Keep connection open for a moment so client registers it
                    time.sleep(1)
                    print("--- SUCCESS! CHECK CLIENT SCREEN ---")
                    
        except BlockingIOError:
            pass
        except Exception as e:
            print(f"[TCP] Error: {e}")

if __name__ == "__main__":
    try:
        start_server()
    except KeyboardInterrupt:
        print("\nExiting...")
