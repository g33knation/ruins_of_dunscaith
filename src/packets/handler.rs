use crate::packets::opcodes::OpCode;
use crate::packets::error::PacketError;
use crate::packets::login::LoginPacket;
use crate::packets::char_select;
use binrw::BinRead;
use std::io::Cursor;
use std::sync::Arc;
use tokio::net::UdpSocket;
use bytes::BytesMut;

pub async fn handle_packet(
    socket: &Arc<UdpSocket>,
    opcode_u16: u16,
    mut payload: BytesMut,
    addr: std::net::SocketAddr,
    pool: &Arc<sqlx::MySqlPool>,
) -> Result<(), PacketError> {
    let opcode = OpCode::from_u16_safe(opcode_u16)
        .ok_or(PacketError::UnknownOpCode(opcode_u16))?;

    log::debug!("Handling OpCode: {:?}", opcode);

    match opcode {
        OpCode::LoginRequest => {
            // We can use BytesMut's reader or AsRef<[u8]>
            // Since BinRw takes a reader, we can use Cursor on the bytes
            let mut reader = Cursor::new(&payload[..]);
            let pkt = LoginPacket::read(&mut reader)?;
            
            // Example of offloading heavy work (crypto) to blocking thread
            let socket_clone = socket.clone();
            let pool_clone = pool.clone();
            
            tokio::task::spawn_blocking(move || {
                // Simulate password verification or DB lookup
                log::info!("Processing LoginRequest for: {}", pkt.get_name());
                // In real impl, we would check DB and send response using socket_clone
            }).await.map_err(|e| PacketError::HandlerError(e.to_string()))?;
            
            Ok(())
        },
        OpCode::SendCharInfo => {
            // Note: This OpCode (0x2409) is usually the Server Response, 
            // but we map it here for demonstration or if client sends it (unlikely).
            // Usually Client sends OP_CharProfile (or similar).
            // We'll assume for now this handler is triggered by the relevant request.
            
            // Handle Char Select (needs Account ID, currently mocking 0)
            char_select::handle(socket.clone(), addr, pool.clone(), 0).await;
            Ok(())
        },
        _ => {
            log::warn!("Unhandled OpCode: {:?}", opcode);
            Ok(())
        }
    }
}
