use binrw::{BinRead, BinWrite, NullString};

// --- Helper Functions ---

pub fn clean_string(bytes: &[u8]) -> String {
    use std::ffi::CStr;
    match CStr::from_bytes_until_nul(bytes) {
        Ok(cstr) => cstr.to_string_lossy().into_owned(),
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    }
}

pub fn string_to_array<const N: usize>(s: &String) -> [u8; N] {
    let mut arr = [0u8; N];
    let bytes = s.as_bytes();
    let len = bytes.len().min(N);
    arr[..len].copy_from_slice(&bytes[..len]);
    arr
}

// --- Types ---

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct SessionKey(pub [u8; 30]);

impl SessionKey {
    pub fn new(bytes: [u8; 30]) -> Self {
        SessionKey(bytes)
    }
}

// --- EQEmu Login Protocol Structures ---
// Based on EQEmu's loginserver/login_types.h

/// Base header in ALL login packets (10 bytes packed)
/// sequence: 2 = handshake, 3 = login, 4 = serverlist
#[derive(Debug, Clone, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct LoginBaseMessage {
    pub sequence: i32,      // 4 bytes
    pub compressed: u8,     // 1 byte (bool)
    pub encrypt_type: i8,   // 1 byte
    pub unk3: i32,          // 4 bytes
}

/// Reply message structure
#[derive(Debug, Clone, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct LoginBaseReplyMessage {
    pub success: u8,        // 1 byte (bool: 0 = failure, 1 = success)
    pub error_str_id: i32,  // 4 bytes (101 = "No Error")
}

/// TCP Handshake Response - sent in response to OP_SessionReady
/// Server sends OP_ChatMessage containing this struct
#[derive(Debug, Clone, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct LoginHandShakeReply {
    pub base_header: LoginBaseMessage,   // sequence = 0x02
    pub base_reply: LoginBaseReplyMessage, // success = true, error_str_id = 101
}

impl LoginHandShakeReply {
    /// Create a successful handshake reply
    pub fn success() -> Self {
        Self {
            base_header: LoginBaseMessage {
                sequence: 0x02,      // Handshake sequence
                compressed: 0,       // Not compressed
                encrypt_type: 0,     // No encryption
                unk3: 0,             // Unused
            },
            base_reply: LoginBaseReplyMessage {
                success: 1,          // Success
                error_str_id: 101,   // "No Error"
            },
        }
    }
}

// --- Packets ---

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
pub struct LoginRequest {
    #[br(map = |bytes: [u8; 30]| clean_string(&bytes))]
    #[bw(map = |s: &String| string_to_array::<30>(s))]
    pub username: String,

    #[br(map = |bytes: [u8; 30]| clean_string(&bytes))]
    #[bw(map = |s: &String| string_to_array::<30>(s))]
    pub password: String,

    pub client_version: u32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct LoginResponse {
    pub result: u32,
    pub account_id: u32,
    pub session_key: SessionKey,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
pub struct ServerListRequest;

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
pub struct LoginClientServerData {
    #[br(map = |s: NullString| String::from_utf8_lossy(&s.to_vec()).to_string())] 
    #[bw(map = |s: &String| NullString::from(s.as_str()))] 
    pub ip: String,
    
    pub server_type: i32,
    pub server_id: i32,
    
    #[br(map = |s: NullString| String::from_utf8_lossy(&s.to_vec()).to_string())]
    #[bw(map = |s: &String| NullString::from(s.as_str()))]
    pub server_name: String,

    #[br(map = |s: NullString| String::from_utf8_lossy(&s.to_vec()).to_string())]
    #[bw(map = |s: &String| NullString::from(s.as_str()))]
    pub country_code: String,
    
    pub language_code: [u8; 1],
    pub server_status: i32,
    pub player_count: i32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct ServerListResponse {
    pub server_count: u32,
    
    #[br(count = server_count, assert(server_count < 1000))]
    pub servers: Vec<LoginClientServerData>,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
pub struct PlayRequest {
    pub server_id: u32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct PlayResponse {
    #[br(map = |bytes: [u8; 16]| clean_string(&bytes))]
    #[bw(map = |s: &String| string_to_array::<16>(s))]
    pub server_ip: String,

    pub session_key: u32,
    pub success: u32, 
}
