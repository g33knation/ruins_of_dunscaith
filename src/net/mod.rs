pub mod packet;
pub mod login_packets;
pub mod db;
pub mod client;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use crate::net::packet::{PacketHeader, LoginOpCode};
use crate::net::login_packets::LoginRequest;
use crate::net::db::DbWorker;
use crate::net::client::{ClientSystem, ClientEvent};
use binrw::BinRead;
use std::io::Cursor;
use uuid::Uuid;
use std::sync::Arc;
use sqlx::PgPool;

pub async fn start_server(pool: Arc<PgPool>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:5998").await?;
    log::info!("LoginServer listening on port 5998");

    // Spawn DB Worker
    let (db_tx, db_rx) = mpsc::channel(100);
    let db_worker = DbWorker {
        pool: pool.clone(),
        rx: db_rx,
    };
    tokio::spawn(db_worker.run());

    loop {
        let (socket, addr) = listener.accept().await?;
        log::info!("New connection from: {}", addr);
        let id = Uuid::new_v4();
        let db_tx = db_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, id, db_tx).await {
                log::error!("Client {} Error: {}", id, e);
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream, id: Uuid, db_tx: mpsc::Sender<crate::net::db::DbRequest>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut reader, mut writer) = stream.split();
    let (app_tx, mut app_rx) = mpsc::channel(32);

    // Spawn Client System Actor
    let mut system = ClientSystem::new(id, db_tx, app_tx);
    tokio::spawn(async move {
        // This loop handles signals from socket or other internal events?
        // Actually system is reactive. We'll pump events to it?
        // Wait, ClientSystem::handle_event is async.
        // We need a channel to send events TO the system if we want it strictly separate.
        // Or we can just call system methods if we keep system local to a Logic task.
        
        // Let's create an event channel for the Logic Task
        let (logic_tx, mut logic_rx) = mpsc::channel(32);
        
        // Spawn Logic Task
        tokio::spawn(async move {
            while let Some(event) = logic_rx.recv().await {
                system.handle_event(event).await;
            }
        });
        
        // Return logic_tx to be used by Reader
        logic_tx
    }); // Wait, this logic pump structure is getting complex for `handle_client`.

    // Simpler: Run System Loop in a separate task, feed it from Reader
    let (logic_tx, mut logic_rx) = mpsc::channel(32);
    
    // Writer Task
    let mut writer_handle = tokio::spawn(async move {
        while let Some(data) = app_rx.recv().await {
            if let Err(_) = writer.write_all(&data).await {
                break;
            }
        }
    });

    // Logic Task
    let logic_handle = tokio::spawn(async move {
        let mut system = ClientSystem::new(id, db_tx, app_tx); // system owns app_tx
        while let Some(event) = logic_rx.recv().await {
            system.handle_event(event).await;
        }
    });

    // Reader Loop (Main thread of this spawn)
    let mut header_buf = [0u8; 4];
    loop {
        if let Err(_) = reader.read_exact(&mut header_buf).await {
            break; 
        }
        
        let mut cursor = Cursor::new(&header_buf);
        let header = match PacketHeader::read(&mut cursor) {
            Ok(h) => h,
            Err(_) => break,
        };
        
        let mut payload = vec![0u8; header.size as usize];
        if let Err(_) = reader.read_exact(&mut payload).await {
            break;
        }

        let opcode = LoginOpCode::from(header.opcode);
        let mut payload_cursor = Cursor::new(&payload);

        match opcode {
            LoginOpCode::Login => {
                if let Ok(req) = LoginRequest::read(&mut payload_cursor) {
                    let _ = logic_tx.send(ClientEvent::Login(req)).await;
                }
            },
            LoginOpCode::ServerListRequest => {
                let _ = logic_tx.send(ClientEvent::ServerList).await;
            },
            LoginOpCode::PlayEverquestRequest => {
                if let Ok(req) = crate::net::login_packets::PlayRequest::read(&mut payload_cursor) {
                    let _ = logic_tx.send(ClientEvent::PlayRequest(req)).await;
                }
            },
            _ => {}
        }
    }
    
    let _ = logic_tx.send(ClientEvent::Disconnect).await;
    // Cleanup
    writer_handle.abort();
    logic_handle.abort(); // Or let it finish disconnect

    Ok(())
}
