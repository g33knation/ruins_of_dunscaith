
use tokio::sync::mpsc;
use std::net::SocketAddr;
use tracing::{info, warn, error, debug};
use shared::net::eq_stream::{EqStreamSession as SharedSession, EQStreamPacket, ProcessPacketResult, parse_eqstream};
use shared::opcodes::OpCode;
use crate::packets;
use crate::db::DatabaseManager;
use std::sync::Arc;

pub struct PendingChar {
    pub name: String,
    pub race_id: u32,
    pub class_id: u32,
    pub deity: u32,
    pub gender: u32,
}

pub struct ClientSessionActor {
    addr: SocketAddr,
    session: SharedSession,
    sender: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    db: Arc<DatabaseManager>,
    pending_char: Option<PendingChar>,
    account_id: i32,
    is_running: bool,
}

impl ClientSessionActor {
    pub fn new(
        addr: SocketAddr, 
        session_id: u32, 
        sender: mpsc::Sender<(SocketAddr, Vec<u8>)>,
        db: Arc<DatabaseManager>,
    ) -> Self {
        let mut session = SharedSession::new(session_id);
        session.crc_key = 0; // Use 0 to match login-server success
        session.enable_combined();
        session.enable_compression(); // Re-enable compression (RoF2 requires it)
        
        Self {
            addr,
            session,
            sender,
            db,
            pending_char: None,
            account_id: 0, 
            is_running: true,
        }
    }

    pub async fn run(&mut self, mut rx: mpsc::Receiver<Vec<u8>>) {
        info!("Session Actor for {} started.", self.addr);
        
        while let Some(data) = rx.recv().await {
            self.handle_packet(&data).await;
            if !self.is_running {
                info!("Session Actor for {} exiting main loop.", self.addr);
                break;
            }
        }
        
        info!("Session Actor for {} stopped.", self.addr);
    }

    pub async fn handle_packet(&mut self, data: &[u8]) {
        // Log ALL raw incoming packets for debugging
        info!("[{}] RX RAW: len={} hex={:02X?}", self.addr, data.len(), &data[..data.len().min(40)]);

        match parse_eqstream(data) {
            Ok((_, pkt)) => {
                match pkt {
                    EQStreamPacket::SessionRequest(req) => {
                        info!("Handling Session Request in Actor for {}", self.addr);
                        let response = self.session.handle_session_request(&req);
                        self.send_raw(response).await;
                    }
                    EQStreamPacket::Ack(seq) => {
                        info!("[{}] RX ACK for seq={}", self.addr, seq);
                    }
                    EQStreamPacket::Disconnect(reason) => {
                        info!("[{}] RX DISCONNECT reason={}", self.addr, reason);
                    }
                    EQStreamPacket::Unknown(opcode, payload) => {
                        self.process_transport_packet(opcode, &payload).await;
                    }
                    _ => {
                        // Other variants handled if needed
                    }
                }
            },
            Err(_) => {}
        }
    }
    
    async fn process_transport_packet(&mut self, opcode: u16, payload: &[u8]) {
        let results = self.session.process_packet(opcode, payload);
        
        for res in results {
            match res {
                ProcessPacketResult::Response(pkt) => self.send_raw(pkt).await,
                ProcessPacketResult::Application(app_op, app_data) => {
                    self.handle_application_packet(app_op, app_data).await;
                }
                _ => {}
            }
        }
    }

    async fn handle_application_packet(&mut self, app_opcode: OpCode, data: Vec<u8>) {
         // LOG ALL OPCODES TO DEBUG CLIENT STATE
         info!("[{}] Processing Application Packet: {:?} (Raw Len={})", self.addr, app_opcode, data.len());

         let decompressed = match SharedSession::decompress_payload(data).await {
            Ok(d) => d,
            Err(e) => {
                error!("Decompression failure: {}", e);
                return;
            }
        };

        match app_opcode {
            OpCode::SessionReady => { // 0x0001
                info!("[{}] OP_SessionReady - Connection Established. Waiting for OP_SendLoginInfo.", self.addr);
                // Do NOT send approval here. Wait for the client to identify itself (0x7a09).
            }
            OpCode::SendLoginInfo => { // 0x7a09
                info!("[{}] Received OP_SendLoginInfo. Hex: {:02X?}", self.addr, &decompressed[..64.min(decompressed.len())]);
                
                let parts: Vec<&[u8]> = decompressed.split(|&b| b == 0).collect();
                let account_id_str = parts.get(0).map(|b| String::from_utf8_lossy(b)).unwrap_or_default().to_string();
                let session_key = parts.get(1).map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
                let account_id: i32 = account_id_str.parse().unwrap_or(0);
                
                info!("[{}] Validating Session: AccountID={} (Str='{}'), Key='{}'", self.addr, account_id, account_id_str, session_key);

                match self.db.verify_session(account_id as i32, &session_key).await {
                    Ok(true) => {
                        info!("[{}] Session Validated successfully for account {}", self.addr, account_id);
                        self.account_id = account_id as i32;
                        self.send_login_approval().await;
                    }
                    Ok(false) => {
                        warn!("[{}] Invalid Session for account {} (Key: {})", self.addr, account_id, session_key);
                        // Allow access if account_id is 1 and key is "RUST_DEBUG" for development override
                        if account_id == 1 && session_key == "RUST_DEBUG" {
                             warn!("[{}] DEBUG OVERRIDE: Allowing access for account 1", self.addr);
                             self.account_id = 1;
                             self.send_login_approval().await;
                        }
                    }
                    Err(e) => {
                        error!("[{}] DB Error during session verification: {}", self.addr, e);
                    }
                }
            },

            OpCode::RoF2ClientReady | OpCode::CharSelectRequest => { 
                info!("[{}] Received {:?} - Sending Character List.", self.addr, app_opcode);
                // ALWAYS send the list.
                self.send_character_list_sequence().await;
            },
            OpCode::ApproveName => { // 0x56a2
                info!("[{}] Received OP_ApproveName", self.addr);
                use binrw::BinRead;
                use std::io::Cursor;
                let mut reader = Cursor::new(&decompressed);
                if let Ok(approval) = packets::NameApprovalStruct::read(&mut reader) {
                    info!("Name Approval Request: {} Race={} Class={} Deity={} Gender={}", 
                         approval.name, approval.race_id, approval.class_id, approval.deity, approval.unknown1);
                    
                    // Store for final creation step
                    self.pending_char = Some(PendingChar {
                        name: approval.name.clone(),
                        race_id: approval.race_id,
                        class_id: approval.class_id,
                        deity: approval.deity,
                        gender: approval.unknown1,
                    });

                    let response = 1u32.to_le_bytes(); 
                    self.send_app_packet(OpCode::ApproveName, &response).await;
                } else {
                    error!("[{}] Failed to parse OP_ApproveName (decompressed len={})", self.addr, decompressed.len());
                }
            },

            OpCode::DeleteCharacter => {
                let name = decompressed.iter()
                    .position(|&b| b == 0)
                    .map(|pos| String::from_utf8_lossy(&decompressed[..pos]).to_string())
                    .unwrap_or_else(|| String::from_utf8_lossy(&decompressed).to_string());
                let name = name.trim().to_string();
                
                info!("[{}] Received OP_DeleteCharacter for '{}' (raw_len={})", self.addr, name, decompressed.len());

                if !name.is_empty() {
                    match self.db.delete_character(1, &name).await {
                        Ok(deleted) => {
                            if deleted {
                                info!("Character {} deleted successfully.", name);
                            } else {
                                warn!("Delete failed: Character {} not found for account 1.", name);
                            }
                            self.send_char_list().await;
                        }
                        Err(e) => error!("Failed to delete character {}: {}", name, e),
                    }
                }
            },

            OpCode::CharacterCreateRequest => { // 0x6773
                info!("[{}] Received OP_CharacterCreateRequest", self.addr);
                self.send_app_packet(OpCode::CharacterCreateRequest, &packets::build_character_create_request_response()).await;
            },
            
            OpCode::CharacterCreate => { // 0x6bbf
                info!("[{}] Received OP_CharacterCreate", self.addr);
                
                if let Some(pending) = self.pending_char.as_ref() {
                    info!("Finalizing Character Creation for '{}'", pending.name);
                    
                    // Clean up existing if duplicate
                    let _ = self.db.delete_character(1, &pending.name).await;

                    // Create Character
                    let _ = self.db.create_character(
                        1, 
                        &pending.name,
                        pending.race_id as i16,
                        pending.class_id as i16,
                        pending.gender as i16,
                        1, 
                        75, 75, 75, 75, 75, 75, 75, 
                        pending.deity as i16,
                        202 
                    ).await.map_err(|e| error!("Final-Create Failed: {}", e));
                    
                    // User Request: Direct Zone to PoK (Skip Char Select)
                    self.initiate_zoning_sequence(pending.name.clone()).await;
                } else {
                     warn!("Received OP_CHARACTER_CREATE without pending char!");
                     // Fallback
                     self.send_char_list().await;
                }
            },

            OpCode::ApproveWorld => { 
                info!("[{}] Received OP_ApproveWorld (Echo) - Client handled burst.", self.addr);
            },
            
            // CRC Checksums - Must acknowledge all 3
            OpCode::WorldClientCrc1 => { 
                info!("[{}] Received Client CRC1 (eqgame.exe)", self.addr); 
            },
            OpCode::WorldClientCrc2 => { 
                info!("[{}] Received Client CRC2 (SkillCaps.txt)", self.addr); 
            },
            OpCode::WorldClientCrc3 => { 
                info!("[{}] Received Client CRC3 (BaseData.txt)", self.addr); 
            }, 
            OpCode::Unknown if decompressed.len() == 0 => { // Could be heartbeat etc
                ()
            }
            
            OpCode::EnterWorld => { // 0x57c3 (Fixing constant name to match EnterWorld)
                let name = decompressed.iter()
                    .position(|&b| b == 0)
                    .map(|pos| String::from_utf8_lossy(&decompressed[..pos]).to_string())
                    .unwrap_or_else(|| String::from_utf8_lossy(&decompressed).to_string());
                let mut name = name.trim().to_string();
                
                // If name is invalid (e.g. empty or non-alphabetic sub-packet data),
                // check if we have a pending character to auto-login
                if name.len() < 3 || !name.chars().all(|c| c.is_alphabetic()) {
                    if let Some(ref pending) = self.pending_char {
                         info!("[{}] Auto-Zoning Pending Character '{}' for OP_EnterWorld", self.addr, pending.name);
                         name = pending.name.clone();
                    } else {
                        debug!("[{}] Ignored invalid EnterWorld name '{}' (Len={})", self.addr, name, decompressed.len());
                        return;
                    }
                }

                info!("[{}] Handle OP_EnterWorld for character '{}' (len={})", self.addr, name, decompressed.len());
                self.initiate_zoning_sequence(name).await;
            },
            
            OpCode::RoF2Unknown1500 => { // 0x1500
                 // Just ignore this flood
                 debug!("Ignored RoF2 Flood 0x1500");
            },

            OpCode::Unknown => {
                  warn!("[{}] UNHANDLED AppOpCode: {:?} (Len={})", self.addr, OpCode::Unknown, decompressed.len());
            }
            _ => { /* Ignore any others */ }
        }
    }
    
    async fn initiate_zoning_sequence(&mut self, name: String) {
        if !name.is_empty() {
            match self.db.get_character_location(&name).await {
                Ok((zone_id, _x, _y, _z, _heading)) => {
                    let _instance_id = 0; // Instance 0 for static zones
                    
                    // Send OP_SEND_LOGIN_INFO (0x7a09)
                    // Captured from akk-stack: "1\0" + Key + "\0" (approx)
                    // This seems critical for client to authorize zone connection.
                    let mut login_info = Vec::new();
                    login_info.extend_from_slice(b"1\0"); // Account Status?
                    login_info.extend_from_slice(b"GeneratedKey123\0"); // Session Key
                    info!("Sending Pre-Handoff LoginInfo.");
                    self.send_app_packet(OpCode::SendLoginInfo, &login_info).await;

                    info!("Char {} is in zone {}", name, zone_id);
                    
                    // IP Address: Use PUBLIC_IP for zone handoff
                    let ip_addr = std::env::var("PUBLIC_IP").unwrap_or("127.0.0.1".to_string()); 
                    let port = 7000u16;
                    let zone_info = packets::build_zone_server_info(&ip_addr, port);
                    
                    // Debug: Dump the packet bytes
                    info!("Handoff: Sending OP_ZONE_SERVER_INFO -> {}:{}", ip_addr, port);
                    
                    // Per akk-stack trace: Server sends ZoneServerInfo TWICE (at 21:53:38.462 and 21:53:38.478)
                    // This appears to be intentional for reliability
                    self.send_app_packet(OpCode::ZoneServerInfo, &zone_info).await;
                    
                    // Small delay then send again (matching akk-stack ~16ms gap)
                    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
                    self.send_app_packet(OpCode::ZoneServerInfo, &zone_info).await;
                    
                    info!("Handoff: Sent OP_ZONE_SERVER_INFO twice for reliability");
                }
                Err(e) => error!("Failed to find character '{}': {}", name, e),
            }
        }
    }

    async fn send_login_approval(&mut self) {
        self.session.compression_enabled = true;

        // ===== EXACT EQEmu world/client.cpp HandleSendLoginInfoPacket sequence =====
        // Reference: https://github.com/EQEmu/Server/blob/master/world/client.cpp

        // 1. GuildsList (EQEmu sends this FIRST, before everything else)
        self.send_app_packet(OpCode::GuildsList, &packets::build_guilds_list()).await;

        // 2. LogServer (560 bytes)
        self.send_app_packet(OpCode::LogServer, &packets::build_log_server()).await;
        
        // 3. ApproveWorld (544 bytes)
        self.send_app_packet(OpCode::ApproveWorld, &packets::build_approve_world()).await;
        
        // 4. EnterWorld — Payload: \0
        self.send_app_packet(OpCode::EnterWorld, &[0]).await;

        // 5. PostEnterWorld — Payload: []
        self.send_app_packet(OpCode::PostEnterWorld, &[]).await;

        info!("[{}] Sent Base Login Burst (Guilds, Log, Approve, Enter, PostEnter) -> Sequential Char Select.", self.addr);
        
        // 6-10. Character selection packets (Expansion, MaxChars, Membership, Details, CharInfo)
        self.send_character_list_sequence().await;
    }

    async fn send_character_list_sequence(&mut self) {
        // 6. ExpansionInfo (68 bytes)
        self.send_app_packet(OpCode::ExpansionInfo, &packets::build_expansion_info()).await;
        
        // 7. MaxCharacters (12 bytes) — called from SendCharInfo() in EQEmu
        self.send_app_packet(OpCode::SendMaxCharacters, &packets::build_send_max_characters()).await;
        
        // 8. Membership (104 bytes) — called from SendCharInfo() in EQEmu
        self.send_app_packet(OpCode::SendMembership, &packets::build_membership()).await;

        // 9. MembershipDetails (1124 bytes) — called from SendCharInfo() in EQEmu
        self.send_app_packet(OpCode::SendMembershipDetails, &packets::build_membership_details()).await;

        // 10. Character List (OP_SendCharInfo) — the actual character data
        self.send_char_list().await;
        
        info!("[{}] Sent Character List Sequence", self.addr);
    }

    async fn send_char_list(&mut self) {
        if self.account_id == 0 {
             warn!("[{}] send_char_list: account_id is 0, cannot fetch characters.", self.addr);
             return;
        }

        info!("[{}] Fetching characters for account_id={}", self.addr, self.account_id);
        
        match self.db.get_characters(self.account_id).await {
            Ok(chars) => {
                info!("[{}] Found {} characters for account {}", self.addr, chars.len(), self.account_id);
                // The packets::build_char_info function now includes logging and force Zone 0
                let data = packets::build_char_info(chars);
                
                // Allow compression logic to handle it (if > 100 bytes)
                self.send_app_packet(OpCode::SendCharInfo, &data).await;
                info!("[{}] Sent OP_SEND_CHAR_INFO ({} bytes)", self.addr, data.len());
            },
            Err(e) => {
                error!("[{}] Failed to fetch characters: {}", self.addr, e);
            }
        }
    }
    
    async fn send_app_packet(&mut self, opcode: OpCode, data: &[u8]) {
        // Hex-dump first 40 bytes of application data for debugging
        let preview_len = data.len().min(40);
        info!("[{}] TX APP {:?} ({} bytes) first_bytes={:02X?}", self.addr, opcode, data.len(), &data[..preview_len]);
        
        let packets = self.session.create_raw_packets(opcode, data);
        for pkt in &packets {
            info!("[{}] Sending packet for {:?} (len={})", self.addr, opcode, pkt.len());
            self.send_raw(pkt.clone()).await;
        }
    }

    async fn send_raw(&self, data: Vec<u8>) {
        if let Err(e) = self.sender.send((self.addr, data)).await {
            error!("Failed to send packet to socket: {}", e);
        }
    }
}
