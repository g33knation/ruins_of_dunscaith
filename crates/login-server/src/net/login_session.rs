use crate::net::client_socket::ClientSocket;
use anyhow::{Context, Result};
use binrw::BinWrite; // <--- MUST BE USED
use shared::opcodes::OpCode;
use tracing::{debug, error, info, instrument, warn};

// --- Tier 1: Data (State) ---
// ... (rest of Tier 1)
#[derive(Debug, PartialEq)]
pub enum ClientState {
    Connected,
    Authenticating,
    Authorized,
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState::Connected
    }
}

#[derive(Debug, Default)]
pub struct LoginSessionData {
    pub username: String,
    pub is_authenticated: bool,
    pub state: ClientState,
}

// --- Tier 2: Protocol Structures ---
// --- Tier 2: Protocol Structures ---
#[derive(BinWrite, Debug)]
#[bw(little)] // Explicit little endian for body data
pub struct SessionReadyBody {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct LoginResponseBody {
    pub account_id: u32,       // Any non-zero ID indicates success, 0 indicates failure
    pub session_key: [u8; 10], // 10 bytes of session data
    pub error_code: u16,       // 0 = Success, 13 = Invalid Password
}

// --- Tier 3: The Actor (Logic) ---
pub struct LoginSessionActor {
    data: LoginSessionData,
    socket: ClientSocket,
    pool: Option<sqlx::PgPool>,
}

impl LoginSessionActor {
    pub fn new(socket: ClientSocket, pool: Option<sqlx::PgPool>) -> Self {
        Self {
            data: LoginSessionData::default(),
            socket,
            pool,
        }
    }

    #[instrument(skip(self), name = "session_loop")]
    pub async fn run(mut self) -> Result<()> {
        info!("Session Actor Started.");

        // 1. [FIX] Send Sanitized OP_SessionReady
        self.send_session_ready().await
            .context("Failed to send initial handshake")?;

        loop {
            match self.socket.read_packet().await {
                Ok((opcode, payload)) => {
                    info!("TCP RX: OpCode=0x{:04X}, Payload={} bytes", opcode as u16, payload.len());
                    if !payload.is_empty() {
                        debug!("TCP RX hex: {:02X?}", &payload[..payload.len().min(64)]);
                    }
                    if let Err(e) = self.handle_packet(opcode, payload).await {
                        error!("Error: {}", e);
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
    }

    async fn send_session_ready(&mut self) -> Result<()> {
        let opcode = OpCode::SessionReady;
        let payload = []; // Empty body is standard for "Ready for login"
        
        // Use the centralized send_raw to ensure correct Size (BE) and OpCode (LE) framing
        self.socket.send_raw(opcode, payload.to_vec()).await?;
        info!("TCP: Sent {:?}", opcode);
        
        Ok(())
    }

    async fn handle_packet(&mut self, opcode: OpCode, payload: Vec<u8>) -> Result<()> {
        match opcode {
            OpCode::Login => {
                // RoF2 Login Payload: [Username\0][Password\0][Rest...]
                let parts: Vec<&[u8]> = payload.split(|&b| b == 0).collect();
                
                let username = parts.get(0).map(|b| String::from_utf8_lossy(b)).unwrap_or_default().to_string();
                let password = parts.get(1).map(|b| String::from_utf8_lossy(b)).unwrap_or_default().to_string();

                info!("--- TCP LOGIN ATTEMPT ---");
                self.data.state = ClientState::Authenticating;

                // Query Database
                let row = if let Some(pool) = &self.pool {
                    sqlx::query("SELECT id, password FROM account WHERE name = $1")
                        .bind(&username)
                        .fetch_optional(pool)
                        .await?
                } else {
                    // MOCK MODE: Always allow testuser/testpass
                    if username == "testuser" {
                        info!("MOCK LOGIN: Auto-approving testuser");
                        // We need a way to mock the row. Since sqlx doesn't make it easy to create a Row manually,
                        // we'll just handle the logic directly here.
                        None 
                    } else {
                        None
                    }
                };

                let mut error_code = 13; // Default to Error
                let mut account_id_out = 0;

                if self.pool.is_none() && username == "testuser" && password == "testpass" {
                    // Hardcoded success for mock mode
                    account_id_out = 1;
                    error_code = 0;
                    self.data.is_authenticated = true;
                    self.data.username = username.clone();
                    self.data.state = ClientState::Authorized;
                } else if let Some(row) = row {
                    use sqlx::Row;
                    let db_pass: String = row.get("password");
                    account_id_out = row.get("id");

                    let is_valid = match shared::crypto::verify_password(&password, &db_pass) {
                         Ok(v) => v,
                         Err(e) => {
                             error!("Argon2 Verification Error: {}", e);
                             false
                         }
                    };

                    if is_valid {
                         info!("TCP Authentication Success for {}", username);
                         self.data.is_authenticated = true;
                         self.data.username = username;
                         self.data.state = ClientState::Authorized;
                         error_code = 0; // Success
                    } else {
                         warn!("TCP Authentication Failed: Invalid password for {}", username);
                         error_code = 13; // Invalid Password
                    }
                } else {
                    warn!("TCP Authentication Failed: User {} not found", username);
                    error_code = 13; // Invalid Password (Keep generic for security)
                }

                let mut session_key = *b"0000000000";
                if error_code == 0 {
                    use rand::Rng;
                    let key_val: u32 = rand::thread_rng().gen();
                    let key_str = format!("{:010}", key_val);
                    session_key.copy_from_slice(key_str.as_bytes());

                    // Save to DB for World server verification
                    if let Some(pool) = &self.pool {
                        let _ = sqlx::query("UPDATE account SET ls_session_key = $1 WHERE id = $2")
                            .bind(&key_str)
                            .bind(account_id_out)
                            .execute(pool)
                            .await;
                    }
                }

                let response = LoginResponseBody {
                    account_id: if error_code == 0 { account_id_out as u32 } else { 0 },
                    session_key,
                    error_code,
                };

                self.socket.send_packet(OpCode::LoginApproval, &response).await?;
                if error_code == 0 {
                    info!("Approval sent (0x0017)! Client should now request the server list.");
                } else {
                    info!("TCP Login Rejection sent (Code={}).", error_code);
                }
                Ok(())
            },
            OpCode::ServerListRequest => {
                info!("Success! Client is requesting the Server List.");
                self.send_server_list().await?;
                Ok(())
            },
            OpCode::Unknown if opcode as u16 == 0 => {
                // Ignore Heartbeat / SpellChecksum
                Ok(())
            }
            _ => {
                debug!("Unhandled OpCode: 0x{:04X}", opcode as u16);
                Ok(())
            }
        }
    }

    async fn send_server_list(&mut self) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        info!("Sending Server List (Modern Akk-Stack RoF2 Style)...");

        let mut payload = Vec::new();

        // 1. Header (Akk-Stack/Modern RoF2 uses u32 Admin, u32 Count)
        payload.write_u32::<LittleEndian>(0)?; // Admin Flag (0)
        payload.write_u32::<LittleEndian>(1)?; // Server Count (1)

        // 2. Server Entry Entries
        let server_ip = std::env::var("PUBLIC_IP").unwrap_or("127.0.0.1".to_string());
        let server_name = "Ruins of Dunscaith";
        
        // IPLen (u32) + IP String + Null
        payload.write_u32::<LittleEndian>(server_ip.len() as u32 + 1)?;
        payload.extend_from_slice(server_ip.as_bytes());
        payload.push(0);

        // Server ID (u32)
        payload.write_u32::<LittleEndian>(1)?;

        // Status (i32) -> 1 = Up
        payload.write_i32::<LittleEndian>(1)?;

        // Players (i32)
        payload.write_i32::<LittleEndian>(0)?;

        // NameLen (u32) + Name String + Null
        payload.write_u32::<LittleEndian>(server_name.len() as u32 + 1)?;
        payload.extend_from_slice(server_name.as_bytes());
        payload.push(0);
        
        // Note: Some RoF2 offsets expect a trailing u8 for "Green/Locked" but most ignore it.
        payload.push(0); 

        // Send Packet OP_ServerListResponse
        self.socket.send_packet(OpCode::ServerListResponse, &payload).await?;
        info!("Sent OP_ServerListResponse to client.");
        Ok(())
    }
}
