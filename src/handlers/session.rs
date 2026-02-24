use std::sync::Arc;
use tokio::net::UdpSocket;

pub async fn handle_session_request(
    buf: &[u8],
    addr: std::net::SocketAddr,
    shared_socket: &Arc<UdpSocket>,
) {
    if buf.len() < 10 { return; }
    
    let session_id_bytes = &buf[6..10]; 
    println!("🤝 Handshake: Processing Request from {}", addr);

    let crc_key: u32 = 0x42424242; 

    let mut response = Vec::with_capacity(15);
    response.push(0x00); response.push(0x02); // OpCode
    response.extend_from_slice(session_id_bytes);
    response.extend_from_slice(&crc_key.to_le_bytes());
    response.push(0x02); // Compressed Flag

    // 🛑 CRITICAL: BIG BUFFER MODE (131,072 Bytes)
    // [00 00 02 00]
    // The "512 LE" mode [00 02 00 00] caused the silence.
    response.push(0x00);
    response.push(0x00);
    response.push(0x02);
    response.push(0x00);

    match shared_socket.send_to(&response, addr).await {
        Ok(_) => println!("📤 Sent RoF2 Response (Big Buffer Mode)"),
        Err(e) => eprintln!("❌ Failed to send: {}", e),
    }
}