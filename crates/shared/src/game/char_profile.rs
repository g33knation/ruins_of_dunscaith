use binrw::{BinRead, BinWrite};
use crate::db::Character;

/// Represents the CharProfile struct used in OP_CharInfo.
/// Maps DB types to packet types (often smaller or fixed-size).
#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct CharProfilePacket {
    pub name: [u8; 64],
    pub last_name: [u8; 32],
    pub char_id: u32,
    pub level: u8,
    pub class: u8,
    pub race: u32,
    pub gender: u8,
    pub pad1: u8,
    pub deity: u32,
    pub face: u32,
    pub hair_color: u32,
    pub hair_style: u32,
    pub beard: u32,
    pub beard_color: u32,
    pub eye_color_1: u32,
    pub eye_color_2: u32,
    pub drakkin_heritage: u32,
    pub drakkin_tattoo: u32,
    pub drakkin_details: u32,
    pub zone_id: u32,
    pub zone_instance: u32,
    pub y: f32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub tutorial: u32,
    pub return_home: u32,
    pub pad2: [u8; 5632], // 184 + 5632 = 5816 (RoF2 Standard PlayerProfile size)
}

impl From<Character> for CharProfilePacket {
    fn from(c: Character) -> Self {
        Self {
            name: string_to_fixed_array(&Some(c.name)),
            last_name: string_to_fixed_array(&c.last_name),
            char_id: c.id as u32,
            level: c.level as u8,
            class: c.class as u8,
            race: c.race as u32,
            gender: c.gender as u8,
            pad1: 0,
            deity: 0,
            face: c.face as u32,
            hair_color: c.hair_color as u32,
            hair_style: c.hair_style as u32,
            beard: c.beard as u32,
            beard_color: c.beard_color as u32,
            eye_color_1: c.eye_color_1 as u32,
            eye_color_2: c.eye_color_2 as u32,
            drakkin_heritage: c.drakkin_heritage as u32,
            drakkin_tattoo: c.drakkin_tattoo as u32,
            drakkin_details: c.drakkin_details as u32,
            zone_id: c.zone_id as u32,
            zone_instance: c.zone_instance as u32,
            y: c.y,
            x: c.x,
            z: c.z,
            heading: c.heading,
            tutorial: 0,
            return_home: 0,
            pad2: [0u8; 5632],
        }
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
