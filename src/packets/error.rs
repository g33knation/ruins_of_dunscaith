use binrw;
use std::fmt;

#[derive(Debug)]
pub enum PacketError {
    BinRw(binrw::Error),
    UnknownOpCode(u16),
    HandlerError(String),
}

impl From<binrw::Error> for PacketError {
    fn from(e: binrw::Error) -> Self {
        PacketError::BinRw(e)
    }
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketError::BinRw(e) => write!(f, "Binary Read/Write Error: {}", e),
            PacketError::UnknownOpCode(op) => write!(f, "Unknown OpCode: {:#04x}", op),
            PacketError::HandlerError(msg) => write!(f, "Handler Error: {}", msg),
        }
    }
}

impl std::error::Error for PacketError {}
