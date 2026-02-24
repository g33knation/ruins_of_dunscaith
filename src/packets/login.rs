use binrw::BinRead;

/// Represents the C++ struct for OP_Login.
/// Typically fixed-size char arrays in EQEmulator.
#[derive(Debug, BinRead)]
#[br(little)]
pub struct LoginPacket {
    // char name[64];
    // We read as bytes, then access via helper to strip nulls/convert
    pub name_bytes: [u8; 64],
    
    // char password[64];
    pub password_bytes: [u8; 64],
    
    // int protocol_version;
    pub protocol_version: i32,
}

impl LoginPacket {
    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.name_bytes)
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn get_password(&self) -> String {
        String::from_utf8_lossy(&self.password_bytes)
            .trim_matches(char::from(0))
            .to_string()
    }
}
