mod mongo;

use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use mongo::Mongo;
use std::sync::Arc;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{
    broadcast,
    broadcast::{Receiver, Sender},
};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

struct Server {
    connection: TcpListener,
    mongo: Arc<mongo::Mongo>,
}

impl Server {
    async fn handle_connection(
        self: Arc<Server>,
        tx: Sender<String>,
        mut rx: Receiver<String>,
        stream: TcpStream,
    ) {
        let socket = accept_async(stream).await.unwrap();
        let (mut write, read) = socket.split();

        let history = self.mongo.all_messages().await.unwrap();
        for message in history {
            write
                .send(Message::Text(message.content.to_string()))
                .await
                .unwrap();
        }

        tokio::spawn(read_incoming_messages(self.mongo.clone(), read, tx));

        loop {
            tokio::select! {
                msg_recv = rx.recv() => {
                    if let msg = msg_recv {
                        write.send(Message::Text(msg.unwrap().to_string())).await.unwrap();
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let server_address = std::env::var("ADDRESS").unwrap_or("0.0.0.0:8080".to_string());
    let mongo_address = std::env::var("MONGODB").unwrap_or("mongodb://mongo:27017".to_string());

    let server = Arc::new(Server {
        connection: TcpListener::bind(server_address).await.unwrap(),
        mongo: Arc::new(
            Mongo::new(mongo_address, "eteedir")
                .await
                .expect("can't connect to mongo"),
        ),
    });

    let (tx, _) = broadcast::channel::<String>(16);

    loop {
        let server = server.clone();
        let (stream, address) = server.connection.accept().await.unwrap();

        println!("Connection: {}", address);

        let rx = tx.clone().subscribe();
        tokio::spawn(server.handle_connection(tx.clone(), rx, stream));
    }
}

async fn read_incoming_messages(
    mongo: Arc<mongo::Mongo>,
    mut read: SplitStream<WebSocketStream<TcpStream>>,
    sender: Sender<String>,
) -> Result<(), Error> {
    while let Some(maybe_message) = read.next().await {
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
            sender.send(message.to_string()).unwrap();
        }
    }
    Ok(())
}
