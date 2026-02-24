import socket
import struct
import time

def test_login_flow():
    print("Testing EQEmu Rust Login Server TCP Flow...")
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5.0)
        s.connect(('127.0.0.1', 5998))
        print("[+] Connected to 127.0.0.1:5998")

        # 1. Send OP_SessionReady (0x0001)
        # Payload: 2 bytes opcode
        payload = struct.pack('<H', 0x0001)
        length = len(payload)
        packet = struct.pack('>H', length) + payload
        s.send(packet)
        print("[+] Sent OP_SessionReady")

        # 2. Wait for SessionReady response or similar
        # Depending on the logic, server might not respond immediately until SessionRequest or SendLoginInfo
        
        # 3. Send OP_Login (0x0002) - Dummy credentials
        # RoF2 Login Payload: [Username\0][Password\0][Rest...]
        login_payload = struct.pack('<H', 0x0002)
        login_payload += b"testuser\x00testpass\x00" + b'\x00' * 43
        
        packet = struct.pack('>H', len(login_payload)) + login_payload
        s.send(packet)
        print("[+] Sent OP_Login (testuser / testpass)")

        # 4. Receive Responses in a loop
        while True:
            resp_len_data = s.recv(2)
            if not resp_len_data:
                print("[-] Connection closed by server")
                break
                
            resp_len = struct.unpack('>H', resp_len_data)[0]
            data = s.recv(resp_len)
            
            if len(data) >= 2:
                op = struct.unpack('<H', data[:2])[0]
                print(f"[+] Received Response OpCode: 0x{op:04x}")
                if op == 0x0017: # OP_LoginApproval
                    print(f"    Payload size: {len(data)-2} bytes")
                    # Should be a LoginResponseBody struct: account_id (4), session_key (10), error_code (2)
                    if len(data) >= 18: 
                        account_id, _, err_code = struct.unpack("<I10sH", data[2:18])
                        print(f"    Account DB ID: {account_id}, Error Code: {err_code}")
                        if account_id > 0 and err_code == 0:
                            print("[+] SUCCESS: Login Approved! Integrations function works!")
                            break
                        else:
                            print("[-] SUCCESS(Protocol) but Error(Login): Auth rejected (account not found or bad pass).")
                            break
                    else:
                        print("[-] SUCCESS: Received OP_LoginApproval but payload too small.")
                        break
            else:
                print("[-] Received malformed packet")

    except Exception as e:
        print(f"[-] Test failed: {e}")
    finally:
        s.close()

if __name__ == '__main__':
    test_login_flow()
