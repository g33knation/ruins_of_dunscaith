
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use std::net::SocketAddr;
use anyhow::{Result, Context};
use tracing::{info, error, warn};
use std::sync::Arc;

pub struct ClientSocketSettings {
    pub port: u16,
}

/// The main IO loop for the World Server.
/// 
/// * `incoming_tx`: Channel to send received packets TO the System Actor.
/// * `outgoing_rx`: Channel to receive packets FROM actors to send to current clients.
pub async fn run_socket_loop(
    settings: ClientSocketSettings,
    incoming_tx: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    mut outgoing_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
) -> Result<()> {
    
    let public_ip = std::env::var("PUBLIC_IP").unwrap_or("127.0.0.1".to_string());
    let addr = format!("{}:{}", "0.0.0.0", settings.port);
    let socket = UdpSocket::bind(&addr).await
        .context("Failed to bind UDP socket")?;
    
    let socket = Arc::new(socket);
    info!("World Server UDP Listening on {}", addr);

    // Split for concurrency (RX loop and TX loop)
    let rx_socket = socket.clone();
    let tx_socket = socket;

    // 1. RX Loop
    let rx_handle = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match rx_socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    let data = buf[..len].to_vec();
                    info!("RX Packet from {}: {} bytes", src, len);
                    // Forward to System Actor
                    if let Err(e) = incoming_tx.send((src, data)).await {
                        error!("Failed to forward packet to system actor: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("UDP Receive Error: {}", e);
                }
            }
        }
    });

    // 2. TX Loop
    let tx_handle = tokio::spawn(async move {
        while let Some((dest, data)) = outgoing_rx.recv().await {
            if let Err(e) = tx_socket.send_to(&data, dest).await {
                warn!("Failed to send UDP packet to {}: {}", dest, e);
            }
        }
    });

    // Wait for either to fail/finish (should be never)
    let _ = tokio::try_join!(rx_handle, tx_handle);

    Ok(())
}
