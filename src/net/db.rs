use tokio::sync::{mpsc, oneshot};
use sqlx::{PgPool, FromRow};
use std::sync::Arc;

#[derive(Debug)]
pub enum DbRequest {
    Authenticate {
        username: String,
        password_hash: String, 
        respond_to: oneshot::Sender<Option<i32>>,
    },
    SetSessionKey {
        account_id: i32,
        key: u32,
        respond_to: oneshot::Sender<bool>,
    },
    GetWorldServer {
        server_id: i32,
        respond_to: oneshot::Sender<Result<Option<String>, sqlx::Error>>,
    },
}

#[derive(FromRow)]
struct WorldIpRow { ip_address: String }


pub struct DbWorker {
    pub pool: Arc<PgPool>, // Use Arc<PgPool> or just PgPool (which is cheap to clone/Arc internally)
    pub rx: mpsc::Receiver<DbRequest>,
}

#[derive(FromRow)]
struct AccountIdRow {
    id: i32,
}

impl DbWorker {
    pub async fn run(mut self) {
        log::info!("DbWorker started");
        while let Some(req) = self.rx.recv().await {
            match req {
                DbRequest::Authenticate { username, password_hash, respond_to } => {
                    // Strict usage of query_as!
                    // Note: We use the seeded 'test'/'test' for now.
                    let result = sqlx::query_as!(
                        AccountIdRow,
                        "SELECT id FROM accounts WHERE username = $1 AND password_hash = $2",
                        username,
                        password_hash
                    )
                    .fetch_optional(self.pool.as_ref())
                    .await;

                    match result {
                        Ok(opt) => {
                            let _ = respond_to.send(opt.map(|r| r.id));
                        },
                        Err(e) => {
                            log::error!("DB Auth Error: {}", e);
                            let _ = respond_to.send(None);
                        }
                    }
                },
                DbRequest::SetSessionKey { account_id, key, respond_to } => {
                    let res = sqlx::query!(
                        "UPDATE accounts SET session_key = $1 WHERE id = $2",
                        key as i64,
                        account_id
                    )
                    .execute(self.pool.as_ref())
                    .await;

                    match res {
                        Ok(_) => { let _ = respond_to.send(true); },
                        Err(e) => {
                            log::error!("DB SetSessionKey Error: {}", e);
                            let _ = respond_to.send(false);
                        }
                    }
                },
                DbRequest::GetWorldServer { server_id, respond_to } => {
                    let res = sqlx::query_as!(
                        WorldIpRow,
                        "SELECT ip_address FROM world_servers WHERE id = $1",
                        server_id
                    )
                    .fetch_optional(self.pool.as_ref())
                    .await;
                    
                    let reply = res.map(|opt| opt.map(|r| r.ip_address));
                    let _ = respond_to.send(reply);
                }
            }
        }
        log::info!("DbWorker stopped");
    }
}
