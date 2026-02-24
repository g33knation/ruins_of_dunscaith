use binrw::{BinRead, BinWrite, NullString};

// --- Helper Functions ---

pub fn clean_string(bytes: &[u8]) -> String {
    use std::ffi::CStr;
    // CStr::from_bytes_until_nul checks for nulls. If none, it takes the whole slice as per previous logic? 
    // Wait, from_bytes_until_nul returns Error if no null.
    // The previous logic was "unwrap_or(bytes.len())".
    // We should fallback to full slice if from_bytes_until_nul fails due to missing null.
    
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
    #[br(map = |s: NullString| String::from_utf8_lossy(&s.into_vec()).to_string())] 
    #[bw(map = |s: &String| NullString::from(s.as_str()))] 
    pub ip: String,
    
    pub server_type: i32,
    pub server_id: i32,
    
    #[br(map = |s: NullString| String::from_utf8_lossy(&s.into_vec()).to_string())]
    #[bw(map = |s: &String| NullString::from(s.as_str()))]
    pub server_name: String,

    #[br(map = |s: NullString| String::from_utf8_lossy(&s.into_vec()).to_string())]
    #[bw(map = |s: &String| NullString::from(s.as_str()))]
    pub country_code: String,
    
    pub language_code: [u8; 1],
    pub server_status: i32,
    pub player_count: i32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
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
pub struct PlayResponse {
    #[br(map = |bytes: [u8; 16]| clean_string(&bytes))]
    #[bw(map = |s: &String| string_to_array::<16>(s))]
    pub server_ip: String,

    pub session_key: u32,
    pub success: u32, 
}
