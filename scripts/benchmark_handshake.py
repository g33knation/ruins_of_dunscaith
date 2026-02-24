import socket
import struct
import time
import threading
import concurrent.futures

UDP_IP = "127.0.0.1"
UDP_PORT = 9000
NUM_CLIENTS = 50

def run_client(client_id):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2)
    
    # SessionRequest: OpCode 1
    # ID: 1 (reusing same ID is fine for load test if server allows, or increment)
    # Key: "12345"
    account_id = 1
    session_key = b"12345" + b"\x00" * (32 - len(b"12345"))
    packet = struct.pack("<HI32s", 1, account_id, session_key)
    
    start_time = time.time()
    try:
        sock.sendto(packet, (UDP_IP, UDP_PORT))
        data, _ = sock.recvfrom(1024)
        end_time = time.time()
        
        # Check if OpCode 2
        if len(data) >= 2:
            resp_opcode = struct.unpack("<H", data[:2])[0]
            if resp_opcode == 0x0002:
                return (True, end_time - start_time)
            
    except socket.timeout:
        return (False, 2.0)
    except Exception as e:
        return (False, 0.0)
    
    return (False, 0.0)

def benchmark():
    print(f"Starting Benchmark: {NUM_CLIENTS} concurrent clients...")
    
    success_count = 0
    total_time = 0
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=NUM_CLIENTS) as executor:
        futures = [executor.submit(run_client, i) for i in range(NUM_CLIENTS)]
        
        for future in concurrent.futures.as_completed(futures):
            success, latency = future.result()
            if success:
                success_count += 1
                total_time += latency
    
    avg_latency = (total_time / success_count) if success_count > 0 else 0
    print(f"Results: {success_count}/{NUM_CLIENTS} successful.")
    print(f"Average Latency: {avg_latency*1000:.2f} ms")

if __name__ == "__main__":
    benchmark()
