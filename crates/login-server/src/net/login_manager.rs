use crate::net::client_socket::ClientSocket;
use crate::net::login_session::LoginSessionActor;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct LoginManager {
    pool: sqlx::PgPool,
}

impl LoginManager {
    pub async fn new(pool: sqlx::PgPool) -> anyhow::Result<Self> {
        Ok(Self { pool })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Bind TCP to 5998 and 5999 (Akk-Stack/Modern LS Ports)
        let ports = [5998, 5999];
        let mut futures = Vec::new();

        for port in ports {
            let addr = format!("0.0.0.0:{}", port);
            let pool_clone = self.pool.clone();
            
            let handle = tokio::spawn(async move {
                match TcpListener::bind(&addr).await {
                    Ok(listener) => {
                        info!("Login Manager listening on TCP {} - Ready!", addr);
                        loop {
                            match listener.accept().await {
                                Ok((stream, addr)) => {
                                    info!("Accepted TCP connection from: {}", addr);
                                    let socket = ClientSocket::new(stream);
                                    let actor = LoginSessionActor::new(socket, pool_clone.clone());
                                    
                                    tokio::spawn(async move {
                                        if let Err(e) = actor.run().await {
                                            error!("Client Session Error for {}: {}", addr, e);
                                        } else {
                                            info!("Session closed for {}", addr);
                                        }
                                    });
                                }
                                Err(e) => error!("TCP Accept Error on {}: {}", addr, e),
                            }
                        }
                    }
                    Err(e) => error!("Failed to bind TCP on {}: {}", addr, e),
                }
            });
            futures.push(handle);
        }

        // Keep the manager alive
        for f in futures {
            let _ = f.await;
        }
        Ok(())
    }
}
