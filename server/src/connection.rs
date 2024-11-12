use std::net::SocketAddr;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;

use crate::cassandra;

type Socket = WebSocketStream<TcpStream>;

pub struct Connection {
    outbound_msg_send: mpsc::Sender<cassandra::Message>,
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
        }
    }

    pub async fn queue_message(&self, message: cassandra::Message) {
        let _ = self.outbound_msg_send.send(message).await;
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
        mut outbound_messages: mpsc::Receiver<cassandra::Message>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        while let Some(msg) = outbound_messages.recv().await {
            let ws_message = tokio_tungstenite::tungstenite::Message::Text(msg.content);
            write.send(ws_message).await?;
        }

        Ok(())
    }
}
