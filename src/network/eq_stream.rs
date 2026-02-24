use flate2::read::ZlibDecoder;
use std::io::Read;

pub struct EQStream {
    pub key: u32,
}

impl EQStream {
    pub fn new(key: u32) -> Self {
        Self { key }
    }

    // Rotates the key (EQ's "Rolling XOR" encryption)
    fn update_key(&mut self, length: usize) {
        let mut key = self.key;
        for _ in 0..length {
            let temp = (key.rotate_left(5) as u64 | (key >> 27) as u64) as u32; // Emulates EQ's weird cast
            key = (temp.wrapping_add(key)) ^ 0x31415926; // Pi constant
        }
        self.key = key;
    }

    pub fn decode(&mut self, packet: &[u8]) -> Option<Vec<u8>> {
        // 1. Check Protocol (RoF2 usually sends 0x00 at the start of protocol packets)
        let mut offset = 0;
        if packet.len() > 0 && packet[0] == 0x00 {
            offset = 1; // Skip protocol byte
        }

        if offset >= packet.len() {
             return None;
        }

        let mut data = packet[offset..].to_vec();

        // 2. Decompress (if flag 0x5a is present, it's compressed)
        // Note: For the handshake login, we usually assume compressed if we told them 0x02
        if data.len() > 1 && data[0] == 0x5a {
             let mut decoder = ZlibDecoder::new(&data[1..]);
             let mut decompressed = Vec::new();
             if decoder.read_to_end(&mut decompressed).is_ok() {
                 data = decompressed;
             }
        }

        // 3. Decrypt (XOR)
        // We use the rolling key we generated in the handshake
        // (This is a simplified XOR for the login phase)
        // In full World implementation, this is more complex, but for Login, 
        // usually we just strip the opcode.
        
        // FOR NOW: Just return the data so we can SEE what opcode 0x03 looks like.
        // The Login Server handshake is actually unencrypted usually, just compressed.
        Some(data)
    }
}
