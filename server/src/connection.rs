use std::borrow::Borrow;
use std::net::SocketAddr;
use std::ops::Deref;

use common::Packet;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Verifier;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::WebSocketStream;

type Socket = WebSocketStream<TcpStream>;

pub struct Connection {
    outbound_msg_send: mpsc::Sender<String>,
    public_key: RwLock<Option<PKey<openssl::pkey::Public>>>,
}

impl Connection {
    pub fn new(
        socket: Socket,
        address: SocketAddr,
        inbound_messages: mpsc::Sender<(SocketAddr, tokio_tungstenite::tungstenite::Message)>,
    ) -> Connection {
        let (write, read) = socket.split();
        let (outbound_send, outbound_recv) = mpsc::channel(16);

        tokio::spawn(Self::read_loop(read, address, inbound_messages));
        tokio::spawn(Self::write_loop(write, outbound_recv));

        Connection {
            outbound_msg_send: outbound_send,
            public_key: RwLock::new(None),
        }
    }

    pub async fn queue_packet<P: Packet>(&self, packet: P) {
        let _ = self.outbound_msg_send.send(packet.network_encode()).await;
    }

    pub async fn read_loop(
        mut read: SplitStream<Socket>,
        address: SocketAddr,
        inbound_messages: mpsc::Sender<(SocketAddr, tokio_tungstenite::tungstenite::Message)>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(m) => {
                    if let Err(_) = inbound_messages.send((address, m)).await {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("client read error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn write_loop(
        mut write: SplitSink<Socket, tokio_tungstenite::tungstenite::Message>,
        mut outbound_messages: mpsc::Receiver<String>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        while let Some(encoded_packet) = outbound_messages.recv().await {
            let ws_message = tokio_tungstenite::tungstenite::Message::Text(encoded_packet);
            write.send(ws_message).await?;
        }

        Ok(())
    }

    pub async fn has_public_key(&self) -> bool {
        return self.public_key.read().await.is_some();
    }

    pub async fn set_public_key(&self, pem: &[u8]) -> Result<(), openssl::error::ErrorStack> {
        let pkey = PKey::public_key_from_pem(pem)?;
        let _ = self.public_key.write().await.insert(pkey);
        Ok(())
    }

    pub async fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        let public_key_read = self.public_key.read().await;
        let public_key = public_key_read.as_ref().unwrap();

        let mut verifier =
            Verifier::new(MessageDigest::sha256(), &public_key).expect("failed to create verifier");

        verifier.update(data).expect("failed to update verifier");

        verifier.verify(signature).is_ok()
    }
}
