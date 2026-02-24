use binrw::{BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite)]
#[br(big)]
#[bw(big)]
pub struct PacketHeader {
    pub opcode: u16,
    pub size: u16, 
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum LoginOpCode {
    SessionReady = 0x0001,
    Login = 0x0002,
    ServerListRequest = 0x0004,
    PlayEverquestRequest = 0x000d,
    PlayEverquestResponse = 0x0021,
    LoginAccepted = 0x0017,
    ServerListResponse = 0x0018,
    Unknown = 0xFFFF,
}

impl From<u16> for LoginOpCode {
    fn from(code: u16) -> Self {
        match code {
            0x0001 => LoginOpCode::SessionReady,
            0x0002 => LoginOpCode::Login,
            0x0004 => LoginOpCode::ServerListRequest,
            0x000d => LoginOpCode::PlayEverquestRequest,
            0x0021 => LoginOpCode::PlayEverquestResponse,
            0x0017 => LoginOpCode::LoginAccepted,
            0x0018 => LoginOpCode::ServerListResponse,
            _ => LoginOpCode::Unknown,
        }
    }
}
