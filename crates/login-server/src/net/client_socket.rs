use anyhow::Result;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use shared::opcodes::OpCode;
use num_traits::FromPrimitive;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct ClientSocket {
    pub stream: TcpStream,
}

impl ClientSocket {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn read_packet(&mut self) -> Result<(OpCode, Vec<u8>)> {
        // Size is Big Endian (standard network order)
        let size = self.stream.read_u16().await?;
        
        // OpCode is Little Endian (EQ protocol quirk - matches our write path)
        let opcode_raw = self.stream.read_u16_le().await?;
        let opcode = OpCode::from_u16(opcode_raw).unwrap_or(OpCode::Unknown);
        
        // Safety check: Don't allocate massive buffers if size is crazy
        // RoF2 opcode is included in size, so body is size - 2
        let body_len = if size >= 2 { size - 2 } else { 0 };
        
        let mut body = vec![0u8; body_len as usize];
        if body_len > 0 {
            self.stream.read_exact(&mut body).await?;
        }

        Ok((opcode, body))
    }

    pub async fn send_packet<T: binrw::BinWrite>(&mut self, opcode: OpCode, packet: &T) -> Result<()> 
    where for<'a> T::Args<'a>: Default 
    {
        let mut body = Vec::new();
        let mut writer = Cursor::new(&mut body);
        packet.write_options(&mut writer, binrw::Endian::Little, <T as binrw::BinWrite>::Args::default())?;
        self.send_raw(opcode, body).await
    }

    pub async fn send_raw(&mut self, opcode: OpCode, body_bytes: Vec<u8>) -> Result<()> {
        // [CRITICAL FIX]
        // Size must equal Body Length + 2 bytes (for the OpCode)
        let packet_size = (body_bytes.len() + 2) as u16;

        let mut frame = Vec::with_capacity(4 + body_bytes.len());
        WriteBytesExt::write_u16::<BigEndian>(&mut frame, packet_size)?;
        WriteBytesExt::write_u16::<LittleEndian>(&mut frame, opcode as u16)?;   // OpCode = LE (Crucial Fix)
        frame.extend_from_slice(&body_bytes);

        self.stream.write_all(&frame).await?;
        self.stream.flush().await?;
        
        Ok(())
    }
}
