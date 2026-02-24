use tokio::sync::mpsc;
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;
use bytes::Bytes;

#[derive(Debug)]
pub enum OutboundPacket {
    Raw(Vec<u8>),
}

#[derive(Debug)]
pub enum InboundPacket {
    Raw(Vec<u8>),
}

pub struct ClientSocket {
    addr: SocketAddr,
    socket: Arc<UdpSocket>,
    rx_raw_from_dispatcher: mpsc::Receiver<Bytes>,
    rx_outbound_from_logic: mpsc::Receiver<OutboundPacket>,
    tx_inbound_to_logic: mpsc::Sender<InboundPacket>,
}

impl ClientSocket {
    pub fn new(
        addr: SocketAddr,
        socket: Arc<UdpSocket>,
        rx_raw_from_dispatcher: mpsc::Receiver<Bytes>,
        rx_outbound_from_logic: mpsc::Receiver<OutboundPacket>,
        tx_inbound_to_logic: mpsc::Sender<InboundPacket>,
    ) -> Self {
        Self {
            addr,
            socket,
            rx_raw_from_dispatcher,
            rx_outbound_from_logic,
            tx_inbound_to_logic,
        }
    }

    pub async fn run(mut self) {
        log::info!("Zone ClientSocket (Raw) started for {}", self.addr);
        
        loop {
            tokio::select! {
                Some(data) = self.rx_raw_from_dispatcher.recv() => {
                    let _ = self.tx_inbound_to_logic.send(InboundPacket::Raw(data.to_vec())).await;
                }
                Some(out_pkt) = self.rx_outbound_from_logic.recv() => {
                    match out_pkt {
                        OutboundPacket::Raw(data) => {
                            let _ = self.socket.send_to(&data, self.addr).await;
                        }
                    }
                }
            }
        }
    }
}
