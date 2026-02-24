use std::collections::HashMap;
use crate::opcodes::OpCode;
use num_traits::FromPrimitive;
use bytes::{BufMut, BytesMut};
use log::{info, warn, error};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use crc::{Crc, CRC_32_ISO_HDLC};

// use crate::net::packet::SessionRequest; // Removed as it is now local

pub const EQ_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Malformed packet payload")]
    MalformedPayload,
    #[error("Invalid transport opcode: {0:04x}")]
    InvalidOpcode(u16),
}

#[derive(Debug, Clone)]
pub enum EQStreamPacket {
    SessionRequest(SessionRequest),
    SessionResponse(u16, u32, u32, u8),
    Combined(Vec<Vec<u8>>),
    Ack(u16),
    OutOfOrder(u16),
    Disconnect(u32),
    Stats(u16),
    Fragment(u16, u32, Vec<u8>),
    AppPacket(u16, Vec<u8>),
    Unknown(u16, Vec<u8>),
}

#[derive(Debug, Clone, binrw::BinRead, binrw::BinWrite)]
#[br(little)]
pub struct SessionRequest {
    pub session_id: u32,
    pub protocol_version: u32,
    pub max_length: u32,
}

pub fn parse_eqstream(data: &[u8]) -> Result<(&[u8], EQStreamPacket), ProtocolError> {
    if data.len() < 2 { return Err(ProtocolError::MalformedPayload); }
    let opcode = u16::from_be_bytes([data[0], data[1]]);
    let payload = &data[2..];
    
    match opcode {
        0x0001 => {
            if payload.len() < 12 { return Err(ProtocolError::MalformedPayload); }
            let protocol_version = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let session_id = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let max_length = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
            Ok((&[], EQStreamPacket::SessionRequest(SessionRequest { session_id, protocol_version, max_length })))
        }
        0x0005 => {
            if payload.len() < 4 { return Err(ProtocolError::MalformedPayload); }
            Ok((&[], EQStreamPacket::Disconnect(u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]))))
        }
        0x0015 => {
             if payload.len() < 2 { return Err(ProtocolError::MalformedPayload); }
             Ok((&[], EQStreamPacket::Ack(u16::from_be_bytes([payload[0], payload[1]]))))
        }
        _ => Ok((&[], EQStreamPacket::Unknown(opcode, payload.to_vec())))
    }
}

#[derive(Debug, Clone)]
pub enum ProcessPacketResult {
    Response(Vec<u8>),      // Direct transport-layer response (ACK, etc)
    Application(OpCode, Vec<u8>), // Decoded application opcode and payload
}

#[derive(Debug, Default)]
pub struct FragmentReassembly {
    pub buffer: Vec<u8>,
    pub total_len: usize,
}

pub struct EqStreamSession {
    pub session_id: u32,
    pub crc_key: u32,
    pub sequence_in: u16,  // Next expected sequence to receive
    pub sequence_out: u16, // Next sequence to send
    pub combined_enabled: bool,
    pub compression_enabled: bool,
    pub max_length: u32,
    
    // Reliability
    pub last_received_sequence: u16, 
    pub last_acked_sequence: u16,    
    pub sent_packets: HashMap<u16, Vec<u8>>, 
    pub packets_sent: u64,
    pub packets_received: u64,
    
    // Fragmentation Reassembly
    pub fragment_streams: HashMap<u16, FragmentReassembly>,
    
    // OOO Stash
    pub ooo_buffer: HashMap<u16, Vec<u8>>,
}

impl EqStreamSession {
    pub async fn decompress_payload(data: Vec<u8>) -> Result<Vec<u8>, ProtocolError> {
        if data.len() < 2 || data[0] != 0x5a || data[1] != 0x00 {
            return Ok(data);
        }
        
        if data.len() < 6 { return Err(ProtocolError::MalformedPayload); }
        let decompressed_len = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
        
        let mut decoder = flate2::read::ZlibDecoder::new(&data[6..]);
        let mut decompressed = Vec::with_capacity(decompressed_len);
        if std::io::copy(&mut decoder, &mut decompressed).is_ok() {
            Ok(decompressed)
        } else {
            warn!("ZLib Decompression Failed");
            Ok(data) // Fallback to original
        }
    }

    pub fn new(session_id: u32) -> Self {
        Self {
            session_id,
            crc_key: 0,
            sequence_in: 0,
            sequence_out: 0,
            combined_enabled: false,
            compression_enabled: false,
            max_length: 1024, // Restored to 1024 for RoF2 compatibility
            
            last_received_sequence: u16::MAX, // Expect 0 first
            last_acked_sequence: u16::MAX,
            sent_packets: HashMap::new(),
            packets_sent: 0,
            packets_received: 0,
            
            fragment_streams: HashMap::new(),
            ooo_buffer: HashMap::new(),
        }
    }
    
    pub fn enable_combined(&mut self) {
        self.combined_enabled = true;
    }
    
    pub fn enable_compression(&mut self) {
        self.compression_enabled = true;
    }

    pub fn handle_session_request(&mut self, req: &SessionRequest) -> Vec<u8> {
        self.session_id = req.session_id;
        info!("SessionRequest: Protocol={} SessionID={} MaxLen={}", req.protocol_version, req.session_id, req.max_length);
        
        // Reverting to 512 (Standard). We rely on LE Fragment Fix.
        let advertised_max = req.max_length.min(512 * 1024);
        self.compression_enabled = true; // Enabled for RoF2
        
        let mut response = Vec::with_capacity(27);
        
        response.put_u16(0x0002); // SessionResponse
        response.put_u32_le(self.session_id);
        response.put_u32_le(self.crc_key);
        response.put_u8(0x02); // Flag: 2 = Compressed (Zlib)
        response.put_u8(0x01); // Standard Unk
        response.put_u32_le(advertised_max); 
        response.put_u32_le(0);   
        response.put_u32_le(131072); 
        response.put_u32_le(4096); 
        
        response
    }
    
    pub fn process_packet(&mut self, opcode: u16, payload: &[u8]) -> Vec<ProcessPacketResult> {
        match self.process_packet_internal(opcode, payload) {
            Ok(results) => results,
            Err(e) => {
                warn!("Protocol Error: {}", e);
                Vec::new()
            }
        }
    }

    fn validate_crc(&self, opcode: u16, data: &[u8]) -> bool {
        if data.len() < 2 { return false; }
        let packet_crc = u16::from_le_bytes([data[data.len()-2], data[data.len()-1]]);
        let mut digest = EQ_CRC.digest();
        
        // The CRC is calculated over [Opcode(BE)] + [Payload] + [CRC_Key(BE)]
        digest.update(&opcode.to_be_bytes());
        digest.update(&data[..data.len()-2]); // Data (excluding the CRC itself)
        digest.update(&self.crc_key.to_be_bytes()); // Key LAST (Big Endian)
        
        let crc32 = digest.finalize();
        let calced = (crc32 & 0xFFFF) as u16;
        packet_crc == calced
    }

    fn process_packet_internal(&mut self, opcode: u16, payload: &[u8]) -> Result<Vec<ProcessPacketResult>, ProtocolError> {
        // debug!("RX Raw: Op={:04x} Len={} Data={:02x?}", opcode, payload.len(), payload); 
        // Only log relevant ones to reduce noise, or log all for this debug session
        if payload.len() > 0 && payload.len() < 100 {
             info!("RX Raw Packet: Op={:04x} Len={} Data={:02x?}", opcode, payload.len(), payload);
        }
        
        let mut results = Vec::new();
        
        // CRC Validation and Stripping
        // If Key is 0, assume No CRC (Common in Login Server / RoF2 Discovery)
        let checked_payload = if self.crc_key != 0 {
            if payload.len() >= 2 {
                 if !self.validate_crc(opcode, payload) {
                     warn!("CRC Validation Failed for Op {:04x}", opcode);
                     // return Ok(vec![]); // Logic: Drop? Or try processing anyway?
                 }
                 &payload[..payload.len()-2]
            } else {
                 payload
            }
        } else {
            // Key is 0: Assume No CRC appended by client
            payload
        };

        match opcode {
            0x09 => { // OP_Packet (Sequenced)
                if checked_payload.len() < 2 { return Err(ProtocolError::MalformedPayload); }
                let sequence = u16::from_be_bytes([checked_payload[0], checked_payload[1]]);
                
                // Auto-Sync ISN on first packet
                if self.packets_received == 0 {
                     // If client starts at random seq (e.g. 0x1907), accept it.
                     self.last_received_sequence = sequence.wrapping_sub(1);
                     info!("Auto-Synced Initial Sequence to {}", sequence);
                }

                let next_seq = self.last_received_sequence.wrapping_add(1);
                
                // println!("DEBUG: OP_Packet Seq={} Expected={} Last={}", sequence, next_seq, self.last_received_sequence);
                // let _ = std::io::stdout().flush();
                
                if sequence == next_seq {
                    self.last_received_sequence = sequence;
                    self.packets_received += 1;
                    
                    results.push(ProcessPacketResult::Response(self.create_ack(sequence)));
                    
                    if checked_payload.len() >= 4 {
                        let app_opcode_raw = u16::from_le_bytes([checked_payload[2], checked_payload[3]]);
                        let app_opcode = OpCode::from_u16(app_opcode_raw).unwrap_or(OpCode::Unknown);
                        
                        // Handle Combined (Nested)
                        if app_opcode == OpCode::Unknown && (app_opcode_raw == 0x19 || app_opcode_raw == 0x1900) {
                             results.extend(self.process_combined_data(0x19, &checked_payload[4..]));
                        } else {
                             results.push(ProcessPacketResult::Application(app_opcode, checked_payload[4..].to_vec()));
                        }
                    }
                } else if sequence == self.last_received_sequence {
                     results.push(ProcessPacketResult::Response(self.create_ack(sequence)));
                } else if sequence > next_seq {
                     results.push(ProcessPacketResult::Response(self.create_out_of_order(next_seq)));
                }
            },
            0x19 | 0x03 if self.combined_enabled => { // OP_Combined or OP_AppCombined
                results.extend(self.process_combined_data(opcode, checked_payload));
            },
            0x07 => { // OP_SessionStatRequest
                let request_id = if checked_payload.len() >= 2 { u16::from_be_bytes([checked_payload[0], checked_payload[1]]) } else { 0 };
                results.push(ProcessPacketResult::Response(self.create_stat_response(request_id)));
            },
            0x0d | 0x0e | 0x0f | 0x10 => { // OP_Fragment
                 if checked_payload.len() < 2 { return Err(ProtocolError::MalformedPayload); }
                 let sequence = u16::from_be_bytes([checked_payload[0], checked_payload[1]]);
                 let next_seq = self.last_received_sequence.wrapping_add(1);
                 
                 if sequence == next_seq {
                     self.last_received_sequence = sequence;
                     results.push(ProcessPacketResult::Response(self.create_ack(sequence)));
                     if let Some(res) = self.process_fragment_data(opcode, checked_payload)? {
                         results.push(res);
                     }
                     // Drain OOO
                     while let Some(stashed) = self.ooo_buffer.remove(&self.last_received_sequence.wrapping_add(1)) {
                         self.last_received_sequence = self.last_received_sequence.wrapping_add(1);
                         results.push(ProcessPacketResult::Response(self.create_ack(self.last_received_sequence)));
                         // OOO stashed payloads should already have CRC stripped? 
                         // No, we inserted payload.to_vec() which was checked_payload (stripped) or raw?
                         // See below: we insert checked_payload.to_vec()
                         if let Some(res) = self.process_fragment_data(opcode, &stashed)? {
                             results.push(res);
                         }
                     }
                 } else if sequence > next_seq {
                     self.ooo_buffer.insert(sequence, checked_payload.to_vec());
                     results.push(ProcessPacketResult::Response(self.create_out_of_order(next_seq)));
                 }
            },
            0x15 => { // OP_Ack
                if payload.len() >= 2 {
                    let seq = u16::from_be_bytes([payload[0], payload[1]]);
                    self.sent_packets.remove(&seq);
                    if seq > self.last_acked_sequence { self.last_acked_sequence = seq; }
                }
            },
            0x11 => { // OP_OutOfOrder
                if payload.len() >= 2 {
                    let seq = u16::from_be_bytes([payload[0], payload[1]]);
                    if let Some(p) = self.sent_packets.get(&seq) {
                        results.push(ProcessPacketResult::Response(p.clone()));
                    }
                }
            },
             _ => {}
        }
        Ok(results)
    }

    fn process_combined_data(&mut self, opcode: u16, payload: &[u8]) -> Vec<ProcessPacketResult> {
        let mut results = Vec::new();
        let mut offset = 0;
        
        while offset < payload.len() {
            let len = if opcode == 0x19 {
                payload[offset] as usize
            } else {
                payload[offset] as usize
            };
            offset += 1;
            
            if offset + len > payload.len() { break; }
            let sub = &payload[offset..offset + len];
            offset += len;
            
            if sub.len() >= 2 {
                let sub_op_raw = u16::from_le_bytes([sub[0], sub[1]]);
                let sub_op = OpCode::from_u16(sub_op_raw).unwrap_or(OpCode::Unknown);
                results.push(ProcessPacketResult::Application(sub_op, sub[2..].to_vec()));
            }
        }
        results
    }

    fn process_fragment_data(&mut self, opcode: u16, payload: &[u8]) -> Result<Option<ProcessPacketResult>, ProtocolError> {
        let stream = self.fragment_streams.entry(opcode).or_default();
        let mut data_offset = 2; // skip seq
        
        if stream.buffer.is_empty() {
             if payload.len() < 6 { return Err(ProtocolError::MalformedPayload); }
             stream.total_len = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize; // Big Endian
             data_offset = 6;
        }
        
        stream.buffer.extend_from_slice(&payload[data_offset..]);
        
        if stream.buffer.len() >= stream.total_len {
            let full_data = std::mem::take(&mut stream.buffer);
            let mut data = full_data;
            
            // Handle Optional Compression in Fragments
            if data.len() >= 2 && data[0] == 0x5a && data[1] == 0x00 {
                if data.len() < 6 { return Err(ProtocolError::MalformedPayload); }
                let decompressed_len = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
                let mut decoder = flate2::read::ZlibDecoder::new(&data[6..]);
                let mut decompressed = Vec::with_capacity(decompressed_len);
                if std::io::copy(&mut decoder, &mut decompressed).is_ok() {
                    data = decompressed;
                }
            }
            
            if data.len() >= 2 {
                let app_opcode_raw = u16::from_le_bytes([data[0], data[1]]);
                let app_opcode = OpCode::from_u16(app_opcode_raw).unwrap_or(OpCode::Unknown);
                return Ok(Some(ProcessPacketResult::Application(app_opcode, data[2..].to_vec())));
            }
        }
        Ok(None)
    }

    pub async fn create_reliable_packets(&mut self, app_opcode: OpCode, data: &[u8]) -> Vec<Vec<u8>> {
        self.create_raw_packets(app_opcode, data)
    }

    pub fn create_raw_packets(&mut self, app_opcode: OpCode, data: &[u8]) -> Vec<Vec<u8>> {
        let mut payload = data.to_vec();
        
        if app_opcode == OpCode::SendCharInfo { // OP_SendCharInfo (0x00d2)
             info!("create_raw_packets: OP_SendCharInfo (0x00d2). data.len()={}, compression_enabled={}", data.len(), self.compression_enabled);
        }

        // Catch-all for large packets to see if compression logic is triggering
        if data.len() > 250 {
             info!("create_raw_packets: Large Packet (Opcode {:?}). data.len()={}, compression_enabled={}", app_opcode, data.len(), self.compression_enabled);
        }

        if self.compression_enabled && data.len() >= 100 {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(data).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    let mut comp_layer = Vec::with_capacity(6 + compressed.len());
                    comp_layer.push(0x5a);
                    comp_layer.push(0x00);
                    comp_layer.put_u32_le(data.len() as u32);
                    comp_layer.extend_from_slice(&compressed);
                    payload = comp_layer;
                }
            }
        }

        let mut app_layer = Vec::with_capacity(payload.len() + 2);
        app_layer.put_u16_le(app_opcode as u16);
        app_layer.put_slice(&payload);
        
        let _max_app_payload = (self.max_length as usize).saturating_sub(6);
        
        let max_sequenced = (self.max_length as usize).saturating_sub(6);
        if app_layer.len() <= max_sequenced {
            let mut packet = Vec::with_capacity(6 + app_layer.len());
            packet.put_u8(0x00);
            packet.put_u8(0x09);
            let seq = self.sequence_out;
            packet.put_u16(seq); 
            self.sequence_out = self.sequence_out.wrapping_add(1);
            
            packet.put_slice(&app_layer);
            
            self.append_crc(&mut packet);
            self.sent_packets.insert(seq, packet.clone());
            self.packets_sent += 1;
            return vec![packet];
        }
        
        let mut fragments = Vec::new();
        let mut offset = 0;
        let total_len = app_layer.len() as u32;
        
        while offset < app_layer.len() {
            let is_first = offset == 0;
            let header_size = if is_first { 8 } else { 4 };
            let chunk_size = (self.max_length as usize).saturating_sub(header_size + 2).min(app_layer.len() - offset);
            
            let mut packet = Vec::with_capacity(header_size + chunk_size + 2);
            packet.put_u8(0x00);
            packet.put_u8(0x0d);
            let seq = self.sequence_out;
            self.sequence_out = self.sequence_out.wrapping_add(1);
            if is_first { packet.put_u32(total_len); } // Big Endian (Standard)
            packet.put_slice(&app_layer[offset..offset + chunk_size]);
            offset += chunk_size;
            self.append_crc(&mut packet);
            self.sent_packets.insert(seq, packet.clone());
            self.packets_sent += 1;
            fragments.push(packet);
        }
        fragments
    }

    pub fn append_crc(&self, packet: &mut Vec<u8>) {
        if self.crc_key != 0 {
            let mut digest = EQ_CRC.digest();
            digest.update(packet); // Data FIRST
            digest.update(&self.crc_key.to_be_bytes()); // Key LAST (Big Endian)
            let crc32 = digest.finalize();
            // EQStream uses LITTLE-ENDIAN for the 16-bit CRC!
            packet.put_u16_le((crc32 & 0xFFFF) as u16);
        }
    }

    pub fn create_ack(&self, sequence: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(6);
        packet.put_u8(0); packet.put_u8(0x15); packet.put_u16(sequence);
        self.append_crc(&mut packet);
        packet
    }
    
    pub fn create_out_of_order(&self, expected: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(6);
        packet.put_u8(0); packet.put_u8(0x11); packet.put_u16(expected);
        self.append_crc(&mut packet);
        packet
    }

    pub fn create_disconnect(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(8);
        packet.put_u8(0); packet.put_u8(0x05); packet.put_u32(self.session_id);
        self.append_crc(&mut packet);
        packet
    }

    pub fn create_stat_request(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(42);
        packet.put_u8(0); packet.put_u8(0x07); 
        packet.put_slice(&[0u8; 38]); 
        self.append_crc(&mut packet);
        packet
    }
    
    pub fn create_stat_response(&self, request_id: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(42);
        packet.put_u8(0); packet.put_u8(0x08); packet.put_u16(request_id);
        packet.put_u32(0); // Time
        packet.put_u32(self.packets_sent as u32);
        packet.put_u32(self.packets_received as u32);
        packet.put_u16(self.sequence_out);
        packet.put_u16(self.last_received_sequence);
        packet.put_slice(&[0u8; 20]);
        self.append_crc(&mut packet);
        packet
    }
}
