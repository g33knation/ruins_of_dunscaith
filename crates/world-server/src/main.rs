
mod actors;
mod net;
mod db;
mod packets;

use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::db::DatabaseManager;
use crate::net::socket::{run_socket_loop, ClientSocketSettings};
use crate::actors::system::ClientSystemActor;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize Logging
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    info!("Starting World Server (Actor Architecture)...");

    // 2. Initialize Database
    let db = match DatabaseManager::new().await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Fatal: Failed to connect to database: {}", e);
            return Err(e);
        }
    };

    // 3. Create Channels
    // Socket -> System (Incoming Packets)
    let (input_tx, input_rx) = mpsc::channel(1024);
    
    // Actors -> Socket (Outgoing Packets)
    let (output_tx, output_rx) = mpsc::channel(1024);

    // 4. Spawn System Actor
    let system_actor = ClientSystemActor::new(input_rx, output_tx, db);
    let system_handle = tokio::spawn(async move {
        system_actor.run().await;
    });

    // 5. Run Socket Loop (Main Thread or Spawned)
    let socket_settings = ClientSocketSettings { port: 9000 };
    match run_socket_loop(socket_settings, input_tx, output_rx).await {
        Ok(_) => info!("Socket loop exited cleanly."),
        Err(e) => error!("Socket loop exited with error: {}", e),
    }
    
    // If socket loop exits, waiting for system...
    let _ = system_handle.await;

    Ok(())
}
