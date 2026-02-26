use crc::{Crc, CRC_32_ISO_HDLC};

pub const EQ_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

fn main() {
    let crc_key: u32 = 0;
    let opcode: u16 = 0x0009;
    
    let mut digest = EQ_CRC.digest();
    digest.update(&crc_key.to_le_bytes());
    digest.update(&opcode.to_be_bytes());
    
    let crc32 = digest.finalize();
    let crc16 = (crc32 & 0xFFFF) as u16;
    
    println!("CRC16 for Key 0, Op 0x09: {:04x}", crc16);
}
