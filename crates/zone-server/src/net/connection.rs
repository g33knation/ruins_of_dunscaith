use tokio::sync::mpsc;
use tokio::time::{Instant, Duration};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use bytes::{Bytes, BytesMut, Buf};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write, Cursor};
use binrw::{BinRead, BinWrite};

// --- Safety: Error Types ---
#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("Encryption Error: {0}")]
    Encryption(String),
    #[error("Compression Error: {0}")]
    Compression(std::io::Error),
    #[error("Invalid Sequence: Expected {expected}, Got {actual}")]
    InvalidSequence { expected: u16, actual: u16 },
    #[error("Join Error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

// --- Component: ClientCrypto ---
// Encapsulates XOR and Compression logic.
pub struct ClientCrypto {
    xor_key: u32,
}

impl ClientCrypto {
    pub fn new() -> Self {
        Self { xor_key: 0 }
    }
    
    pub fn set_key(&mut self, key: u32) {
        self.xor_key = key;
    }

    // CPU-bound: Decrypt and Decompress
    pub fn transform_incoming(&mut self, mut data: Vec<u8>) -> Result<Vec<u8>, PacketError> {
        // 1. XOR Decryption (Stub)
        // Check for 'raw' packets? Or apply to all?
        // Logic: if data.len() > 2 ...
        // self.xor_rotate(&mut data);
        
        // 2. Decompression
        // Header 0x5a 0xa5
        if data.len() > 2 && data[0] == 0x5a && data[1] == 0xa5 {
             // Strip header? Usually Zlib stream follows
             // Assuming header is custom, then zlib.
             // If standard zlib header, ZlibDecoder handles it.
             // Let's peek at [2..].
             let mut decoder = ZlibDecoder::new(&data[2..]);
             let mut decompressed = Vec::new();
             if let Err(e) = decoder.read_to_end(&mut decompressed) {
                  return Err(PacketError::Compression(e));
             }
             data = decompressed;
        }

        Ok(data)
    }

    // CPU-bound: Compress and Encrypt
    pub fn transform_outgoing(&mut self, mut data: Vec<u8>) -> Result<Vec<u8>, PacketError> {
         // 1. Compress
         if data.len() > 500 {
             let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
             if let Err(e) = encoder.write_all(&data) {
                 return Err(PacketError::Compression(e));
             }
             if let Ok(compressed) = encoder.finish() {
                 let mut new_buf = vec![0x5a, 0xa5];
                 new_buf.extend_from_slice(&compressed);
                 data = new_buf;
             }
         }
         
         // 2. Encrypt
         // self.xor_rotate(&mut data);

         Ok(data)
    }
}

// --- Reliability Types ---
#[derive(Debug)]
pub enum ReliabilityEvent {
    PacketToResend(Bytes),
    MaxRetriesReached(u16),
}

struct SentPacket {
    seq: u16,
    payload: Bytes,
    sent_at: Instant,
    retries: u8,
}

// --- Component: ReliabilityLayer ---
pub struct ReliabilityLayer {
    pub seq_out: u16,
    pub seq_in: u16,
    unacked: BTreeMap<u16, SentPacket>,
    rto: Duration,
}

impl ReliabilityLayer {
    pub fn new() -> Self {
        Self { 
            seq_out: 0, 
            seq_in: 0,
            unacked: BTreeMap::new(),
            rto: Duration::from_millis(250),
        }
    }
    
    pub fn next_seq(&mut self) -> u16 {
        let s = self.seq_out;
        self.seq_out = self.seq_out.wrapping_add(1);
        s
    }

    pub fn enqueue(&mut self, packet_data: Bytes, seq: u16) {
        self.unacked.insert(seq, SentPacket {
            seq,
            payload: packet_data,
            sent_at: Instant::now(),
            retries: 0,
        });
    }

    pub fn ack(&mut self, acked_seq: u16) {
        if self.unacked.remove(&acked_seq).is_some() {
            log::debug!("Acked packet {}", acked_seq);
        }
    }
    
    pub fn can_accept(&mut self, seq: u16) -> bool {
        if seq == self.seq_in {
            self.seq_in = self.seq_in.wrapping_add(1);
            return true;
        }
        // Basic retransmit/duplicate handling implied (drop if != seq_in for now)
        // In real impl, we buffer future packets.
        false
    }

    pub fn get_retransmissions_batched(&mut self, max_batch_size: usize) -> Vec<ReliabilityEvent> {
        let now = Instant::now();
        let mut events = Vec::new();
        let mut candidates_for_resend = Vec::new();

        for (&seq, packet) in &self.unacked {
            if now.duration_since(packet.sent_at) > self.rto {
                candidates_for_resend.push(seq);
                if candidates_for_resend.len() >= max_batch_size {
                    break;
                }
            }
        }

        for seq in candidates_for_resend {
            if let Some(packet) = self.unacked.get_mut(&seq) {
                packet.sent_at = now;
                packet.retries += 1;
                if packet.retries < 10 {
                    events.push(ReliabilityEvent::PacketToResend(packet.payload.clone()));
                } else {
                    events.push(ReliabilityEvent::MaxRetriesReached(packet.seq));
                    self.unacked.remove(&seq); 
                }
            }
        }
        events
    }
}

// --- Client Socket ---
pub struct ClientSocket {
    addr: SocketAddr,
    outbound_tx: mpsc::Sender<(Bytes, SocketAddr)>,
    reliability: ReliabilityLayer,
    crypto: Option<ClientCrypto>, // Option to allow moving into closure
}

impl ClientSocket {
    pub fn new(addr: SocketAddr, outbound_tx: mpsc::Sender<(Bytes, SocketAddr)>) -> Self {
        Self {
            addr,
            outbound_tx,
            reliability: ReliabilityLayer::new(),
            crypto: Some(ClientCrypto::new()), 
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    
    pub fn set_session_key(&mut self, key: u32) {
        if let Some(c) = self.crypto.as_mut() {
            c.set_key(key);
        }
    }

    // Async pipeline using spawn_blocking for CPU work
    pub async fn process_incoming(&mut self, data: Vec<u8>) -> Result<Option<Vec<u8>>, PacketError> {
        // 1. Move Crypto State and Data to Blocking Task
        let mut crypto = self.crypto.take().expect("Crypto state missing");
        
        let result = tokio::task::spawn_blocking(move || {
            let res = crypto.transform_incoming(data);
            (res, crypto)
        }).await;

        // 2. Restore State and Handle Result
        match result {
            Ok((transform_res, returned_crypto)) => {
                self.crypto = Some(returned_crypto); // Restore
                
                let data = transform_res?; // Propagate PacketError
                
                // 3. Reliability Check (main thread, fast logic)
                if data.len() >= 4 {
                     let opcode_le = u16::from_le_bytes([data[0], data[1]]);
                     if opcode_le == 0x0015 { // OP_Ack
                          let seq = u16::from_le_bytes([data[2], data[3]]);
                          self.reliability.ack(seq);
                          return Ok(None); 
                     }
                     
                     let seq = u16::from_le_bytes([data[2], data[3]]);
                     if !self.reliability.can_accept(seq) {
                          // Log duplicate/old?
                          return Ok(None); 
                     }
                     
                     self.send_ack(seq);
                }
                Ok(Some(data))
            },
            Err(join_err) => {
                // Fatal: Thread panic or pool closed. Crypto state lost.
                self.crypto = Some(ClientCrypto::new()); // Reset or let die
                Err(PacketError::Join(join_err))
            }
        }
    }

    pub async fn send_packet(&mut self, opcode: u16, payload: &[u8]) -> Result<(), PacketError> {
         let seq = self.reliability.next_seq();
         
         let mut buf = Vec::with_capacity(4 + payload.len());
         buf.extend_from_slice(&opcode.to_le_bytes());
         buf.extend_from_slice(&seq.to_le_bytes());
         buf.extend_from_slice(payload);
         
         // Offload Compression/Encryption
         let mut crypto = self.crypto.take().expect("Crypto missing");
         
         let result = tokio::task::spawn_blocking(move || {
             let res = crypto.transform_outgoing(buf);
             (res, crypto)
         }).await;
         
         match result {
             Ok((transform_res, returned_crypto)) => {
                 self.crypto = Some(returned_crypto);
                 let final_data = transform_res?;
                 
                 let bytes = Bytes::from(final_data);
                 self.reliability.enqueue(bytes.clone(), seq);
                 let _ = self.outbound_tx.send((bytes, self.addr)).await;
                 Ok(())
             },
             Err(join_err) => {
                 self.crypto = Some(ClientCrypto::new());
                 Err(PacketError::Join(join_err))
             }
         }
    }
    
    fn send_ack(&mut self, seq: u16) {
        let mut buf = Vec::new();
        let opcode = 0x0015u16; 
        buf.extend_from_slice(&opcode.to_le_bytes());
        buf.extend_from_slice(&seq.to_le_bytes());
        let bytes = Bytes::from(buf);
        let _ = self.outbound_tx.try_send((bytes, self.addr));
    }

    pub async fn tick(&mut self) -> bool {
        let events = self.reliability.get_retransmissions_batched(50);
        for event in events {
            match event {
                ReliabilityEvent::PacketToResend(p) => {
                     let _ = self.outbound_tx.send((p, self.addr)).await;
                },
                ReliabilityEvent::MaxRetriesReached(_) => return false,
            }
        }
        true
    }
}
