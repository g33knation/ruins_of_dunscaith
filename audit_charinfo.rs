use std::io::Write;

fn main() {
    let mut data: Vec<u8> = Vec::new();
    
    // Helper to write u8
    fn write_u8(vec: &mut Vec<u8>, val: u8) {
        vec.push(val);
    }
    
    // Helper to write u16 le
    fn write_u16(vec: &mut Vec<u8>, val: u16) {
        vec.extend_from_slice(&val.to_le_bytes());
    }
    
    // Helper to write u32 le
    fn write_u32(vec: &mut Vec<u8>, val: u32) {
        vec.extend_from_slice(&val.to_le_bytes());
    }

    // Simulate one character
    println!("Offset | Field | Size | Value");
    println!("-------|-------|------|------");
    
    // Count
    let start = data.len();
    write_u32(&mut data, 1);
    println!("{:04} | Count | 4 | 1", start);
    
    // Character Entry Start
    let char_start = data.len();
    println!("--- Character Entry Start ---");
    
    // Name (64 bytes)
    let name_start = data.len();
    let name = "DebugChar";
    let mut name_bytes = [0u8; 64];
    let name_slice = name.as_bytes();
    let copy_len = name_slice.len().min(63);
    // Manual copy
    for i in 0..copy_len {
        name_bytes[i] = name_slice[i];
    }
    data.extend_from_slice(&name_bytes);
    println!("{:04} | Name | 64 | {}", name_start, name);
    
    // Class
    println!("{:04} | Class | 1 | 1", data.len());
    write_u8(&mut data, 1);
    
    // Race
    println!("{:04} | Race | 4 | 1", data.len());
    write_u32(&mut data, 1);
    
    // Level
    println!("{:04} | Level | 1 | 1", data.len());
    write_u8(&mut data, 1);
    
    // Class (Redundant)
    println!("{:04} | Class2 | 1 | 1", data.len());
    write_u8(&mut data, 1);
    
    // Race (Redundant)
    println!("{:04} | Race2 | 4 | 1 (Wait, code says u8?)", data.len());
    // WARNING: Code has data.put_u8(ch.race as u8); at line 398 in packets.rs
    // Wait, let's verify what the CODE actually does.
    // Line 398: data.put_u8(ch.race as u8);
    // Line 392: data.put_u32_le(ch.race as u32);
    // So Race1 is u32, Race2 is u8.
    write_u8(&mut data, 1);
    
    // Zone ID
    println!("{:04} | ZoneID | 2 | 202", data.len());
    write_u16(&mut data, 202);
    
    // Zone Instance
    println!("{:04} | Instance | 2 | 0", data.len());
    write_u16(&mut data, 0);
    
    // Gender
    println!("{:04} | Gender | 1 | 0", data.len());
    write_u8(&mut data, 0);
    
    // Face
    println!("{:04} | Face | 1 | 0", data.len());
    write_u8(&mut data, 0);
    
    // Equipment (9 slots * 24 bytes)
    println!("{:04} | Equipment | 216 | 0s", data.len());
    for _ in 0..9 {
        write_u32(&mut data, 0); // Material
        write_u32(&mut data, 0); 
        write_u32(&mut data, 0);
        write_u32(&mut data, 0);
        write_u32(&mut data, 0);
        write_u32(&mut data, 0); // Color
    }
    
    // Unknown50 (u32[2])
    println!("{:04} | Unk50 | 8 | 0s", data.len());
    write_u32(&mut data, 0);
    write_u32(&mut data, 0);
    
    // Deity
    println!("{:04} | Deity | 4 | 212", data.len());
    write_u32(&mut data, 212);
    
    // IDs, Times
    println!("{:04} | IDs/Times | 32 | 0s (8x u32)", data.len());
    for _ in 0..8 {
         write_u32(&mut data, 0);
    }
    
    // Enabled
    println!("{:04} | Enabled | 1 | 1", data.len());
    write_u8(&mut data, 1);
    
    // Tutorial
    println!("{:04} | Tutorial | 1 | 0", data.len());
    write_u8(&mut data, 0);
    
    // Unknown48
    println!("{:04} | Unk48 | 4 | 0", data.len());
    write_u32(&mut data, 0);
    
    // Heroic
    println!("{:04} | Heroic | 4 | 0", data.len());
    write_u32(&mut data, 0);
    
    // Unk50 (u32[2])
    println!("{:04} | Unk50_2 | 8 | 0", data.len());
    write_u32(&mut data, 0);
    write_u32(&mut data, 0);
    
    // GoHome
    println!("{:04} | GoHome | 4 | 0", data.len());
    write_u32(&mut data, 0);
    
    // Appearance (Hair/Face/Etc - 9 fields * 4 bytes)
    println!("{:04} | Appearance | 36 | 0s", data.len());
    for _ in 0..9 {
        write_u32(&mut data, 0);
    }
    
    // Unk51
    println!("{:04} | Unk51 | 4 | 0", data.len());
    write_u32(&mut data, 0);
    
    // SecIDFile
    println!("{:04} | SecIDFile | 4 | 0", data.len());
    write_u32(&mut data, 0);
    
    let current_size = data.len() - char_start;
    println!("--- Entry End (Pre-Pad) ---");
    println!("Current Entry Size: {}", current_size);
    
    let target = 3336;
    println!("Target Size: {}", target);
    println!("Padding Needed: {}", target - current_size);
}
