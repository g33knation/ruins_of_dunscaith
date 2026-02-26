use bytes::BufMut;
use shared::net::eq_stream::{EqStreamSession as SharedSession, SessionRequest, ProcessPacketResult};
use shared::opcodes::OpCode;
use rand;

pub struct EqStreamSession {
    session: SharedSession,
    pool: Option<sqlx::PgPool>,
}

impl EqStreamSession {
    pub fn new(session_id: u32, pool: Option<sqlx::PgPool>) -> Self {
        let mut session = SharedSession::new(session_id);
        session.crc_key = 0; // Force 0 for Login Server (Zero-Key DES)
        session.enable_combined(); // RoF2 stability
        Self {
            session,
            pool,
        }
    }

    pub fn handle_session_request(&mut self, req: &SessionRequest) -> Vec<u8> {
        self.session.handle_session_request(req)
    }

    pub async fn receive_packet(&mut self, opcode: u16, payload: &[u8]) -> Vec<Vec<u8>> {
        let results = self.session.process_packet(opcode, payload);
        let mut responses = Vec::new(); // Wire responses (ACKs, etc)
        
        for res in results {
            match res {
                ProcessPacketResult::Response(pkt) => {
                    responses.push(pkt);
                },
                ProcessPacketResult::Application(app_op, app_data) => {
                     // Detect and handle RoF2 Zlib Compression (5a 01)
                    let decompressed = match SharedSession::decompress_payload(app_data).await {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::error!("Decompression failure: {}", e);
                            continue;
                        }
                    };

                    if let Some(replies) = self.handle_app_packet(app_op, &decompressed).await {
                        responses.extend(replies);
                    }
                }
            }
        }
        responses
    }
    
    async fn handle_app_packet(&mut self, app_opcode: OpCode, payload: &[u8]) -> Option<Vec<Vec<u8>>> {
        match app_opcode {
            OpCode::SessionReady => { // 0x0001
                tracing::info!("Handling OP_SessionReady, sending HandshakeReply + StatRequest");
                let mut replies = Vec::new();
                replies.extend(self.create_handshake_reply().await);     
                replies.push(self.session.create_stat_request());      
                Some(replies)
            },
            OpCode::Login => { // OP_Login (RoF2 uses 0x0002)
                tracing::info!("Received OP_Login (Op={:?})", app_opcode);
                self.handle_login(payload).await
            },
            OpCode::ServerListRequest => { // 0x0005
                tracing::info!("Received OP_ServerListRequest");
                let seq = if payload.len() >= 4 {
                    u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                } else { 0 };
                let mut list_replies = Vec::new();
                list_replies.extend(self.create_server_list_response_with_seq(seq).await);
                Some(list_replies)
            },
            OpCode::PlayEverquestRequest => { // 0x000d
                tracing::info!("Received OP_PlayEverquestRequest");
                if payload.len() < 14 { 
                    tracing::warn!("OP_PlayEverquestRequest too short");
                    return None; 
                }
                let sequence = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let server_number = u32::from_le_bytes([payload[10], payload[11], payload[12], payload[13]]);
                
                tracing::info!("Received OP_PlayEverquestRequest: server={} sequence={}", server_number, sequence);
                let response = self.create_play_everquest_response(sequence, server_number).await;
                
                // IMPORTANT: Disconnect the session after approving play.
                // This tells the client to close the login connection and proceed to World.
                let disconnect = self.session.create_disconnect();
                
                let mut play_replies = Vec::new();
                play_replies.extend(response);
                play_replies.push(disconnect);
                return Some(play_replies); 
            },
            _ => {
                tracing::warn!("Unhandled AppOpCode {:?}", app_opcode);
                None
            }
        }
    }

    async fn handle_login(&mut self, data: &[u8]) -> Option<Vec<Vec<u8>>> {
        let mut packets = Vec::new();
        let header_size = 10;
        
        if data.len() < header_size {
            tracing::warn!("Login packet too short ({})", data.len());
            return None;
        }

        let encrypted = &data[header_size ..];
        
        let is_token = encrypted.is_empty();
        let mut is_success = false;
        let mut error_code = 101; // No Error default

        if !is_token {
            use des::Des;
            use cipher::{BlockDecryptMut, KeyIvInit};
            type DesCbc = cbc::Decryptor<Des>;
            let key = [0u8; 8];
            let iv = [0u8; 8];
            let mut buffer = encrypted.to_vec();
            let decryptor = DesCbc::new(&key.into(), &iv.into());
            match decryptor.decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buffer) {
                Ok(plaintext) => {
                    let parts: Vec<&[u8]> = plaintext.split(|&b| b == 0).collect();
                    let username = parts.get(0).map(|b| String::from_utf8_lossy(b)).unwrap_or_default().to_string();
                    let password = parts.get(1).map(|b| String::from_utf8_lossy(b)).unwrap_or_default().to_string();

                    tracing::info!("--- UDP LOGIN ATTEMPT for {} ---", username);
                    let row = if let Some(pool) = &self.pool {
                        match sqlx::query("SELECT id, password FROM account WHERE name = $1")
                            .bind(&username)
                            .fetch_optional(pool)
                            .await {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::error!("DB error looking up user {}: {}", username, e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    if self.pool.is_none() && username == "testuser" && password == "testpass" {
                        tracing::info!("UDP MOCK LOGIN: Auto-approving testuser");
                        is_success = true;
                        error_code = 101;
                        packets.extend(self.create_login_response(true, 101, 1, "0123456789").await);
                    } else if let Some(row) = row {
                        use sqlx::Row;
                        let db_pass: String = row.get("password");
                        let account_id: i32 = row.get("id");
                        
                        if db_pass == password {
                            tracing::info!("UDP Authentication Success for {} (ID={})", username, account_id);
                            
                            // Generate Session Key
                            let session_key = format!("{:05}", rand::random::<u16>());
                            
                            // Update DB with session key
                            if let Some(pool) = &self.pool {
                                let _ = sqlx::query("UPDATE account SET ls_session_key = $1 WHERE id = $2")
                                    .bind(&session_key)
                                    .bind(account_id)
                                    .execute(pool)
                                    .await;
                            }

                            is_success = true;
                            error_code = 101; // No Error
                            packets.extend(self.create_login_response(true, 101, account_id as u32, &session_key).await);
                        } else {
                            tracing::warn!("UDP Authentication Failed: Invalid password for {}", username);
                            is_success = false;
                            error_code = 13; // Common code for failure
                            packets.extend(self.create_login_response(false, 13, 0, "").await);
                        }
                    } else {
                        tracing::warn!("UDP Authentication Failed: User {} not found", username);
                        is_success = false;
                        error_code = 13;
                        packets.extend(self.create_login_response(false, 13, 0, "").await);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to decrypt login packet credentials: {:?}", e);
                    is_success = false;
                    error_code = 13;
                    packets.extend(self.create_login_response(false, 13, 0, "").await);
                }
            }
        } else {
            // Token based login or empty payload
            tracing::warn!("UDP Login Attempt without credentials (Token logic not fully implemented). Denying.");
            is_success = false;
            error_code = 13;
            packets.extend(self.create_login_response(false, 13, 0, "").await);
        }
        
        // ServerListResponse is always sent after LoginAccepted
        packets.extend(self.create_server_list_response_with_seq(0).await); 
        Some(packets)
    }

    async fn create_handshake_reply(&mut self) -> Vec<Vec<u8>> {
        // AppOp: 0x0017 (OP_ChatMessage - SoD/RoF handshake)
        let mut payload = Vec::with_capacity(17);
        // LoginBaseMessage
        payload.put_u32_le(2);
        payload.put_u8(0);
        payload.put_u8(0);
        payload.put_u32_le(0);
        
        // LoginBaseReplyMessage
        payload.put_u8(1);
        payload.put_u32_le(101); 
        payload.put_u8(0);
        
        // unknown
        payload.put_u8(0);
        
        let pkts = self.session.create_reliable_packets(OpCode::LoginApproval, &payload).await;
        pkts
    }

    async fn create_login_response(&mut self, is_success: bool, error_code: u32, account_id: u32, session_key: &str) -> Vec<Vec<u8>> {
        // AppOp: 0x0004 (OP_LoginAccepted - RoF2 UDP standard)
        let mut payload = Vec::with_capacity(90);
        
        // LoginBaseMessage
        payload.put_u32_le(3);
        payload.put_u8(0); 
        payload.put_u8(2); // DES
        payload.put_u32_le(0);
        
        // Build PlayerLoginReply
        let mut reply_struct = Vec::with_capacity(80);
        reply_struct.put_u32_le(if is_success { 1 } else { 0 }); // Offset 0-3
        reply_struct.put_u32_le(error_code);                    // Offset 4-7
        reply_struct.put_u32_le(account_id);                    // Offset 8-11
        
        // Key (Offset 12-43)
        let k_bytes = session_key.as_bytes();
        reply_struct.put_slice(k_bytes);
        reply_struct.put_u8(0); // Null terminator
        for _ in 0..(31 - k_bytes.len()) {
            reply_struct.put_u8(0);
        }

        reply_struct.put_u32_le(0); // failed_attempts
        reply_struct.put_u8(1); // show_player_count
        
        // Flooding offer fields
        reply_struct.put_u32_le(0xFFFFFFFF); 
        reply_struct.put_u32_le(0xFFFFFFFF); 
        reply_struct.put_u32_le(0); 
        reply_struct.put_u32_le(0); 
        reply_struct.put_u32_le(0xFFFFFFFF); 
        reply_struct.put_u32_le(0xFFFFFFFF); 
        reply_struct.put_u32_le(0); 
        
        while reply_struct.len() < 80 {
            reply_struct.put_u8(0);
        }
        
        tracing::info!("DEBUG: LoginAccepted ReplyStruct Hex: {:02X?}", reply_struct);

        // Encrypt
        use des::Des;
        use cipher::{BlockEncryptMut, KeyIvInit};
        type DesCbcEnc = cbc::Encryptor<Des>;
        let key = [0u8; 8]; 
        let iv = [0u8; 8];
        let encryptor = DesCbcEnc::new(&key.into(), &iv.into());
        let mut encrypted_reply = reply_struct.clone();
        if let Ok(_) = encryptor.encrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut encrypted_reply, reply_struct.len()) {
            payload.put_slice(&encrypted_reply); 
        } else {
            payload.put_slice(&reply_struct);
        }
        let pkts = self.session.create_reliable_packets(OpCode::LoginAccepted, &payload).await; // 0x0018 = LoginAccepted
        pkts
    }

    async fn create_server_list_response_with_seq(&mut self, sequence: u32) -> Vec<Vec<u8>> {
        let mut payload = Vec::with_capacity(200);
        
        // 1. Header (10 bytes)
        payload.put_u32_le(sequence); 
        payload.put_u8(0);     
        payload.put_u8(0);     
        payload.put_u32_le(0); 

        // 2. Reply (6 bytes)
        payload.put_u8(1); // Success
        payload.put_u32_le(101); // No Error
        payload.put_u8(0); // Null string

        // 3. Count (4 bytes)
        payload.put_u32_le(1);

        // 4. Entry 1
        // STRICT LAYOUT REWRITE (Attempt 2):
        // struct LoginClientServerData {
        //    ip: String (NullString),
        //    server_type: i32,
        //    server_id: i32,
        //    server_name: String (NullString),
        //    country_code: String (NullString),
        //    language_code: [u8; 1],
        //    server_status: i32,
        //    player_count: i32,
        // }
        
        let public_ip = "192.168.1.24".to_string(); // HARDCODED
        
        // 1. IP (C-String)
        let ip_bytes = public_ip.as_bytes();
        payload.put_slice(ip_bytes);
        payload.put_u8(0); 
        
        // 2. Type (i32) -> 1 (Standard?)
        payload.put_i32_le(1); 
        
        // 3. ID (i32) -> 50
        payload.put_i32_le(50);
        
        // 4. Name (C-String)
        payload.put_slice(b"RuinsFixed\0");
        
        // 5. Country (C-String)
        payload.put_slice(b"US\0");
        
        // 6. Language (u8 or [u8;1]) -> 0
        payload.put_u8(0); 
        
        // 7. Status (i32) -> 0 (Up)
        payload.put_i32_le(0);
        
        // 8. Players (i32) -> 0
        payload.put_i32_le(0);
        
        tracing::info!("DEBUG: Sending STRICT ServerListResponse. IP='{}' ID=50 Type=1.", public_ip);
        tracing::info!("DEBUG: Full Payload HEX: {:02X?}", payload);
        
        tracing::info!("Sending OP_ServerListResponse (0x0006) STRICT-LAYOUT-2. Seq={}", sequence);
        let pkts = self.session.create_reliable_packets(OpCode::ServerListResponse, &payload).await;
        pkts
    }

    async fn create_play_everquest_response(&mut self, sequence: u32, server_number: u32) -> Vec<Vec<u8>> {
        // OpCode 0x0022 - PLAY EVERQUEST RESPONSE
        // DISCOVERED FROM eqstream_old.rs: This packet does NOT contain the IP.
        // The IP comes from ServerListResponse.
        // This packet just confirms: "Yes, you can join server_number".
        //
        // Structure (20 bytes total):
        // - LoginBaseMessage (10): seq, compressed, encrypt, unk3
        // - LoginBaseReplyMessage (6): success, error_id, empty_string
        // - server_number (4)
        
        let mut payload = Vec::with_capacity(20);
        
        // --- 1. LoginBaseMessage (10 Bytes) ---
        payload.put_u32_le(sequence);
        payload.put_u8(0); // compressed
        payload.put_u8(0); // encrypt
        payload.put_u32_le(0); // unk3
        
        // --- 2. LoginBaseReplyMessage (6 Bytes) ---
        payload.put_u8(1);        // Success = true
        payload.put_u32_le(101);  // Error = 101 (No Error)
        payload.put_u8(0);        // Empty string (null terminator)
        
        // --- 3. Server Number (4 Bytes) ---
        payload.put_u32_le(server_number);
        
        tracing::info!("Sending OP_PlayEverquestResponse (0x0021) ORIGINAL STRUCT. ServerNum={}", server_number);
        let pkts = self.session.create_reliable_packets(OpCode::PlayEverquestResponse, &payload).await;
        pkts
    }
}
