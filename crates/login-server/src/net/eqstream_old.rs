use std::collections::HashMap;
use nom::{
    bytes::complete::take,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
    IResult,
};
use bytes::BufMut;
use crc::{Crc, CRC_32_ISO_HDLC};

pub const EQ_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[derive(Debug, Clone)]
pub enum EQStreamPacket {
    SessionRequest(SessionRequest),
    SessionResponse(SessionResponse),
    Combined(Vec<u8>),
    Packet(Vec<u8>),
    Unknown(u16, Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct SessionRequest {
    pub opcode: u8, // usually 0x01
    pub protocol_version: u32,
    pub session_id: u32, // connect_code
    pub max_length: u32,
}

#[derive(Debug, Clone)]
pub struct SessionResponse {
    pub opcode: u8,         // 0x02
    pub session_id: u32,    // connect_code
    pub crc_key: u32,       // encode_key
    pub crc_bytes: u8,      // 2
    pub encode_pass1: u8,
    pub encode_pass2: u8,
    pub max_length: u32,
}

// ...

pub fn parse_eqstream(input: &[u8]) -> IResult<&[u8], EQStreamPacket> {
    // Basic EQStream Frame: [Zero:1][OpCode:1][Data...]
    // But OpCode 1 (SessionRequest) is special?
    // Actually header is 2 bytes: [Zero:1][Op:1] for 1-byte opcodes?
    // EQEmu says: if p.GetInt8(0) == 0 { switch p.GetInt8(1) }
    
    let (input, zero) = nom::number::complete::u8(input)?;
    if zero != 0 {
        // Encoded/Sequenced packet
        return Ok((input, EQStreamPacket::Unknown(0xFFFF, input.to_vec())));
    }
    
    let (input, opcode) = nom::number::complete::u8(input)?;

    match opcode {
        0x01 => {
            // Session Request
            // Struct ReliableStreamConnect (14 bytes total, we read 2 already)
            // Need 12 more bytes: [Prot:4][Connect:4][Max:4]
            let (input, protocol_version) = be_u32(input)?;
            let (input, session_id) = be_u32(input)?;
            let (input, max_length) = be_u32(input)?;
            
            // Payload might follow ("Everquest\0")
            let (input, _payload) = take(input.len())(input)?;
            
            Ok((input, EQStreamPacket::SessionRequest(SessionRequest {
                opcode,
                protocol_version,
                session_id,
                max_length,
            })))
        },
        _ => {
            let (input, data) = take(input.len())(input)?;
            Ok((input, EQStreamPacket::Unknown(opcode as u16, data.to_vec())))
        }
    }
}

// ...

pub struct EqStreamSession {
    pub session_id: u32,
    pub crc_key: u32,
    pub sequence_in: u16, // Next expected sequence
    pub sequence_out: u16, // Next sequence to send
}

impl EqStreamSession {
    pub fn new(session_id: u32) -> Self {
        Self {
            session_id,
            crc_key: 0x12345678, 
            sequence_in: 0,
            sequence_out: 0,
        }
    }

    pub fn handle_session_request(&self, req: &SessionRequest) -> Vec<u8> {
        let mut response = Vec::with_capacity(32);
        
        response.put_u8(0);
        response.put_u8(0x02); // OP_SessionResponse
        response.put_u32(req.session_id); 
        response.put_u32(self.crc_key);   
        response.put_u8(2);               
        response.put_u8(0);               
        response.put_u8(0);               
        response.put_u32(512);            

        self.append_crc(&mut response);
        response
    }


    pub fn receive_packet(&mut self, opcode: u16, payload: &[u8]) -> Vec<Vec<u8>> {
        // ... (lines 115-177 same as before) ...
        // I will just invoke replace on the struct and new and add methods at bottom
        let mut responses = Vec::new();
        match opcode {
            0x09 => { // OP_Packet
                if payload.len() >= 2 {
                    let sequence = u16::from_be_bytes([payload[0], payload[1]]);
                    tracing::info!("Received OP_Packet Seq={:04X}", sequence);
                    
                    // Simple ACK (Liberal)
                    responses.push(self.create_ack(sequence));
                    
                    // Application Layer (Payload [2..])
                    if payload.len() >= 4 {
                        let app_opcode = u16::from_le_bytes([payload[2], payload[3]]); 
                        tracing::info!("AppOpCode={:04X}", app_opcode);
                        
                        match app_opcode {
                            0x0001 => { // OP_SessionReady
                                tracing::info!("Handling OP_SessionReady, sending OP_ChatMessage (HandshakeReply) + StatRequest");
                                responses.push(self.create_handshake_reply());
                                responses.push(self.create_stat_request());
                            },
                            0x0002 | 0x0003 => { // OP_Login
                                tracing::info!("Received OP_Login (Op={:04X})", app_opcode);
                                if let Some(replies) = self.handle_login(&payload[4..]) {
                                    responses.extend(replies);
                                }
                            },
                            0x0004 => { // OP_ServerListRequest (SoD/RoF)
                                tracing::info!("Received OP_ServerListRequest");
                                // Extract sequence from first 4 bytes of app payload
                                let seq = if payload.len() >= 8 {
                                    u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]])
                                } else {
                                    0
                                };
                                responses.push(self.create_server_list_response_with_seq(seq));
                            },
                            0x000d => { // OP_PlayEverquestRequest (SoD/RoF)
                                tracing::info!("Received OP_PlayEverquestRequest");
                                // PlayEverquestRequest: LoginBaseMessage(10) + server_number(4)
                                // Extract sequence and server_number
                                let seq = if payload.len() >= 8 {
                                    u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]])
                                } else {
                                    0
                                };
                                let server_id = if payload.len() >= 18 {
                                    // payload[4..14] = LoginBaseMessage, payload[14..18] = server_number
                                    u32::from_le_bytes([payload[14], payload[15], payload[16], payload[17]])
                                } else {
                                    1
                                };
                                tracing::info!("PlayRequest: seq={}, server_id={}", seq, server_id);
                                responses.push(self.create_play_everquest_response(seq, server_id));
                            },
                            _ => {
                                tracing::warn!("Unhandled AppOpCode {:04X} Data={:02X?}", app_opcode, &payload[4..]);
                            }
                        }
                    }
                }
            },
            0x07 => {
                responses.push(self.create_stat_response());
            },
            _ => {}
        }
        responses
    }

    fn handle_login(&mut self, data: &[u8]) -> Option<Vec<Vec<u8>>> {
        // Data: [LoginBaseMessage:10] [EncryptedBlob...] [CRC:2]
        tracing::info!("Login Data Payload ({}) : {:02X?}", data.len(), data);

        // Header is 10 bytes (Packed LoginBaseMessage)
        let header_size = 10;
        let crc_size = 2; // CRC enabled
        
        if data.len() < (header_size + crc_size) {
            tracing::warn!("Login packet too short ({})", data.len());
            return None;
        }

        // Encrypted blob is between Header and CRC
        let encrypted = &data[header_size .. data.len() - crc_size];
        
        // Token logic check (Empty Payload)
        let is_token = encrypted.len() == 0;
        if is_token {
             tracing::warn!("Login encrypted payload is empty (Token Login)");
        } else if encrypted.len() % 8 != 0 {
            tracing::warn!("Login encrypted payload not block aligned (Len={})", encrypted.len());
            return None;
        }
        
        if !is_token {
            // Decrypt DES-CBC Zero Key/IV
            use des::Des;
            use cipher::{BlockDecryptMut, KeyIvInit};
            type DesCbc = cbc::Decryptor<Des>;

            let key = [0u8; 8];
            let iv = [0u8; 8];
            let mut buffer = encrypted.to_vec();
            
            let decryptor = DesCbc::new(&key.into(), &iv.into());
            if let Ok(plaintext) = decryptor.decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buffer) {
                 let user_pass_str = String::from_utf8_lossy(plaintext);
                 tracing::info!("Decrypted Login Credentials: {:?}", user_pass_str);
            } else {
                 return None;
            }
        }
        
        // Prepare Response(s)
        let mut packets = Vec::new();
        packets.push(self.create_login_accepted());
        packets.push(self.create_server_list_response());
        
        Some(packets)
    }

    fn create_server_list_response(&mut self) -> Vec<u8> {
        self.create_server_list_response_with_seq(0)
    }
    
    fn create_server_list_response_with_seq(&mut self, sequence: u32) -> Vec<u8> {
        let mut payload = Vec::with_capacity(100);
        
        // LoginBaseMessage (10 bytes)
        payload.put_u32_le(sequence); // sequence from client's request
        payload.put_u8(0);     // compressed
        payload.put_u8(0);     // encrypt_type
        payload.put_u32_le(0); // unk3
        
        // LoginBaseReplyMessage (variable)
        payload.put_u8(1); // Success
        payload.put_u32_le(101); // No Error (0x65)
        payload.put_u8(0); // Empty string (null terminator)
        
        // Server Count
        payload.put_u32_le(1);
        
        // LoginClientServerData for each server:
        // 1. IP (string, null-terminated)
        payload.put_slice(b"127.0.0.1\0");
        // 2. server_type (int32) - 1=Standard, 8=Preferred, 16=Legends
        payload.put_i32_le(1); // Standard
        // 3. server_id (uint32)
        payload.put_u32_le(1);
        // 4. server_name (string, null-terminated)
        payload.put_slice(b"Rust Server\0");
        // 5. country_code (string, null-terminated) - lowercase!
        payload.put_slice(b"us\0");
        // 6. language_code (string, null-terminated) - lowercase!
        payload.put_slice(b"en\0");
        // 7. server_status (int32) - 0=Up, 1=Down, 4=Locked
        payload.put_i32_le(0); // Up
        // 8. player_count (uint32)
        payload.put_u32_le(0);
        
        self.create_reliable_packet(0x0019, &payload) // SoD/RoF: OP_ServerListResponse
    }
    
    fn create_play_everquest_response(&mut self, sequence: u32, server_number: u32) -> Vec<u8> {
        // OP_PlayEverquestResponse (0x0022 for SoD/RoF)
        // PlayEverquestResponse: LoginBaseMessage(10) + LoginBaseReplyMessage(6) + server_number(4)
        let mut payload = Vec::with_capacity(20);
        
        // LoginBaseMessage (10 bytes)
        payload.put_u32_le(sequence);
        payload.put_u8(0); // compressed
        payload.put_u8(0); // encrypt_type
        payload.put_u32_le(0); // unk3
        
        // LoginBaseReplyMessage (6 bytes)
        payload.put_u8(1); // Success = true
        payload.put_u32_le(101); // No Error (0x65)
        payload.put_u8(0); // Empty string
        
        // server_number (4 bytes)
        payload.put_u32_le(server_number);
        
        tracing::info!("Sending OP_PlayEverquestResponse: success, server={}", server_number);
        self.create_reliable_packet(0x0022, &payload) // SoD/RoF: OP_PlayEverquestResponse
    }

    fn create_login_accepted(&mut self) -> Vec<u8> {
        // AppOp: 0x0018 (OP_LoginAccepted - SoD/RoF)
        // Payload: [LoginBaseMessage:10] [EncryptedReply:80]
        // EQEmu encrypts sizeof(PlayerLoginReply) (58 bytes, padded to 64), then appends 16 zeros
        
        let mut payload = Vec::with_capacity(90);
        
        // LoginBaseMessage (10 bytes)
        // seq=3 (Login), compressed=0, encrypt=2 (DES), unk3=0
        payload.put_u32_le(3);
        payload.put_u8(0); // compressed
        payload.put_u8(2); // encrypt_type = 2 (DES)
        payload.put_u32_le(0);
        
        // Build PlayerLoginReply struct (58 bytes)
        let mut reply_struct = Vec::with_capacity(64);
        
        // LoginBaseReplyMessage (6 bytes)
        reply_struct.put_u8(1); // Success
        reply_struct.put_u32_le(101); // No Error
        reply_struct.put_u8(0); // Str = 0
        
        reply_struct.put_u8(0); // Unk1
        reply_struct.put_u8(0); // Unk2
        reply_struct.put_u32_le(1); // LSID (AccountID=1)
        
        // Key (11 bytes) "0123456789\0"
        reply_struct.put_slice(b"0123456789\0");
        
        reply_struct.put_u32_le(0); // FailedAttempts
        reply_struct.put_u8(1); // ShowPlayerCount
        
        reply_struct.put_u32_le(99); // OfferMinDays
        reply_struct.put_u32_le(0xFFFFFFFF); // OfferMinViews = -1
        reply_struct.put_u32_le(0); // OfferCooldown
        
        reply_struct.put_u32_le(0); // WebOfferNumber
        reply_struct.put_u32_le(99); // WebOfferMinDays
        reply_struct.put_u32_le(0xFFFFFFFF); // WebOfferMinViews = -1
        reply_struct.put_u32_le(0); // WebOfferCooldown
        
        // Username(1) + Unknown(1) = 2 null bytes
        reply_struct.put_u8(0);
        reply_struct.put_u8(0);
        
        // Pad to 64 bytes for DES (58 -> 64)
        while reply_struct.len() % 8 != 0 {
            reply_struct.put_u8(0);
        }
        
        // Encrypt using DES-CBC with zero key/IV
        use des::Des;
        use cipher::{BlockEncryptMut, KeyIvInit};
        type DesCbcEnc = cbc::Encryptor<Des>;
        let key = [0u8; 8];
        let iv = [0u8; 8];
        let encryptor = DesCbcEnc::new(&key.into(), &iv.into());
        
        let mut encrypted_reply = reply_struct.clone();
        if let Ok(_) = encryptor.encrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut encrypted_reply, reply_struct.len()) {
            payload.put_slice(&encrypted_reply); // 64 encrypted bytes
        } else {
            tracing::error!("Encryption Failed");
            payload.put_slice(&reply_struct);
        }
        
        // Append 16 zeros (matching EQEmu's char encrypted_buffer[80])
        payload.put_slice(&[0u8; 16]);
        
        self.create_reliable_packet(0x0018, &payload) // SoD/RoF: OP_LoginAccepted
    }

    fn create_ack(&self, sequence: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(6);
        packet.put_u8(0);
        packet.put_u8(0x15); 
        packet.put_u16(sequence);
        self.append_crc(&mut packet);
        packet
    }

    fn create_stat_request(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(42);
        packet.put_u8(0);
        packet.put_u8(0x07); // OP_SessionStatRequest
        packet.put_slice(&[0u8; 38]); // Zero payload
        self.append_crc(&mut packet);
        packet
    }

    fn create_stat_response(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(42);
        packet.put_u8(0);
        packet.put_u8(0x08); 
        packet.put_slice(&[0u8; 38]); // Zero payload
        self.append_crc(&mut packet);
        packet
    }

    fn create_reliable_packet(&mut self, app_opcode: u16, data: &[u8]) -> Vec<u8> {
        // [00 09] [Seq:2] [AppOp:2] [Data...]
        let mut packet = Vec::with_capacity(6 + data.len());
        packet.put_u8(0);
        packet.put_u8(0x09); // OP_Packet
        packet.put_u16(self.sequence_out);
        self.sequence_out = self.sequence_out.wrapping_add(1);
        
        // App Layer
        packet.put_u16_le(app_opcode);
        packet.put_slice(data);
        
        self.append_crc(&mut packet);
        packet
    }

    fn create_handshake_reply(&mut self) -> Vec<u8> {
        // AppOp: 0x0017 (OP_ChatMessage - SoD/RoF handshake)
        // Payload: LoginHandShakeReply (17 bytes)
        let mut payload = Vec::with_capacity(17);
        // LoginBaseMessage (10)
        // seq=2, compressed=0, encrypt=0, unk3=0
        payload.put_u32_le(2);
        payload.put_u8(0);
        payload.put_u8(0);
        payload.put_u32_le(0);
        
        // LoginBaseReplyMessage (6)
        // success=1, error=101, str=0
        payload.put_u8(1);
        payload.put_u32_le(101); // 0x65
        payload.put_u8(0);
        
        // unknown(1)
        payload.put_u8(0);
        
        self.create_reliable_packet(0x0017, &payload) // SoD/RoF: OP_ChatMessage
    }

    fn append_crc(&self, packet: &mut Vec<u8>) {
        // CRC with zero key (crc_key=0 in Session Response)
        let mut digest = EQ_CRC.digest();
        digest.update(&self.crc_key.to_le_bytes()); // crc_key = 0
        digest.update(packet);
        let crc32 = digest.finalize();
        let crc16 = (crc32 & 0xFFFF) as u16;
        // Big-endian CRC (works for handshake)
        packet.put_u16(crc16);
    }
}
