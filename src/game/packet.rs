use byteorder::{LittleEndian, WriteBytesExt};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

pub enum Protocol {
    OpCode(u16),
    Combined,
}

pub struct EQPacket {
    pub opcode: u16,
    pub payload: Vec<u8>,
    pub compressed: bool,
    pub encrypted: bool,
}

impl EQPacket {
    pub fn new(opcode: u16, payload: Vec<u8>) -> Self {
        Self {
            opcode,
            payload,
            compressed: true, // RoF2 defaults to compressed
            encrypted: false, // Login is usually unencrypted, Game is encrypted
        }
    }

    /// Packs the raw data into the format RoF2 expects (OpCode + Compression)
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // 1. Write OpCode (Little Endian)
        buffer.write_u16::<LittleEndian>(self.opcode).unwrap();

        // 2. Compression Logic
        if self.compressed && self.payload.len() > 16 {
            // RoF2 Compression Header: 0x5a 0x01 (zlib)
            buffer.write_u16::<LittleEndian>(0x015a).unwrap();
            
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&self.payload).unwrap();
            let compressed_data = encoder.finish().unwrap();
            
            buffer.extend_from_slice(&compressed_data);
        } else {
            buffer.extend_from_slice(&self.payload);
        }

        buffer
    }
}
