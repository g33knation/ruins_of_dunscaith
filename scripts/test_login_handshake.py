import socket
import struct
import time

LOGIN_IP = "127.0.0.1"
LOGIN_PORT = 5998

def test_login_connect():
    print(f"Connecting to Login Server at {LOGIN_IP}:{LOGIN_PORT}")
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((LOGIN_IP, LOGIN_PORT))
        print("-> Connected!")
        
        # RoF2 Login Protocol Check
        # 1. Server sends Packet on connect? Or Client sends first?
        # Usually Client sends LoginRequest.
        # Header: OpCode(4), Size(4)? Or just OpCode?
        # Let's try sending a dummy login packet.
        # OpCode 0x0001 (Login)
        # Struct: user, pass...
        
        # Just verifying connection is handled by server is enough for "Reachability" check
        # But let's see if server kicks us or logs "New connection"
        
        # Sleep to let server log it
        time.sleep(1)
        
        # Send garbage to trigger "New connection: ..." log at least
        print("-> Sending handshake...")
        # Header: OpCode=1, Size=0 (Stub)
        packet = struct.pack("<I", 1) 
        sock.send(packet)
        
        # Read response?
        # try:
        #    data = sock.recv(1024)
        #    print(f"<- Received: {data.hex()}")
        # except:
        #    pass
            
        sock.close()
        print("Test Finished.")
        return True
    except Exception as e:
        print(f"Connection Failed: {e}")
        return False

if __name__ == "__main__":
    test_login_connect()
