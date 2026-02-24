import socket
import struct
import sys
import time

# --- CONFIGURATION: PORT 5999 ---
UDP_IP = "0.0.0.0"
UDP_PORT = 5999
TCP_IP = "0.0.0.0"
TCP_PORT = 5999

# --- PACKET SETTINGS ---
# Port 5999 in Little Endian (0x176F -> 6F 17)
PORT_BYTES = b'\x6f\x17' 
STATUS_BYTES = b'\x01\x00'

def start_server():
    print(f"--- MOCK SERVER ON PORT 5999 ---")
    
    sock_udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock_udp.bind((UDP_IP, UDP_PORT))
    sock_udp.setblocking(False)
    print(f"[UDP] Listening on {UDP_IP}:{UDP_PORT}")

    sock_tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock_tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock_tcp.bind((TCP_IP, TCP_PORT))
    sock_tcp.listen(1)
    sock_tcp.setblocking(False)
    print(f"[TCP] Listening on {TCP_IP}:{TCP_PORT}")

    print("--- WAITING FOR CLIENT ---")

    while True:
        # UDP
        try:
            data, addr = sock_udp.recvfrom(1024)
            if b"everquest" in data.lower():
                print(f"[UDP] Ping from {addr}")
                response = bytearray()
                magic_idx = data.lower().find(b"everquest")
                null_idx = data.find(b'\x00', magic_idx)
                
                response.extend(data[:null_idx+1]) 
                response.extend(b'\x7F\x00\x00\x01') 
                response.extend(PORT_BYTES)          # 5999 LE
                response.extend(STATUS_BYTES)        
                response.extend(b'\x00\x00\x00\x00') 
                response.extend(b'Python5999\x00')
                response.extend(b'Fresh Port\x00')
                
                sock_udp.sendto(response, addr)
        except BlockingIOError:
            pass

        # TCP
        try:
            conn, tcp_addr = sock_tcp.accept()
            print(f"\n[TCP] !!! CONNECTED on 5999 !!!")
            conn.setblocking(True)
            
            # OP_SessionReady (Size=14, OpCode=1 LE)
            packet = bytearray()
            packet.extend((14).to_bytes(2, 'big'))      
            packet.extend((1).to_bytes(2, 'little'))    
            packet.extend(b'\x00' * 12)                 
            conn.sendall(packet)
            print("[TCP] Sent SessionReady")

            data = conn.recv(1024)
            if data:
                 print(">>> LOGIN RECEIVED! SUCCESS! <<<")
                 sys.exit(0)
                    
        except BlockingIOError:
            pass
        except Exception as e:
            print(f"[TCP] Error: {e}")

if __name__ == "__main__":
    start_server()
