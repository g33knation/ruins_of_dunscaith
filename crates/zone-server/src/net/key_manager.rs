use tokio::sync::{mpsc, oneshot};

// --- Messages ---
#[derive(Debug)]
pub enum KeyManagerRequest {
    RegisterKey {
        account_id: u32, // or session_id? World usually sends AccountID + Key + SessionID
        session_key: [u8; 32],
    },
    ValidateKey {
        account_id: u32,
        // session_key: [u8; 32], // Validating against stored
        session_id: u32, // Often client sends SessionID
        respond_to: oneshot::Sender<bool>,
    }
}

// --- Actor ---
pub struct ZoneKeyManager {
    rx: mpsc::Receiver<KeyManagerRequest>,
    pool: sqlx::PgPool, 
}

impl ZoneKeyManager {
    pub fn new(pool: sqlx::PgPool) -> (Self, mpsc::Sender<KeyManagerRequest>) {
        let (tx, rx) = mpsc::channel(32);
        (Self { rx, pool }, tx)
    }

    pub async fn run(mut self) {
        while let Some(req) = self.rx.recv().await {
            match req {
                KeyManagerRequest::RegisterKey { account_id, session_key: _ } => {
                    log::info!("ZoneKeyManager: RegisterKey ignored for Account {}", account_id);
                },
                KeyManagerRequest::ValidateKey { account_id, session_id: _, respond_to } => {
                     // Verify against DB accounts table
                     let res = sqlx::query!("SELECT id FROM accounts WHERE id = $1", account_id as i32)
                        .fetch_optional(&self.pool)
                        .await;
                        
                     let is_valid = match res {
                         Ok(Some(_)) => true,
                         _ => false,
                     };
                     
                     let _ = respond_to.send(is_valid);
                }
            }
        }
    }
}
