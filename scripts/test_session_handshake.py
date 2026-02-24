import socket
import struct
import time

def test_handshake():
    UDP_IP = "127.0.0.1"
    UDP_PORT = 9000

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2)

    # OpCode 0x0001 (SessionRequest) - Little Endian
    opcode = 1
    
    # SessionRequestPacket
    # account_id: u32 (e.g. 1)
    # session_key: [u8; 32] (should be numeric string for parsing)
    account_id = 1
    session_key = b"12345" + b"\x00" * (32 - len(b"12345"))

    # Pack: H (u16), I (u32), 32s (32 bytes)
    # < means little-endian 
    packet = struct.pack("<HI32s", opcode, account_id, session_key)

    print(f"Sending SessionRequest to {UDP_IP}:{UDP_PORT}")
    print(f"Hex: {packet.hex()}")
    
    sock.sendto(packet, (UDP_IP, UDP_PORT))

    try:
        data, addr = sock.recvfrom(1024)
        print(f"Received packet from {addr}")
        print(f"Hex: {data.hex()}")
        
        # Parse response
        # Expecting OpCode 0x0002 (SessionResponse)
        # u16 opcode
        if len(data) >= 2:
            resp_opcode = struct.unpack("<H", data[:2])[0]
            print(f"Response OpCode: {resp_opcode:#06x}")
            
            if resp_opcode == 0x0002:
                print("SUCCESS: Received SessionResponse")
                # Parse payload
                # session_id: u32
                # session_key: [u8; 32]
                # max_length: u32
                # unknown: u32
                if len(data) >= 2 + 4 + 32 + 4 + 4:
                    payload = data[2:]
                    sid, skey, mlen, unk = struct.unpack("<I32sII", payload[:44])
                    print(f"SessionID: {sid}")
                    print(f"SessionKey: {skey.hex()}")
                    print(f"MaxLength: {mlen}")
            elif resp_opcode == 0x0019: # CharSelectInfo (if handshake skipped/mocked)
                 print("Received CharSelectInfo")
            else:
                 print("Received unexpected opcode")
        
    except socket.timeout:
        print("Timeout waiting for response. Is the server running?")

if __name__ == "__main__":
    test_handshake()
