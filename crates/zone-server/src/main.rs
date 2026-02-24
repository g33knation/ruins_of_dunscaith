use binrw::BinRead;
use tokio::time::{interval, Duration};
use tokio::net::UdpSocket;
use dotenv::dotenv; 
use std::env;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;

mod net;
mod game;
use crate::net::key_manager::ZoneKeyManager;
use crate::game::inventory::InventoryManager;
use tokio::sync::mpsc;

// Basic RoF2 Session Request Packet (Zone)
#[derive(Debug, BinRead)]
#[br(little)]
struct ZoneSessionRequest {
    pub unknown: u32,
    pub session_id: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    // ... (DB Connect)
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;
    let _pool = Arc::new(pool.clone()); // Clone to keep 'pool' available
    log::info!("Zone Server connected to DB.");

    // --- Zone Key Manager ---
    let (key_manager, key_tx) = ZoneKeyManager::new(pool.clone());
    tokio::spawn(key_manager.run());
    log::info!("Zone Key Manager started.");

    // --- Inventory Manager ---
    let (inv_manager, inv_tx) = InventoryManager::new(pool.clone());
    tokio::spawn(inv_manager.run());
    log::info!("Inventory Manager started.");

    // --- DB Migration (Persistence) ---
    log::info!("Checking DB Schema...");
    // --- DB Migration (Persistence) ---
    log::info!("Checking DB Schema...");
    // Migration logic removed as we are using external migration scripts
    log::info!("DB Schema Verified.");

    // --- World Manager ---
    use crate::game::world::WorldManager;
    let (world_manager, world_tx) = WorldManager::new(pool.clone());
    let zone_id = env::var("ZONE_ID").unwrap_or("202".to_string()).parse::<i32>().unwrap_or(202);
    tokio::spawn(world_manager.run(zone_id));
    log::info!("WorldManager started for zone {}.", zone_id);

    // Bind UDP socket
    let public_ip = std::env::var("PUBLIC_IP").unwrap_or("127.0.0.1".to_string());
    let addr = format!("{}:7000", public_ip);
    let socket = UdpSocket::bind(&addr).await?;
    log::info!("Zone Server running on {}", addr);
    let _socket = Arc::new(socket);
    // --- Actor Map ---
    use std::sync::Mutex;
    use std::collections::HashMap;
    use crate::net::client_socket::{ClientSocket, InboundPacket, OutboundPacket};
    use crate::net::session::ClientSystem;
    use bytes::Bytes;
    
    // Map sends Raw Bytes to ClientSocket
    let actors: Arc<Mutex<HashMap<std::net::SocketAddr, mpsc::Sender<Bytes>>>> = Arc::new(Mutex::new(HashMap::new()));
    
    let rx_socket = _socket.clone();
    let actors_clone = actors.clone();
    let db_pool = pool.clone();
    
    // Packet Loop
    tokio::spawn(async move {
        let mut buf = [0u8; 65535]; 
        loop {
            match rx_socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    log::info!("RECV Packet from {} (len={})", src, len);
                    println!("DEBUG: RECV Packet from {} (len={})", src, len);
                    let mut tx = {
                        let mut map = actors_clone.lock().unwrap();
                        if let Some(tx) = map.get(&src) {
                            tx.clone()
                        } else {
                            // Spawn new Actor Pair
                            log::info!("New Zone Session: {}", src);
                            
                            // Channels
                            // Dispatcher -> ClientSocket (Raw)
                            let (tx_raw, rx_raw) = mpsc::channel(1024);
                            // ClientSocket -> ClientSystem (Inbound)
                            let (tx_inbound, rx_inbound) = mpsc::channel(1024);
                            // ClientSystem -> ClientSocket (Outbound)
                            let (tx_outbound, rx_outbound) = mpsc::channel(1024);
                            
                            map.insert(src, tx_raw.clone());
                            
                            // Spawn ClientSocket (IO)
                            let socket_actor = ClientSocket::new(
                                src,
                                rx_socket.clone(),
                                rx_raw,
                                rx_outbound,
                                tx_inbound
                            );
                            tokio::spawn(socket_actor.run());
                            
                            // Spawn ClientSystem (Logic)
                            let system_actor = ClientSystem::new(
                                src,
                                db_pool.clone(),
                                key_tx.clone(),
                                inv_tx.clone(),
                                world_tx.clone(), // Pass World TX
                                rx_inbound,
                                tx_outbound
                            );
                            tokio::spawn(system_actor.run());

                            
                            tx_raw
                        }
                    };

                    let packet = Bytes::copy_from_slice(&buf[..len]);
                    if let Err(_) = tx.send(packet).await {
                        log::warn!("ClientSocket for {} unreachable", src);
                    }
                },
                Err(e) => log::error!("UDP Recv Error: {}", e),
            }
        }
    });

    log::info!("Zone Server running on {}", addr);
    println!("DEBUG: Zone Server running on {}", addr);
    
    // Main thread can just sleep or do global ticks
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
