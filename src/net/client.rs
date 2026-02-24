use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use crate::net::db::DbRequest;
use crate::net::login_packets::{self, LoginRequest, LoginResponse, ServerListResponse, LoginClientServerData, SessionKey, PlayRequest, PlayResponse};
use crate::net::packet::LoginOpCode;
use rand::Rng;

pub enum ClientEvent {
    Login(LoginRequest),
    ServerList,
    PlayRequest(PlayRequest),
    Disconnect,
}

pub struct ClientSystem {
    pub id: Uuid,
    pub account_id: Option<i32>, // Track internal state
    pub db_sender: mpsc::Sender<DbRequest>,
    pub app_sender: mpsc::Sender<Vec<u8>>, 
}

impl ClientSystem {
    pub fn new(id: Uuid, db_sender: mpsc::Sender<DbRequest>, app_sender: mpsc::Sender<Vec<u8>>) -> Self {
        Self { 
            id, 
            account_id: None,
            db_sender, 
            app_sender 
        }
    }

    pub async fn handle_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Login(req) => self.handle_login(req).await,
            ClientEvent::ServerList => self.handle_server_list().await,
            ClientEvent::PlayRequest(req) => self.handle_play_request(req).await,
            ClientEvent::Disconnect => {
                log::info!("Client {} disconnected (Logic)", self.id);
            }
        }
    }

    async fn handle_login(&mut self, req: LoginRequest) {
        log::info!("Client {} processing login for '{}'", self.id, req.username);
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.db_sender.send(DbRequest::Authenticate {
            username: req.username.clone(),
            password_hash: req.password.clone(),
            respond_to: tx,
        }).await {
            log::error!("Client {} failed to send DB request: {}", self.id, e);
            self.send_login_response(false, 0, 0).await;
            return;
        }

        match rx.await {
            Ok(Some(account_id)) => {
                log::info!("Client {} Authenticated. Account: {}", self.id, account_id);
                self.account_id = Some(account_id); // set state
                self.send_login_response(true, account_id as u32, 1).await;
            },
            Ok(None) => {
                log::warn!("Client {} Auth Failed (Invalid Creds)", self.id);
                self.send_login_response(false, 0, 0).await;
            },
            Err(e) => {
                log::error!("Client {} DB Response Error: {}", self.id, e);
                self.send_login_response(false, 0, 0).await;
            }
        }
    }

    async fn handle_play_request(&mut self, req: PlayRequest) {
        log::info!("Client {} PlayRequest for Server {}", self.id, req.server_id);
        
        let account_id = match self.account_id {
            Some(id) => id,
            None => {
                log::warn!("Client {} tried to Play without Auth", self.id);
                return;
            }
        };

        // 1. Get World Server IP
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.db_sender.send(DbRequest::GetWorldServer {
            server_id: req.server_id as i32, 
            respond_to: tx
        }).await {
             log::error!("DB Send Error (GetWorldServer): {}", e);
             return;
        }

        let server_ip = match rx.await {
            Ok(Ok(Some(ip))) => ip,
            Ok(Ok(None)) => {
                log::error!("World Server {} Not Found", req.server_id);
                return; // Send specific error packet?
            },
            Ok(Err(e)) => {
                log::error!("DB Error fetching World Server: {}", e);
                return;
            },
            Err(e) => {
                log::error!("Channel Error (GetWorldServer): {}", e);
                return;
            }
        };

        // 2. Generate Key (u32)
        let key: u32 = rand::thread_rng().gen();
        
        // 3. Update DB
        let (tx_upd, rx_upd) = oneshot::channel();
        // Schema is BIGINT, so pass as i64. u32 fits in i64.
        if let Err(e) = self.db_sender.send(DbRequest::SetSessionKey {
            account_id,
            key: key,
            respond_to: tx_upd
        }).await {
             log::error!("DB Send Error (SetSessionKey): {}", e);
             return;
        }

        // 4. Response
        match rx_upd.await {
            Ok(true) => {
                let resp = PlayResponse {
                    server_ip,
                    session_key: key,
                    success: 1,
                };
                
                // Serialize & Send
                use binrw::BinWrite;
                use std::io::Cursor;
                use crate::net::packet::PacketHeader;
                
                let mut payload = Vec::new();
                if let Ok(_) = resp.write(&mut Cursor::new(&mut payload)) {
                    let header = PacketHeader {
                        size: payload.len() as u16,
                        opcode: LoginOpCode::PlayEverquestResponse as u16,
                    };
                    let mut out = Vec::new();
                    if let Ok(_) = header.write(&mut Cursor::new(&mut out)) {
                        out.extend(payload);
                        let _ = self.app_sender.send(out).await;
                    }
                }
            },
            Ok(false) => {
                 log::error!("Client {} Failed to set SessionKey in DB", self.id);
            },
            Err(e) => {
                 log::error!("Critical: DB Worker Error (SetSessionKey): {}", e);
            }
        }
    }

    async fn handle_server_list(&mut self) {
        let servers = vec![
            LoginClientServerData {
                ip: "127.0.0.1".to_string(),
                server_type: 0,
                server_id: 1,
                server_name: "RustTestServer".to_string(),
                country_code: "US".to_string(),
                language_code: [0],
                server_status: 1, 
                player_count: 0,
            }
        ];
        
        let resp = ServerListResponse {
            server_count: servers.len() as u32,
            servers,
        };
        
        // Serialize
        use binrw::BinWrite;
        use std::io::Cursor;
        use crate::net::packet::PacketHeader;
        
        let mut payload = Vec::new();
        if let Ok(_) = resp.write(&mut Cursor::new(&mut payload)) {
             let header = PacketHeader {
                size: payload.len() as u16,
                opcode: LoginOpCode::ServerListResponse as u16,
            };
            let mut out = Vec::new();
            if let Ok(_) = header.write(&mut Cursor::new(&mut out)) {
                out.extend(payload);
                let _ = self.app_sender.send(out).await;
            }
        }
    }

    async fn send_login_response(&self, success: bool, account_id: u32, result: u32) {
        let resp = LoginResponse {
            result: if success { 1 } else { 0 }, // 1 = Success
            account_id,
            session_key: SessionKey::new(if success { [0xAA; 30] } else { [0x00; 30] }),
        };

        use binrw::BinWrite;
        use std::io::Cursor;
        use crate::net::packet::PacketHeader;

        let mut payload = Vec::new();
        if let Ok(_) = resp.write(&mut Cursor::new(&mut payload)) {
             let header = PacketHeader {
                size: payload.len() as u16,
                opcode: LoginOpCode::LoginAccepted as u16,
            };
            let mut out = Vec::new();
            if let Ok(_) = header.write(&mut Cursor::new(&mut out)) {
                out.extend(payload);
                let _ = self.app_sender.send(out).await;
            }
        }
    }
}
