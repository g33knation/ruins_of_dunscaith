use std::net::UdpSocket;
use std::time::Duration;

fn main() {
    println!("🤖 STARTING MOCK CLIENT...");
    
    // 1. Bind to localhost specifically
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind host socket");
    // 2. Connect to our Server
    socket.connect("127.0.0.1:9000").expect("Failed to connect to server");
    
    socket.set_read_timeout(Some(Duration::from_millis(500))).unwrap();

    // 3. SEND HANDSHAKE (Loop until response)
    // Pad to 12 bytes to satisfy server "len >= 10" check
    let handshake = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF, 0xFF, 0xFF];
    
    let mut buf = [0u8; 4096];
    let mut connected = false;

    for i in 0..5 {
        println!("📤 Sending Handshake (Attempt {})...", i+1);
        socket.send(&handshake).expect("Failed to send handshake");
        
        match socket.recv(&mut buf) {
            Ok(n) => {
                 println!("📥 RECEIVED HANDSHAKE RESPONSE ({} bytes)", n);
                 print_hex(&buf[..n]);
                 connected = true;
                 break;
            }
            Err(_) => {
                // Timeout, retry
            }
        }
    }

    if !connected {
        println!("❌ Failed to connect after 5 attempts.");
        return;
    }

    // 5. SEND KEEPALIVE / DATA (Big Endian OpCode 9)
    // This triggers the "Burst" on the server
    // [00 09] [Seq 00 00]
    let keepalive = vec![0x00, 0x09, 0x00, 0x00]; 
    socket.send(&keepalive).expect("Failed to send KeepAlive");
    println!("📤 Sent KeepAlive (Triggering Burst)...");

    // 6. LISTEN FOR BURST
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    loop {
        match socket.recv(&mut buf) {
            Ok(n) => {
                let op = (buf[0] as u16) << 8 | buf[1] as u16;
                println!("📥 RCVD PACKET [Op: {:#04x}] Len: {}", op, n);
                // Print the first 16 bytes of the payload to check formatting
                if n > 4 {
                    print!("   Payload Header: ");
                    print_hex(&buf[4..std::cmp::min(n, 20)]);
                }
            },
            Err(_) => {
                println!("🛑 Stream ended (Timeout).");
                break;
            }
        }
    }
}

fn print_hex(data: &[u8]) {
    for b in data {
        print!("{:02x} ", b);
    }
    println!("");
}
