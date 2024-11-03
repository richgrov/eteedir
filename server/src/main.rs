mod mongo;

use futures_util::{SinkExt, StreamExt};
use mongo::Mongo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

struct Server {
    clients: RwLock<HashMap<SocketAddr, Arc<RwLock<Connection>>>>,
    connection: TcpListener,
    mongo: Arc<mongo::Mongo>,
}

struct Connection {
    socket: WebSocketStream<TcpStream>,
}

impl Server {
    async fn handle_connection(
        self: Arc<Server>,
        tx: Sender<String>,
        mut rx: Receiver<String>,
        stream: TcpStream,
        address: SocketAddr,
    ) {
        let socket = accept_async(stream).await.unwrap();
        let connection = Arc::new(RwLock::new(Connection { socket }));

        self.clients
            .write()
            .await
            .insert(address, connection.clone());

        let history = self.mongo.all_messages().await.unwrap();
        for message in history {
            connection
                .write()
                .await
                .socket
                .send(Message::Text(message.content.to_string()))
                .await
                .unwrap();
        }

        tokio::spawn(read_incoming_messages(
            self.mongo.clone(),
            connection.clone(),
            tx,
        ));

        loop {
            tokio::select! {
                msg_recv = rx.recv() => {
                    connection.write().await.socket.send(Message::Text(msg_recv.unwrap())).await.unwrap();
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let server_address = std::env::var("ADDRESS").unwrap_or("0.0.0.0:8080".to_string());
    let mongo_address = std::env::var("MONGODB").unwrap_or("mongodb://localhost:27017".to_string());

    let server = Arc::new(Server {
        clients: RwLock::new(HashMap::new()),
        connection: TcpListener::bind(server_address).await.unwrap(),
        mongo: Arc::new(
            Mongo::new(mongo_address, "eteedir")
                .await
                .expect("can't connect to mongo"),
        ),
    });

    loop {
        let server = server.clone();
        let (stream, address) = server.connection.accept().await.unwrap();

        println!("Connection: {}", address);

        let (sender, channel) = mpsc::channel(16);
        tokio::spawn(server.handle_connection(sender.clone(), channel, stream, address));
    }
}

async fn read_incoming_messages(
    mongo: Arc<mongo::Mongo>,
    connection: Arc<RwLock<Connection>>,
    sender: Sender<String>,
) -> Result<(), Error> {
    while let Some(maybe_message) = connection.write().await.socket.next().await {
        let message = match maybe_message {
            Ok(m) => m,
            Err(_) => {
                break;
            }
        };

        if message.is_text() {
            mongo
                .insert_message(&mongo::Message {
                    content: message.to_string(),
                })
                .await
                .unwrap();
            sender.send(message.to_string()).await.unwrap();
        }
    }
    Ok(())
}
