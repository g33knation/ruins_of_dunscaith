import socket
import struct
import sys

# --- CONFIGURATION ---
UDP_IP = "0.0.0.0"
UDP_PORT = 5998
TCP_IP = "0.0.0.0"
TCP_PORT = 5998

# --- MAGIC BYTES (The "Golden" Guess) ---
# Port 5998 in Little Endian (0x6E, 0x17) -> Client reads 0x176E (5998)
PORT_BYTES = b'\x6e\x17' 
# Status 1 (Up) in Little Endian
STATUS_BYTES = b'\x01\x00'

def start_server():
    sock_udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock_udp.bind((UDP_IP, UDP_PORT))
    print(f"[UDP] Listening on {UDP_IP}:{UDP_PORT}")

    sock_tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock_tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock_tcp.bind((TCP_IP, TCP_PORT))
    sock_tcp.listen(1)
    print(f"[TCP] Listening on {TCP_IP}:{TCP_PORT}")
    
    sock_udp.setblocking(False)
    sock_tcp.setblocking(False)

    print("--- WAITING FOR CLIENT ---")

    while True:
        # CHECK UDP
        try:
            data, addr = sock_udp.recvfrom(1024)
            
            # --- FIX: Case Insensitive Search ---
            lower_data = data.lower()
            magic_idx = lower_data.find(b"everquest")
            
            if magic_idx != -1:
                print(f"[UDP] Magic Word Found! Sending Invite to {addr}...")
                
                response = bytearray()
                null_idx = data.find(b'\x00', magic_idx)
                
                response.extend(data[:null_idx+1]) # Echo raw header
                response.extend(b'\x7F\x00\x00\x01') # IP 127.0.0.1
                response.extend(PORT_BYTES)          # Port (LE)
                response.extend(STATUS_BYTES)        # Status (UP)
                response.extend(b'\x00\x00\x00\x00') # Load
                response.extend(b'PythonLogin\x00')
                response.extend(b'Python EQEmu Login\x00')
                
                sock_udp.sendto(response, addr)
        except BlockingIOError:
            pass

        # CHECK TCP
        try:
            conn, tcp_addr = sock_tcp.accept()
            print(f"\n[TCP] !!! CONNECTION ACCEPTED from {tcp_addr} !!!")
            conn.setblocking(True)
            
            # Send Handshake (Size 14 = 12 body + 2 opcode)
            packet = bytearray()
            packet.extend((14).to_bytes(2, 'big'))   # Size (BE)
            packet.extend((1).to_bytes(2, 'big'))    # OpCode 1 (BE)
            packet.extend(b'\x00' * 12)              # Body (12 bytes)
            
            conn.sendall(packet)
            print("[TCP] Sent OP_SessionReady")
            
            # Wait for Login
            data = conn.recv(1024)
            if data:
                print(f"[TCP] Received Data: {data.hex()}")
                if len(data) > 4:
                    opcode = int.from_bytes(data[2:4], 'big')
                    print(f"[TCP] OpCode Received: 0x{opcode:04X}")
                    if opcode == 2:
                        print("\n>>> GOLDEN PATH CONFIRMED <<<")
                        print(f"UDP Port Bytes: {PORT_BYTES.hex()}")
                        print(f"UDP Status Bytes: {STATUS_BYTES.hex()}")
                        print(f"TCP Framing: Size included OpCode (Size=14)")
                        sys.exit(0)
            
        except BlockingIOError:
            pass
        except Exception as e:
            print(f"[TCP] Error: {e}")

if __name__ == "__main__":
    try:
        start_server()
    except KeyboardInterrupt:
        print("\nExiting...")
