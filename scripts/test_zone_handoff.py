import socket
import struct
import time
import sys

# Configuration
UDP_IP = "127.0.0.1"
WORLD_PORT = 9000
TIMEOUT = 2.0

def test_zone_handoff():
    print(f"Testing Zone Handoff on {UDP_IP}:{WORLD_PORT}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(TIMEOUT)

    try:
        # 1. Send SessionRequest (OpCode 0x0001)
        # account_id=1, key="12345"
        account_id = 1
        session_key = b"12345" + b"\x00" * (32 - len(b"12345"))
        req_packet = struct.pack("<HI32s", 1, account_id, session_key)
        sock.sendto(req_packet, (UDP_IP, WORLD_PORT))
        print("-> Sent SessionRequest")

        # 2. Receive SessionResponse (OpCode 0x0002)
        data, addr = sock.recvfrom(4096)
        opcode = struct.unpack("<H", data[:2])[0]
        if opcode != 0x0002:
            print(f"<- Received unexpected OpCode: {hex(opcode)}")
            return False
        print(f"<- Received SessionResponse (OpCode {hex(opcode)})")

        # 3. Receive CharSelectResponse (OpCode 0x0019) 
        # (Server sends this immediately after SessionResponse if auth success)
        data, addr = sock.recvfrom(4096)
        opcode = struct.unpack("<H", data[:2])[0]
        if opcode != 0x0019:
            print(f"<- Received unexpected OpCode: {hex(opcode)} (Expected CharSelect)")
            return False
        print(f"<- Received CharSelectResponse (OpCode {hex(opcode)})")
        
        # 4. Send EnterWorld (OpCode 0x001D for RoF2)
        # Payload: char_id (u32) or empty?
        # Let's send char_id = 1 (assuming it exists or using 0 for 'current')
        char_id = 1
        # OpCode 0x001D + char_id
        enter_packet = struct.pack("<HI", 0x001D, char_id)
        sock.sendto(enter_packet, (UDP_IP, WORLD_PORT))
        print("-> Sent EnterWorldRequest (RoF2)")

        # 5. Receive ZoneAddress (OpCode 0x000D)
        data, addr = sock.recvfrom(4096)
        opcode = struct.unpack("<H", data[:2])[0]
        if opcode != 0x000D:
             print(f"<- Received unexpected OpCode: {hex(opcode)} (Expected ZoneAddress)")
             return False
        
        # Parse ZoneAddress
        # Header: [OpCode: u16] [Seq: u16] (Total 4 bytes)
        # Payload: [IP: 16] [Port: 2] [Pad: 2]
        print(f"DEBUG Hex: {data.hex()}")
        if len(data) < 4 + 16 + 2 + 2:
            print("<- ZoneAddress packet too short")
            return False
            
        seq = struct.unpack("<H", data[2:4])[0]
        ip_bytes = data[4:20]
        port = struct.unpack("<H", data[20:22])[0]
        
        ip_str = ip_bytes.split(b'\0', 1)[0].decode('utf-8')
        print(f"<- Received ZoneAddress: IP={ip_str}, Port={port}")
        
        return True

    except socket.timeout:
        print("Timed out waiting for server response.")
        return False
    except Exception as e:
        print(f"Error: {e}")
        return False
    finally:
        sock.close()

if __name__ == "__main__":
    if test_zone_handoff():
        print("TEST PASSED: Zone Handoff successful.")
        sys.exit(0)
    else:
        print("TEST FAILED.")
        sys.exit(1)
