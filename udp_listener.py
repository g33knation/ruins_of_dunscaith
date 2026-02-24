import socket
import sys

UDP_IP = "0.0.0.0"
UDP_PORT = 9000

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind((UDP_IP, UDP_PORT))

print(f"UDP Listener listening on {UDP_IP}:{UDP_PORT}")

while True:
    data, addr = sock.recvfrom(1024)
    print(f"RECEIVED PACKET from {addr}: {data.hex()}")
    sys.stdout.flush()
