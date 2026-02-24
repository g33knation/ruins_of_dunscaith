use binrw::{BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite)]
#[br(big)]
#[bw(big)]
pub struct PacketHeader {
    pub size: u16,
    pub opcode: u16,
}
