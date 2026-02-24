use binrw::{BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct PacketHeader {
    pub size: u16,
    pub opcode: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[br(repr = u16)]
#[bw(repr = u16)]
pub enum LoginOpCode {
    SessionReady = 0x0001,
    Login = 0x0002,
    ServerListRequest = 0x0004,
    PlayEverquestRequest = 0x000d,
    LoginAccepted = 0x0017,
    ServerListResponse = 0x0018,
    PlayEverquestResponse = 0x0021,
    Unknown(u16),
}

