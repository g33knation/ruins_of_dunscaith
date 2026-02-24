use crc::{Crc, CRC_32_ISO_HDLC};

pub const EQ_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

fn main() {
    let opcode: u16 = 0x0009;
    let seq: u16 = 0x0000;
    let key: u32 = 0;
    
    let mut digest = EQ_CRC.digest();
    digest.update(&opcode.to_be_bytes()); // 00 09
    digest.update(&seq.to_be_bytes()); // 00 00
    digest.update(&key.to_be_bytes()); // 00 00 00 00
    
    let crc32 = digest.finalize();
    let crc16 = (crc32 & 0xFFFF) as u16;
    
    println!("CRC for Op=0009 Seq=0000 Key=0 is {:02x?} (LE: {:02x} {:02x})", crc16, crc16 as u8, (crc16 >> 8) as u8);
}
