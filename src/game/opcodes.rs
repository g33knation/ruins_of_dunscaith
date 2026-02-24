/// OpCodes for the Rain of Fear 2 (RoF2) Client.
/// Values derived from patch_RoF2.conf and packet logs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OpCode {
    /// Client initiates connection
    SessionRequest = 0x0001,
    
    /// Server accepts connection, provides CRC Key
    SessionResponse = 0x0002,

    /// Client Keep-Alive / Packet Status
    /// Often sent to confirm connection stability before Login.
    PacketStatus = 0x0009,

    /// Client Disconnect
    /// Sent if the handshake fails or CRC key mismatches.
    Disconnect = 0x0015,

    /// Client Request: Character Info / Profile
    /// Hex: 0xD083
    CharInfo = 0xD083,

    /// Server Response: Character Info / Profile
    /// Hex: 0x2409
    SendCharInfo = 0x2409,
    
    /// Client requests to login
    Login = 0x3E91, 
    
    /// Server accepts login
    LoginAccepted = 0x5123,
}

impl OpCode {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(OpCode::SessionRequest),
            0x0002 => Some(OpCode::SessionResponse),
            0x0009 => Some(OpCode::PacketStatus),
            0x0015 => Some(OpCode::Disconnect),
            0xD083 => Some(OpCode::CharInfo),
            0x2409 => Some(OpCode::SendCharInfo),
            0x3E91 => Some(OpCode::Login),
            0x5123 => Some(OpCode::LoginAccepted),
            _ => None,
        }
    }
}