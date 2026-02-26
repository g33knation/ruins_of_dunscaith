mod net;

use crate::net::login_manager::LoginManager;
use anyhow::Result;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // 1. Initialize Logging
    tracing_subscriber::fmt::init();
    info!("Starting Rust Login Server (Dual TCP/UDP)...");
    
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "stub".to_string());
    let pool = if database_url == "stub" {
        info!("Running in STUBBED database mode (Testing only)");
        None
    } else {
        Some(sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url).await?)
    };

    // 2. Start Dual-Protocol Login (RoF2/Akk-Stack Support)
    // 2a. UDP Discovery Handlers (Ports 5998 & 5999)
    for port in [5998, 5999] {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::net::discovery::start_discovery_listener(port, pool_clone).await {
                error!("UDP Discovery Listener (Port {}) Failed: {}", port, e);
            }
        });
    }

    // 2b. TCP Login Manager (Ports 5998 & 5999)
    let mut manager = LoginManager::new(pool).await?;
    info!("Starting TCP Login Manager (Ports 5998, 5999)...");
    manager.run().await?;

    Ok(())
}
