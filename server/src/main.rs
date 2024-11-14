mod cassandra;
mod connection;

use cassandra::Cassandra;
use common::{MessagePacket, Packet, ServerboundHandshake};
use connection::Connection;
use openssl::hash::MessageDigest;
use openssl::sign::{Signer, Verifier};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::accept_async;

struct Server {
    map: RwLock<HashMap<SocketAddr, Arc<Connection>>>,
    connection: TcpListener,
    dal: Arc<cassandra::Cassandra>,
    inbound_msg_send: mpsc::Sender<(SocketAddr, tokio_tungstenite::tungstenite::Message)>,
    inbound_msg_recv: Mutex<mpsc::Receiver<(SocketAddr, tokio_tungstenite::tungstenite::Message)>>,
}

impl Server {
    pub async fn accept_loop(self: Arc<Server>) {
        loop {
            let (stream, address) = self.connection.accept().await.unwrap();

            let socket = accept_async(stream).await.unwrap();
            let connection = Arc::new(Connection::new(
                socket,
                address,
                self.inbound_msg_send.clone(),
            ));

            self.map.write().await.insert(address, connection.clone());

            let cloned_self = self.clone();
            tokio::spawn(async move {
                let history = cloned_self.dal.read_messages().await.unwrap();

                for item in history {
                    connection
                        .queue_packet(MessagePacket {
                            content: item.content,
                            signature: item.signature,
                        })
                        .await;
                }
            });
        }
    }

    pub async fn run(self: &Arc<Server>) {
        let mut inbound_msg_recv = self.inbound_msg_recv.lock().await;
        while let Some((address, message)) = inbound_msg_recv.recv().await {
            self.packet_received(address, message).await;
        }
    }

    pub async fn packet_received(
        self: &Arc<Server>,
        client_address: SocketAddr,
        message: tokio_tungstenite::tungstenite::Message,
    ) {
        let read_map = self.map.read().await;
        let sender = match read_map.get(&client_address) {
            Some(client) => client,
            None => {
                eprintln!(
                    "got message from client {} that isn't in client list",
                    client_address
                );
                return;
            }
        };

        let raw_text = match message {
            tokio_tungstenite::tungstenite::Message::Text(text) => text,
            other => {
                eprintln!("unacceptable client message: {}", other);
                return;
            }
        };

        let (id, json_data) = common::network_decode(&raw_text).unwrap();

        macro_rules! parse_packets {
            ($($packet_type:ident => $func:ident),* $(,)?) => {
                match id {
                $(
                    $packet_type::ID => self.$func(&sender, serde_json::from_str(json_data).unwrap()).await,
                )*
                    other => eprintln!("unexpected packet ID {}", other),
                }
            }
        }

        parse_packets!(
            MessagePacket => handle_message,
            ServerboundHandshake => handle_serverbound_handshake,
        );
    }

    async fn handle_message(&self, conn: &Arc<Connection>, message: MessagePacket) {
        if !conn.has_public_key().await {
            eprintln!("tried to send a message without sending its public key");
            return;
        }

        if !conn
            .verify_signature(message.content.as_bytes(), &message.signature)
            .await
        {
            eprintln!("message signature mismatch");
            return;
        }

        let db_msg = cassandra::Message {
            content: message.content.clone(),
            signature: message.signature.clone(),
        };

        self.dal.insert_message(&db_msg).await.unwrap();

        for client in self.map.read().await.values() {
            client.queue_packet(message.clone()).await;
        }
    }

    async fn handle_serverbound_handshake(
        &self,
        sender: &Arc<Connection>,
        handshake: ServerboundHandshake,
    ) {
        if let Err(e) = sender.set_public_key(handshake.public_key.as_bytes()).await {
            eprintln!("invalid public key: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(_) = dotenvy::dotenv() {
        eprintln!(".env was not loaded");
    }

    let server_address = std::env::var("ADDRESS").expect("ADDRESS not set");
    let cassandra_address = std::env::var("CASSANDRA").expect("CASSANDRA not set");

    let (inbound_msg_send, inbound_msg_recv) = mpsc::channel(64);

    let server = Arc::new(Server {
        map: RwLock::new(HashMap::new()),
        connection: TcpListener::bind(server_address).await.unwrap(),
        dal: Arc::new(
            Cassandra::new(cassandra_address)
                .await
                .expect("can't connect to cassandra"),
        ),
        inbound_msg_send,
        inbound_msg_recv: Mutex::new(inbound_msg_recv),
    });

    tokio::spawn(server.clone().accept_loop());
    println!("Starting...");
    server.run().await;
}
