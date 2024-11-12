mod cassandra;
mod connection;
mod mongo;

use cassandra::Cassandra;
use connection::Connection;
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
                    connection.queue_message(item).await;
                }
            });
        }
    }

    pub async fn run(self: &Arc<Server>) {
        let mut inbound_msg_recv = self.inbound_msg_recv.lock().await;
        while let Some((address, message)) = inbound_msg_recv.recv().await {
            self.message_received(address, message).await;
        }
    }

    pub async fn message_received(
        self: &Arc<Server>,
        _client_address: SocketAddr,
        message: tokio_tungstenite::tungstenite::Message,
    ) {
        let eteedir_msg = cassandra::Message {
            content: message.into_text().unwrap(),
        };

        self.dal.insert_message(&eteedir_msg).await.unwrap();

        for client in self.map.read().await.values() {
            client.queue_message(eteedir_msg.clone()).await;
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
