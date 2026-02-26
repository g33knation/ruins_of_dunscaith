use tokio::net::UdpSocket;
use tracing::{error, info};
use shared::net::eq_stream::{parse_eqstream, EQStreamPacket};
use crate::net::eqstream::EqStreamSession; // Local wrapper
use bytes::BufMut;
use std::io::Write; // Import flush

pub async fn start_discovery_listener(port: u16, pool: Option<sqlx::PgPool>) -> anyhow::Result<()> {
    // Bind to 0.0.0.0 to catch traffic on all interfaces (Loopback, LAN, etc.)
    let addr = format!("0.0.0.0:{}", port);
    let sock = UdpSocket::bind(&addr).await?;
    info!("UDP Login Server bound to {}", addr);

    let mut sessions: std::collections::HashMap<std::net::SocketAddr, EqStreamSession> = std::collections::HashMap::new();

    loop {
        let mut buf = [0u8; 1024];
        match sock.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let data = &buf[..len];
                    info!("UDP RX from {}: {:02x?}", addr, data);
 
                match parse_eqstream(data) {
                    Ok((_, packet)) => {
                        match packet {
                            EQStreamPacket::SessionRequest(req) => {
                                info!("EQStream Session Request from {}: Protocol={} SessionID={:08X} MaxLen={}", 
                                      addr, req.protocol_version, req.session_id, req.max_length);
                                
                                let mut session = EqStreamSession::new(req.session_id, pool.clone());
                                let response = session.handle_session_request(&req);
                                
                                sessions.insert(addr, session);
                                
                                sock.send_to(&response, addr).await?;
                                info!("Sent Session Response to {}: {:02x?}", addr, response);
                            },
                            EQStreamPacket::Unknown(opcode, payload) => {
                                if let Some(session) = sessions.get_mut(&addr) {
                                    let responses = session.receive_packet(opcode, &payload).await;
                                     for resp in responses {
                                         info!("Sending Response to {}: {:02x?}", addr, resp);
                                         let _ = sock.send_to(&resp, addr).await;
                                         tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                                     }
                                } else {
                                    // info!("Unknown Packet from Unknown Session {}: Op={:04X}", addr, opcode);
                                }
                            }
                            _ => {
                                // info!("Unhandled Packet Type from {}", addr);
                            }
                        }
                    },

                    Err(e) => {
                        error!("Failed to parse packet from {}: {}", addr, e);
                    }
                }
            }
            Err(e) => error!("UDP Error: {}", e),
        }
    }
}
