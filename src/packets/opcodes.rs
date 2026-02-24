use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, FromPrimitive, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OpCode {
    // Session / Login
    LoginRequest = 0x0001, // Placeholder, need verification
    
    // Character
    SendCharInfo = 0x2409, // From char_select.rs
    
    // Zone
    ZoneChange = 0x2284, // Example, need verification
    
    // Common
    OP_AckPacket = 0x0000,
}

impl OpCode {
    pub fn from_u16_safe(v: u16) -> Option<Self> {
        FromPrimitive::from_u16(v)
    }
}
