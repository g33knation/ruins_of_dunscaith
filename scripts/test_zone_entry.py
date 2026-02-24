import socket
import struct
import time
import sys

# Configuration
UDP_IP = "127.0.0.1"
ZONE_PORT = 9001
TIMEOUT = 3.0

def test_zone_entry():
    print(f"Testing Zone Entry on {UDP_IP}:{ZONE_PORT}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(TIMEOUT)

import zlib

def test_zone_entry():
    # ... setup ...
    print(f"Testing Zone Entry on {UDP_IP}:{ZONE_PORT}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(TIMEOUT)

    try:
        # 1. Send ZoneSessionRequest (OpCode 0x0001)
        # Reliable Protocol: Op(u16) + Seq(u16) + Payload
        session_id = 1
        seq_num = 0
        unknown = 0
        req_packet = struct.pack("<HHII", 0x0001, seq_num, unknown, session_id)
        sock.sendto(req_packet, (UDP_IP, ZONE_PORT))
        print("-> Sent ZoneSessionRequest")

        # Helper to process incoming packet
        def recv_and_process():
            data, addr = sock.recvfrom(65535)
            
            # Decompress if needed
            if len(data) > 2 and data[0] == 0x5a and data[1] == 0xa5:
                # print("<- Decompressing packet...")
                data = zlib.decompress(data[2:])
            
            if len(data) < 4:
                return None, None, 0
            
            opcode = struct.unpack("<H", data[:2])[0]
            seq = struct.unpack("<H", data[2:4])[0]
            payload = data[4:]
            
            # Handle ACK from Server
            if opcode == 0x0015:
                # print(f"<- Received ACK for Seq {seq}")
                return opcode, payload, seq
                
            # Send ACK for this packet
            # Ack Packet: Op(0x0015) + Seq(u16)
            ack_packet = struct.pack("<HH", 0x0015, seq)
            sock.sendto(ack_packet, (UDP_IP, ZONE_PORT))
            # print(f"-> Sent ACK for Seq {seq}")
            
            return opcode, payload, seq

        # 2. Receive SessionResponse (OpCode 0x0002)
        start_time = time.time()
        while True:
            if time.time() - start_time > TIMEOUT:
                raise socket.timeout
            try:
                opcode, payload, seq = recv_and_process()
                if opcode == 0x0002:
                    print(f"<- Received SessionResponse (OpCode {hex(opcode)})")
                    break
            except socket.timeout:
                raise

        # 3. Send ZoneEntry (OpCode 0x001D)
        char_name = b"RustLord" + b"\x00" * (64 - len(b"RustLord"))
        char_id = 2 
        zone_id = 202
        instance_id = 0
        
        seq_num += 1
        entry_payload = char_name + struct.pack("<III", char_id, zone_id, instance_id)
        entry_packet = struct.pack("<HH", 0x001D, seq_num) + entry_payload
        sock.sendto(entry_packet, (UDP_IP, ZONE_PORT))
        print("-> Sent ZoneEntry")
        
        # 4. Receive PlayerProfile (OpCode 0x0026)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 200000) # Ensure kernel buf is large too
        
        while True:
             if time.time() - start_time > TIMEOUT:
                raise socket.timeout
             
             opcode, payload, seq = recv_and_process()
             if opcode is None or opcode == 0x0015: continue
             
             length = len(payload) + 4 # Approx
             print(f"<- Received Packet: OpCode {hex(opcode)}, Payload Size {len(payload)}")
        
             if opcode == 0x0026:
                 # Check Payload size (PlayerProfile size)
                 # Expect ~24000
                 if len(payload) < 23000:
                    print("<- PlayerProfile too small! Padding failed?")
                    return False
                 print("<- PlayerProfile Size Verified (~24KB)")
                 time.sleep(2) # Wait for server logs
                 return True # Done

        return False


    except socket.timeout:
        print("Timed out waiting for server response.")
        return False
    except Exception as e:
        print(f"Error: {e}")
        return False
    finally:
        sock.close()

if __name__ == "__main__":
    if test_zone_entry():
        print("TEST PASSED: Zone Entry successful.")
        sys.exit(0)
    else:
        print("TEST FAILED.")
        sys.exit(1)
