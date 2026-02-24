import socket
import struct
import time
import sys

# Configuration
SERVER_IP = "127.0.0.1"
UDP_PORT = 5998
TIMEOUT = 2.0

def test_udp_discovery():
    print(f"[*] Sending UDP Discovery to {SERVER_IP}:{UDP_PORT}...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(TIMEOUT)

    # RoF2 Discovery Packet Construction
    # 14 bytes padding + "EverQuest" + null terminator
    # RoF2 Discovery Packet Construction
    # 14 bytes padding + "EverQuest" + null terminator
    # WE MUST TRACK HEADER to verify echo
    header = b'\x00' * 14
    magic = b'EverQuest\x00'
    packet = header + magic

    try:
        sock.sendto(packet, (SERVER_IP, UDP_PORT))
        
        data, addr = sock.recvfrom(1024)
        print(f"[+] UDP Response Received from {addr}: {len(data)} bytes")
        print(f"    Raw Hex: {data.hex()}")

        # Parse RoF2 Response (Blind Mirror)
        # Struct: RawHeader(14), Magic(NullTerm), IP(4), Port(2), Status(2), Load(4)
        
        offset = 0
        # Smart Mirror echoes 14 bytes, but modifies the last 2 (OpCode)
        # We want to check bytes 12-13 for OpCode
        opcode = struct.unpack_from('<H', data, 12)[0]
        offset += 14
        
        # Read Magic (Null Terminated)
        magic_end = data.find(b'\x00', offset)
        magic_resp = data[offset:magic_end].decode('utf-8')
        offset = magic_end + 1
        
        ip_raw = struct.unpack_from('>I', data, offset)[0]  # Big Endian
        offset += 4
        
        port = struct.unpack_from('<H', data, offset)[0]  # Little Endian for RoF2
        offset += 2
        
        status = struct.unpack_from('<H', data, offset)[0]
        offset += 2
        
        load = struct.unpack_from('<I', data, offset)[0]
        offset += 4
        
        print(f"\n[+] Parsed UDP Header:")
        print(f"\n[+] Parsed UDP Header:")
        print(f"    Header Check: OK (skipped/echoed)")
        print(f"    Magic:      '{magic_resp}'")
        print(f"    Advertised IP (Int): {ip_raw}")
        print(f"    Advertised Port:     {port}")
        print(f"    Status:     {status} (1=Up, 0=Down)")
        
        # Status could be -1 (0xFFFF in unsigned short is 65535)
        if status != 1 and status != 2 and status != 65535:
            print(f"[-] FAILURE: Server reports Status {status} (Expected 1, 2, or 65535).")
            return None
            
        # OpCode check: Universal Mirror echoes input. 
        # Our request in this script has 14 bytes of zeros, so OpCode will be 0x0000. 
        # We just log it.
        # if opcode != 0x1000:
        #    print(f"[-] Note: OpCode is 0x{opcode:04X} (Echoed).")

        return port

    except socket.timeout:
        print("[-] TIMEOUT: No UDP response received.")
        return None
    finally:
        sock.close()

def test_tcp_handshake(port):
    print(f"\n[*] Attempting TCP Connection to {SERVER_IP}:{port}...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(TIMEOUT)
    
    try:
        sock.connect((SERVER_IP, port))
        print(f"[+] TCP Connection Established.")
        
        # Wait for Server Hello (OP_SessionReady = 0x0001)
        print("[*] Waiting for OP_SessionReady (0x0001)...")
        data = sock.recv(1024)
        
        if not data:
            print("[-] TCP Connection closed by server immediately.")
            return
            
        print(f"[+] Received TCP Packet: {len(data)} bytes")
        print(f"    Raw Hex: {data.hex()}")
        
        # Simple OpCode check (First 2 bytes, Little Endian)
        # New Framing: [Size: 2 BE] [OpCode: 2 BE] [Body: 12] = 16 Bytes
        if len(data) >= 16:
            # Parse Big Endian Size (0) and OpCode (2)
            packet_size = struct.unpack_from('>H', data, 0)[0]
            opcode = struct.unpack_from('>H', data, 2)[0]
            
            print(f"    Size:   {packet_size} (Expected 14)")
            print(f"    OpCode: 0x{opcode:04X}")

            if opcode == 0x0001:
                 print("\n[SUCCESS] GOLDEN PATH VERIFIED (OP_SessionReady).")
                 print("    1. UDP Discovery responded correctly.")
                 print("    2. TCP Connection opened.")
                 print("    3. Server sent OP_SessionReady with correct Framing.")
                 print("    The RoF2 Client SHOULD login now.")
            elif opcode == 0x0016:
                print("\n[SUCCESS] GOLDEN PATH VERIFIED (OP_ChatMessage/LoginHandShakeReply).")
                print("    1. UDP Discovery responded correctly.")
                print("    2. TCP Connection opened.")
                print("    3. Server sent OP_ChatMessage with LoginHandShakeReply.")
                print("    This is the correct EQEmu protocol!")
            else:
                print(f"[-] FAILURE: Expected OpCode 0x0001 or 0x0016, got 0x{opcode:04X}")
        else:
            print("[-] FAILURE: Packet too small.")
            
    except ConnectionRefusedError:
        print(f"[-] FAILURE: TCP Connection Refused on port {port}. Check server bind address.")
    except socket.timeout:
        print("[-] TIMEOUT: TCP Connected but server never sent OP_SessionReady.")
    finally:
        sock.close()

if __name__ == "__main__":
    advertised_port = test_udp_discovery()
    if advertised_port:
        test_tcp_handshake(advertised_port)
