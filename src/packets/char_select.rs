use binrw::BinWrite;
use crate::game::packet::EQPacket;
use crate::packets::char_profile::CharProfilePacket;
use std::sync::Arc;
use tokio::net::UdpSocket;
use std::io::Cursor;

const OP_SEND_CHAR_INFO: u16 = 0x2409; // The Server Response OpCode

pub async fn handle(
    socket: Arc<UdpSocket>,
    addr: std::net::SocketAddr,
    _pool: Arc<sqlx::MySqlPool>,
    _account_id: i32,
) {
    // 1. HARDCODE CHARACTER (For Speed)
    // We skip the DB for now and just return a "Level 1 Warrior" named "Hero".
    
    let char_data = CharProfilePacket {
        name: string_to_fixed_array(&Some("Hero".to_string())),
        last_name: string_to_fixed_array(&Some("OfNorrath".to_string())),
        gender: 0, // Male
        race: 1,   // Human
        class: 1,  // Warrior
        level: 1,
        exp: 0,
        practice_points: 0,
        mana: 0,
        cur_hp: 100,
        endurance: 0,
        str: 75,
        sta: 75,
        cha: 75,
        dex: 75,
        int: 75,
        agi: 75,
        wis: 75,
        face: 0,
        hair_style: 0,
        hair_color: 0,
        beard: 0,
        beard_color: 0,
        eye_color_1: 0,
        eye_color_2: 0,
        drakkin_heritage: 0,
        drakkin_tattoo: 0,
        drakkin_details: 0,
        zone_id: 202, // Poknowledge
        zone_instance: 0,
        y: 0.0,
        x: 0.0,
        z: 0.0,
        heading: 0.0,
    };

    println!("Found character: {} (Mock)", "Hero");

    // 3. Serialize (Struct -> Bytes)
    let mut writer = Cursor::new(Vec::new());
    if let Err(e) = char_data.write(&mut writer) {
        eprintln!("Failed to serialize CharInfo: {}", e);
        return;
    }

    // 4. Wrap in EQPacket for RoF2 Protocol (Compression)
    let payload = writer.into_inner();
    let eq_packet = EQPacket::new(OP_SEND_CHAR_INFO, payload);

    // 5. Send back to client
    if let Err(e) = socket.send_to(&eq_packet.serialize(), addr).await {
        eprintln!("Failed to send CharInfo packet: {}", e);
    } else {
        println!("✅ Sent CharInfo [0x2409] (Wrapped & Compressed) to {}", addr);
    }
}

/// Helper to convert Option<String> to fixed-size byte array.
/// Truncates if string is too long. Pads with nulls.
fn string_to_fixed_array<const N: usize>(s: &Option<String>) -> [u8; N] {
    let mut arr = [0u8; N];
    if let Some(str_val) = s {
        let bytes = str_val.as_bytes();
        let len = bytes.len().min(N);
        arr[0..len].copy_from_slice(&bytes[0..len]);
    }
    arr
}
