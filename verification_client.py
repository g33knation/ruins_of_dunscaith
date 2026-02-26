import socket
import struct
import time
import binascii
import zlib
from typing import Optional, List, Tuple

# --- CONFIGURATION ---
LOGIN_HOST = '127.0.0.1'
LOGIN_PORT = 5998
WORLD_HOST = '127.0.0.1'
WORLD_PORT = 9000
ZONE_HOST = '127.0.0.1' # Usually provided by World
TEST_USER = "testuser"
TEST_PASS = "testpass"

# --- OPCODES ---
# Login Opcodes
OP_SessionReady = 0x0001
OP_Login = 0x0002
OP_LoginApproval = 0x0017

# World Opcodes
OP_SendLoginInfo = 0x7a09
OP_ApproveWorld = 0x7499
OP_SendCharInfo = 0x00d2
OP_CharSelectRequest = 0x00d1
OP_EnterWorld = 0x57c3
OP_ZoneServerInfo = 0x4c44

# Zone Opcodes
OP_ZoneEntry = 0x2211 # Placeholder, check actual RoF2
OP_ClientReady = 0x0001 # Logic specific
OP_ChannelMessage = 0x0211 # Say/Tell/etc.

def calc_eq_crc(packet: bytes, crc_key: int = 0xFFFFFFFF) -> bytes:
    data = packet + struct.pack('>I', crc_key)
    crc = binascii.crc32(data) & 0xFFFF
    return struct.pack('<H', crc)

class MockEqClient:
    def __init__(self):
        self.account_id = 0
        self.session_key = ""
        self.char_name = ""
        self.world_session_id = 0x12345678
        self.seq = 0

    def pack_login(self, opcode: int, payload: bytes = b"") -> bytes:
        size = len(payload) + 2
        return struct.pack('>H', size) + struct.pack('<H', opcode) + payload

    def pack_udp(self, trans_op: int, app_op: Optional[int] = None, payload: bytes = b"", sequenced: bool = False) -> bytes:
        if sequenced:
            header = struct.pack('>H', 0x0009) + struct.pack('>H', self.seq)
            self.seq += 1
        else:
            header = struct.pack('>H', trans_op)
            
        if app_op is not None:
            body = struct.pack('<H', app_op) + payload
        else:
            body = payload
            
        full = header + body
        return full + calc_eq_crc(full)

    def decompress_if_needed(self, data: bytes) -> bytes:
        if len(data) > 2 and data[0] == 0x5a: # Check for Zlib header
            try:
                # RoF2 often compresses after 2-byte header or uses raw zlib
                return zlib.decompress(data[2:])
            except zlib.error:
                try:
                    return zlib.decompress(data)
                except zlib.error:
                    return data
        return data

    def run_login(self):
        print(f"[*] Connecting to Login Server {LOGIN_HOST}:{LOGIN_PORT}...")
        try:
            ls = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            ls.settimeout(5)
            ls.connect((LOGIN_HOST, LOGIN_PORT))
            
            ls.send(self.pack_login(OP_SessionReady))
            ls.recv(1024) # Skip response
            
            login_payload = TEST_USER.encode() + b"\x00" + TEST_PASS.encode() + b"\x00" + b"\x00" * 43
            ls.send(self.pack_login(OP_Login, login_payload))
            
            for _ in range(5):
                data = ls.recv(1024)
                if len(data) < 4: continue
                app_op = struct.unpack_from('<H', data, 2)[0]
                if app_op == OP_LoginApproval:
                    aid, key, err = struct.unpack_from('<I10sH', data, 4)
                    if err == 0:
                        self.account_id = aid
                        self.session_key = key.decode('ascii', errors='ignore').strip('\x00')
                        print(f"[+] Login SUCCESS! ID: {self.account_id}, Key: {self.session_key}")
                        ls.close()
                        return True
            ls.close()
        except Exception as e:
            print(f"[-] Login Error: {e}")
        return False

    def run_world(self):
        print(f"[*] Connecting to World Server {WORLD_HOST}:{WORLD_PORT}...")
        try:
            ws = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            ws.settimeout(3)
            
            # Session Request
            ws.sendto(struct.pack('>H', 0x0001) + struct.pack('<III', 1, self.world_session_id, 1024), (WORLD_HOST, WORLD_PORT))
            ws.recvfrom(1024)
            
            # Send Ready & Login Info
            ws.sendto(self.pack_udp(0x0009, OP_SessionReady, sequenced=True), (WORLD_HOST, WORLD_PORT))
            payload = struct.pack('<I10s', self.account_id, self.session_key.encode() + b'\x00')
            ws.sendto(self.pack_udp(0x0009, OP_SendLoginInfo, payload, sequenced=True), (WORLD_HOST, WORLD_PORT))
            
            # Wait for Approval & Char Info
            handoff = None
            for _ in range(50):
                try:
                    data, _ = ws.recvfrom(4096)
                    trans_op = struct.unpack_from('>H', data, 0)[0]
                    app_op = 0
                    if trans_op == 0x0009: app_op = struct.unpack_from('<H', data, 4)[0]
                    elif trans_op == 0x0002: app_op = struct.unpack_from('<H', data, 2)[0]
                    
                    if app_op == OP_ApproveWorld:
                        print("[+] World Approved Session")
                        ws.sendto(self.pack_udp(0x0009, OP_CharSelectRequest, sequenced=True), (WORLD_HOST, WORLD_PORT))
                    elif app_op == OP_SendCharInfo:
                        # Character List payload starts AFTER OpCode
                        app_data = self.decompress_if_needed(data[6:])
                        count = struct.unpack_from('<I', app_data, 0)[0]
                        if count > 0:
                            self.char_name = struct.unpack_from('64s', app_data, 4)[0].decode('ascii', errors='ignore').strip('\x00')
                            print(f"[+] Found Character: {self.char_name}")
                            ws.sendto(self.pack_udp(0x0009, OP_EnterWorld, self.char_name.encode() + b'\x00', sequenced=True), (WORLD_HOST, WORLD_PORT))
                    elif app_op == OP_ZoneServerInfo:
                        app_data = self.decompress_if_needed(data[6:])
                        ip = struct.unpack_from('128s', app_data, 0)[0].decode('ascii', errors='ignore').strip('\x00')
                        port = struct.unpack_from('<H', app_data, 128)[0]
                        handoff = (ip, port)
                        print(f"[+] Received Zone Handoff: {ip}:{port}")
                        break
                except socket.timeout: continue
            
            ws.close()
            return handoff
        except Exception as e:
            print(f"[-] World Error: {e}")
        return None

    def run_zone(self, host, port):
        print(f"[*] Connecting to Zone Server {host}:{port}...")
        try:
            zs = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            zs.settimeout(3)
            self.seq = 0 # Reset seq for zone
            
            # Handshake
            zs.sendto(struct.pack('>H', 0x0001) + struct.pack('<III', 1, 0x87654321, 1024), (host, port))
            zs.recvfrom(1024)
            zs.sendto(self.pack_udp(0x0009, OP_SessionReady, sequenced=True), (host, port))
            
            print("[+] Zone Handshake Complete. Testing AI Bridge...")
            # Send /ai command as a Chat Message
            ai_cmd = "/ai What is the lore of this zone?".encode() + b"\x00"
            zs.sendto(self.pack_udp(0x0009, OP_ChannelMessage, ai_cmd, sequenced=True), (host, port))
            print("[*] Sent /ai command to Zone Server")
            
            for _ in range(10):
                try:
                    data, _ = zs.recvfrom(4096)
                    print(f"[DEBUG] Zone Received Packet (len={len(data)})")
                except socket.timeout: break
            
            zs.close()
            print("[+] Zone Test Sequence Finished.")
            return True
        except Exception as e:
            print(f"[-] Zone Error: {e}")
        return False

if __name__ == "__main__":
    client = MockEqClient()
    if client.run_login():
        handoff = client.run_world()
        if handoff:
            client.run_zone(handoff[0], handoff[1])
        else:
            print("[-] Test failed at World stage.")
    else:
        print("[-] Test failed at Login stage.")
