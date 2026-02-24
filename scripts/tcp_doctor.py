import socket
import struct
import binascii

# Configuration
SERVER_IP = "127.0.0.1"
SERVER_PORT = 5998

def diagnose_tcp():
    print(f"[*] Connecting to TCP {SERVER_IP}:{SERVER_PORT}...")
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(3.0)
        sock.connect((SERVER_IP, SERVER_PORT))
        print("[+] Connection Established! Waiting for Greeting...")

        # Read the first packet from the server
        data = sock.recv(1024)
        
        if not data:
            print("[-] Server closed connection without sending data.")
            return

        print(f"\n[+] Received {len(data)} bytes.")
        print(f"    Raw Hex: {binascii.hexlify(data).decode('utf-8')}")

        # DIAGNOSIS LOGIC
        if len(data) >= 2:
            # Check First 2 Bytes (OpCode?)
            b1, b2 = data[0], data[1]
            print(f"    Byte 0: 0x{b1:02X}")
            print(f"    Byte 1: 0x{b2:02X}")

            if b1 == 0x01 and b2 == 0x00:
                print("\n[!] DIAGNOSIS: LITTLE ENDIAN DETECTED (BAD)")
                print("    Server sent '01 00'. Client reads this as OpCode 256.")
                print("    FIX: You must write OpCode as Big Endian (00 01).")
            elif b1 == 0x00 and b2 == 0x01:
                print("\n[+] DIAGNOSIS: BIG ENDIAN DETECTED (GOOD)")
                print("    OpCode is correct. Checking framing...")
            elif b1 == 0x00 and b2 == 0x0F:
                print("\n[!] DIAGNOSIS: SIZE-FIRST FRAMING DETECTED")
                print("    Server sent Size (15) first, then OpCode.")
                print("    RoF2 might expect OpCode FIRST: [OpCode BE][Size BE][Body]")
            else:
                print("\n[?] DIAGNOSIS: Unknown Packet Format.")
        
    except ConnectionRefusedError:
        print("[-] Connection Refused. Is the server running?")
    except socket.timeout:
        print("[-] Timed out. Server accepted TCP but sent nothing.")
    finally:
        sock.close()

if __name__ == "__main__":
    diagnose_tcp()
