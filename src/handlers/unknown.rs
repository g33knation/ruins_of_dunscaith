/// Action to take after handling the unknown packet
pub enum PacketAction {
    Ignore,
    Disconnect,
}

/// Handles unknown OpCodes to prevent server crashes or silent failures.
/// 
/// Logging these packets is crucial for reverse engineering the RoF2 protocol quirks,
/// especially during the initial handshake and login sequence where undocumented
/// packets (like 0x500) might appear.
pub fn handle_unknown_packet(opcode: u16, data: &[u8]) -> PacketAction {
    println!("⚠️ UNKNOWN OPCODE: {:#06X} | Length: {}", opcode, data.len());
    
    // Log the first 16 bytes for quick debugging
    if !data.is_empty() {
        let preview = data.iter().take(16).map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
        println!("   Raw Data (First 16): [{}]", preview);
    }

    match opcode {
        // OpCode 0x0500 detection
        // In the context of RoF2 connection issues, 0x0500 appearing with SessionID data
        // usually implies a SessionDisconnect or fatal protocol error sent by the client.
        // The client often sends this when it fails to decode the SessionResponse correctly.
        0x0500 => {
            println!("   -> CRITICAL: OpCode 0x0500 detected.");
            println!("   -> ANALYSIS: This packet often signifies a client-side fatal error (e.g., CRC Key mismatch).");
            println!("   -> RECOMMENDATION: Check SessionResponse structure alignment (14 bytes) and CRC Key generation.");
            PacketAction::Disconnect
        },
        _ => {
            // By default, we ignore unknown packets to keep the connection alive
            // unless they are known to be fatal.
            PacketAction::Ignore
        }
    }
}
