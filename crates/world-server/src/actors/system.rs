
use tokio::sync::mpsc;
use std::net::SocketAddr;
use std::collections::HashMap;
use tracing::{info, warn, error};
use std::sync::Arc;
use crate::db::DatabaseManager;
use crate::actors::session::ClientSessionActor;
use shared::net::eq_stream::{parse_eqstream, EQStreamPacket};

pub struct ClientSystemActor {
    rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>, // Parsing Input from Socket
    socket_tx: mpsc::Sender<(SocketAddr, Vec<u8>)>, // Output to Socket
    db: Arc<DatabaseManager>,
    sessions: HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>,
}

impl ClientSystemActor {
    pub fn new(
        rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
        socket_tx: mpsc::Sender<(SocketAddr, Vec<u8>)>,
        db: Arc<DatabaseManager>,
    ) -> Self {
        Self {
            rx,
            socket_tx,
            db,
            sessions: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        info!("ClientSystemActor started.");

        while let Some((addr, data)) = self.rx.recv().await {
            // Check if session exists
            if let Some(session_tx) = self.sessions.get(&addr) {
                // Forward packet to session
                if let Err(_) = session_tx.send(data.clone()).await {
                    // Session died?
                    warn!("Session {} channel closed, removing.", addr);
                    self.sessions.remove(&addr);
                }
                continue;
            }

            // New Session?
            // Need to peek if it's a SessionRequest (0x01)
            // Or parse_eqstream to check
            match parse_eqstream(&data) {
                Ok((_, pkt)) => {
                    match pkt {
                        EQStreamPacket::SessionRequest(req) => {
                            info!("New Session Request from {}. Spawning Actor.", addr);
                            
                            let (session_tx, mut session_rx) = mpsc::channel(100);
                            self.sessions.insert(addr, session_tx);
                            
                            // Spawn Session Actor
                            let socket_tx_clone = self.socket_tx.clone();
                            let db_clone = self.db.clone();
                            let session_id = req.session_id; // Use Request ID
                            
                            tokio::spawn(async move {
                                let mut actor = ClientSessionActor::new(
                                    addr,
                                    session_id,
                                    socket_tx_clone,
                                    db_clone
                                );
                                
                                actor.run(session_rx).await;
                            });
                            
                            // Send proper Handshake Response immediately?
                            // Access to ws.session... 
                            // Easier to spawn and send the packet "to itself".
                            if let Some(tx) = self.sessions.get(&addr) {
                                let _ = tx.send(data).await;
                            }
                        },
                        _ => {
                            // Unknown packet from unknown address. Ignore.
                            // info!("Ignored packet from unknown {}", addr);
                        }
                    }
                },
                Err(_) => {}
            }
        }
    }
}
