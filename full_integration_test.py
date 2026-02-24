import socket
import struct
import time
import sys
import binascii
from typing import Optional, List, cast

# Opcodes
OP_SessionRequest = 0x0001
OP_SessionResponse = 0x0002
OP_SessionReady = 0x0001 # App OpCode
OP_Login = 0x0002 # Login OpCode
OP_LoginApproval = 0x0017
OP_SendLoginInfo = 0x7a09 # World OpCode
OP_ApproveWorld = 0x7499
OP_SendCharInfo = 0x00d2
OP_CharSelectRequest = 0x00d1
OP_EnterWorld = 0x57c3
OP_ZoneServerInfo = 0x4c44

def calc_eq_crc(packet: bytes, crc_key: int = 0xFFFFFFFF) -> bytes:
    # CRC is over [PacketData] + [CRC_Key: 4 bytes BE]
    # Then result is result & 0xFFFF
    data = packet + struct.pack('>I', crc_key)
    crc = binascii.crc32(data) & 0xFFFF
    return struct.pack('<H', crc)

def pack_login_packet(opcode: int, payload: bytes = b"") -> bytes:
    # Login TCP Header: [Len: 2 bytes BE][Op: 2 bytes LE]
    # Size includes the OpCode
    size = len(payload) + 2
    return struct.pack('>H', size) + struct.pack('<H', opcode) + payload

def pack_world_udp_packet(transport_op: int, app_opcode: Optional[int] = None, app_payload: bytes = b"") -> bytes:
    # World UDP Header: [TransportOp: 2 bytes BE][Payload]
    if app_opcode is not None:
        # App Packet: [AppOp: 2 bytes LE][Payload]
        payload = struct.pack('<H', app_opcode) + app_payload
    else:
        payload = b""
    
    packet = struct.pack('>H', transport_op) + payload
    return packet + calc_eq_crc(packet)

def run_test():
    print("--- Starting Full Integration Test: Login -> World -> Zone ---")
    
    # --- PHASE 1: LOGIN ---
    login_host = '127.0.0.1'
    login_port = 5998
    
    try:
        ls = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        ls.settimeout(5)
        ls.connect((login_host, login_port))
        print(f"[+] Connected to Login Server {login_host}:{login_port}")
        
        # 1. Send OP_SessionReady
        ls.send(pack_login_packet(OP_SessionReady))
        print("[+] Sent Login OP_SessionReady")
        
        # 2. Receive SessionResponse (Inner SessionID)
        resp = ls.recv(1024)
        print(f"[+] Received Response from LS (len={len(resp)})")
        
        # 3. Send OP_Login
        # RoF2 Login Payload: [Username\0][Password\0][Rest...]
        login_payload = b"testuser\x00testpass\x00" + b'\x00' * 43
        ls.send(pack_login_packet(OP_Login, login_payload))
        print("[+] Sent OP_Login (testuser)")
        
        account_id: Optional[int] = None
        session_key: Optional[str] = None
        
        # 4. Wait for LoginApproval
        for _ in range(5):
            data: bytes = ls.recv(1024)
            if len(data) < 4: continue
            
            # Unpack login header: [size: BE][opcode: LE]
            size = struct.unpack_from('>H', data, 0)[0]
            app_op = struct.unpack_from('<H', data, 2)[0]
            
            if app_op == OP_LoginApproval:
                print("[+] Received OP_LoginApproval")
                if len(data) >= 20: 
                    account_id_val, key_bytes, error = struct.unpack_from('<I10sH', data, 4)
                    account_id = int(account_id_val)
                    session_key = key_bytes.decode('ascii', errors='ignore').strip('\x00')
                    print(f"    Account ID: {account_id}, Session Key: {session_key}, Error: {error}")
                    if error == 0:
                        print("[+] Login SUCCESS!")
                        break
                    else:
                        print(f"[-] Login REJECTED (Code {error})")
                        return
        
        ls.close()
        if account_id is None or session_key is None:
            print("[-] Failed to retrieve session data from Login Server")
            return
            
        final_account_id = cast(int, account_id)
        final_session_key = cast(str, session_key)

        # --- PHASE 2: WORLD ---
        time.sleep(1)
        world_host = '127.0.0.1'
        world_port = 9000
        
        print(f"--- Transitioning to World Server {world_host}:{world_port} ---")
        ws = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        ws.settimeout(2)
        
        # 1. Session Request (Transport 0x0001)
        session_id = 0x12345678
        sr_payload = struct.pack('<III', 1, session_id, 1024)
        ws.sendto(struct.pack('>H', 0x0001) + sr_payload, (world_host, world_port))
        print("[+] Sent World Session Request")
        
        # 2. recv session response
        data_ws, addr_ws = ws.recvfrom(1024)
        print(f"[+] Received World Session Response (len={len(data_ws)})")
        
        # 3. Session Ready (App 0x0001)
        state_seq: List[int] = [0]
        def pack_sequenced(opcode: int, payload: bytes = b"") -> bytes:
            current_seq = state_seq[0]
            p = struct.pack('>H', 0x0009) + struct.pack('>H', current_seq) + struct.pack('<H', opcode) + payload
            state_seq[0] += 1
            p += calc_eq_crc(p)
            return p

        ws.sendto(pack_sequenced(OP_SessionReady), (world_host, world_port))
        print("[+] Sent World OP_SessionReady")
        
        # 4. SendLoginInfo (App 0x7a09)
        login_info_payload = struct.pack('<I10s', final_account_id, final_session_key.encode('ascii') + b'\x00')
        ws.sendto(pack_sequenced(OP_SendLoginInfo, login_info_payload), (world_host, world_port))
        print(f"[+] Sent OP_SendLoginInfo (ID={final_account_id}, Key={final_session_key})")
        
        # 5. Wait for OP_ApproveWorld (0x7499)
        approved = False
        for _ in range(40):
            try:
                data_rx, addr_rx = ws.recvfrom(4096)
                if len(data_rx) < 4: continue
                
                trans_op = struct.unpack_from('>H', data_rx, 0)[0]
                print(f"    [DEBUG] Received TransOp: 0x{trans_op:04x}, len={len(data_rx)}")
                if trans_op == 0x0002: # App
                     app_op = struct.unpack_from('<H', data_rx, 2)[0]
                     print(f"[+] Received World App OpCode: 0x{app_op:04x}")
                     if app_op == OP_ApproveWorld:
                         print("[+] World Approved Login!")
                         approved = True
                elif trans_op == 0x0009: # Sequenced
                     app_op = struct.unpack_from('<H', data_rx, 4)[0]
                     print(f"[+] Received World Sequenced OpCode: 0x{app_op:04x}")
                     if app_op == OP_ApproveWorld:
                         print("[+] World Approved Login!")
                         approved = True
                elif trans_op == 0x0003: # Combined
                     print(f"[+] Received Combined Packet")
                     # [TransOp: 2][Len1: 1][Payload1]...[CRC: 2]
                     ptr = 2
                     while ptr < len(data_rx) - 3:
                         slen = data_rx[ptr]
                         ptr += 1
                         if slen > 0 and ptr + slen <= len(data_rx) - 2:
                             sub = data_rx[ptr : ptr + slen]
                             ptr += slen
                             if len(sub) >= 2:
                                 sop = struct.unpack_from('<H', sub, 0)[0]
                                 print(f"    - Combined SubOp: 0x{sop:04x}")
                                 if sop == OP_ApproveWorld:
                                     print("[+] World Approved Login (Combined)!")
                                     approved = True
                
                if approved and trans_op == 0x0009: # Need to wait for sequences to finish
                     pass 
            except socket.timeout:
                continue

        if not approved:
            print("[-] Failed to receive OP_ApproveWorld from World Server")
            return

        # 6. Send CharSelectRequest (App 0x00d1)
        ws.sendto(pack_sequenced(OP_CharSelectRequest), (world_host, world_port))
        print("[+] Sent OP_CharSelectRequest")

        # 7. Wait for OP_SendCharInfo (0x00d2) and send OP_EnterWorld (0x57c3)
        char_name: Optional[str] = None
        for _ in range(30):
            try:
                data_rx, addr_rx = ws.recvfrom(4096)
                if len(data_rx) < 4: continue
                
                trans_op = struct.unpack_from('>H', data_rx, 0)[0]
                payload = b""
                app_op = 0
                
                if trans_op == 0x0002: # App Packet
                    app_op = struct.unpack_from('<H', data_rx, 2)[0]
                    payload = data_rx[4:-2] # Strip CRC
                elif trans_op == 0x0009: # Sequenced
                    app_op = struct.unpack_from('<H', data_rx, 4)[0]
                    payload = data_rx[6:-2] # Strip CRC
                elif trans_op == 0x0003: # Combined
                    # [Op: 2][SubLen1: 1][SubOp1: 2][SubPayload1]...
                    print(f"    (Combined packet received, skipping deep parse for now)")
                    continue
                else:
                    continue
                
                if app_op == OP_SendCharInfo:
                    print(f"[+] Received OP_SendCharInfo (Op={trans_op:04x}, len={len(payload)})")
                    print(f"    Payload Hex (first 32): {binascii.hexlify(payload[:32]).decode()}")
                    if len(payload) >= 68:
                        count = struct.unpack_from('<I', payload, 0)[0]
                        if count > 0:
                            char_bytes = struct.unpack_from('64s', payload, 4)[0]
                            char_name = char_bytes.decode('ascii', errors='ignore').strip('\x00')
                            print(f"    Found Character: '{char_name}'")
                            break
            except socket.timeout: continue

        if char_name:
            # 8. Send OP_EnterWorld (0x57c3)
            enter_world_payload = char_name.encode('ascii') + b'\x00'
            ws.sendto(pack_sequenced(OP_EnterWorld, enter_world_payload), (world_host, world_port))
            print(f"[+] Sent OP_EnterWorld for '{char_name}'")

            # 9. Wait for OP_ZoneServerInfo (0x4c44)
            for _ in range(30):
                try:
                    data_rx, addr_rx = ws.recvfrom(4096)
                    if len(data_rx) < 4: continue
                    
                    trans_op = struct.unpack_from('>H', data_rx, 0)[0]
                    app_op = 0xFFFFFFFF
                    if trans_op == 0x0002:
                        app_op = struct.unpack_from('<H', data_rx, 2)[0]
                    elif trans_op == 0x0009:
                        app_op = struct.unpack_from('<H', data_rx, 4)[0]
                    
                    if app_op == OP_ZoneServerInfo:
                        print("[!!!] SUCCESS: Received OP_ZoneServerInfo (Handoff)!")
                        # Payload: [IP: 128 bytes][Port: 2 bytes LE]
                        offset = 4 if trans_op == 0x0002 else 6
                        ip_handoff = struct.unpack_from('128s', data_rx, offset)[0].decode('ascii', errors='ignore').strip('\x00')
                        port_handoff = struct.unpack_from('<H', data_rx, offset + 128)[0]
                        print(f"    Redirecting to Zone: {ip_handoff}:{port_handoff}")
                        print("--- Phase 6 Integration Test Complete! ---")
                        return
                except socket.timeout: continue

        print("[-] Failed to complete zoning flow")
        ws.close()

    except Exception as e:
        print(f"[-] Error: {e}")

if __name__ == "__main__":
    run_test()
