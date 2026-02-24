import socket
import struct
import sys

# --- CONFIGURATION ---
UDP_IP = "0.0.0.0"
UDP_PORT = 5998
TCP_IP = "0.0.0.0"
TCP_PORT = 5998

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

    packet_count = 0
    
    # 4 Configurations to cycle through
    configs = [
        (b'\x6e\x17', b'\x01\x00', "Port: LE (5998), Status: 1 (UP)"),
        (b'\x17\x6e', b'\x01\x00', "Port: BE (5998), Status: 1 (UP)"),
        (b'\x6e\x17', b'\xff\xff', "Port: LE (5998), Status: -1 (Unlocked)"),
        (b'\x17\x6e', b'\xff\xff', "Port: BE (5998), Status: -1 (Unlocked)")
    ]

    print("--- BRUTE FORCING CONFIGURATION ---")

    while True:
        # CHECK UDP
        try:
            data, addr = sock_udp.recvfrom(1024)
            packet_count += 1
            
            # Change config every 5 packets
            config_idx = (packet_count // 5) % 4
            current_port, current_status, desc = configs[config_idx]

            lower_data = data.lower()
            magic_idx = lower_data.find(b"everquest")
            
            if magic_idx != -1:
                print(f"[UDP #{packet_count}] Trying: {desc}")
                
                response = bytearray()
                null_idx = data.find(b'\x00', magic_idx)
                
                response.extend(data[:null_idx+1]) 
                response.extend(b'\x7F\x00\x00\x01') 
                response.extend(current_port)       
                response.extend(current_status)     
                response.extend(b'\x00\x00\x00\x00') 
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
            
            # If we get here, the LAST configuration used was the correct one.
            winning_idx = (packet_count // 5) % 4
            winner = configs[winning_idx]
            
            print("\n" + "="*40)
            print(f"WINNER FOUND: {winner[2]}")
            print(f"Port Bytes: {winner[0].hex()}")
            print(f"Status Bytes: {winner[1].hex()}")
            print("="*40 + "\n")
            
            # Send Handshake (Size 14)
            packet = bytearray()
            packet.extend((14).to_bytes(2, 'big'))   
            packet.extend((1).to_bytes(2, 'big'))    
            packet.extend(b'\x00' * 12)              
            
            conn.sendall(packet)
            print("[TCP] Sent OP_SessionReady")
            
            data = conn.recv(1024)
            if data:
                print(f"[TCP] Received Data: {data.hex()}")
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
