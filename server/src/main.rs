mod mongo;

use futures_util::{SinkExt, StreamExt};
use mongo::Mongo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

struct Server {
    map: RwLock<HashMap<SocketAddr, Arc<RwLock<Connection>>>>,
    connection: TcpListener,
    mongo: Arc<mongo::Mongo>,
}

struct Connection {
    socket: WebSocketStream<TcpStream>,
}

impl Server {
    pub async fn server_stream(self: Arc<Server>) {
        loop {
            let (stream, address) = self.connection.accept().await.unwrap();

            let cloned_self = self.clone();

            tokio::spawn(async move {
                let socket = accept_async(stream).await.unwrap();
                let connection = Arc::new(RwLock::new(Connection { socket }));

                cloned_self
                    .map
                    .write()
                    .await
                    .insert(address, connection.clone());

                let history = cloned_self.mongo.all_messages().await.unwrap();
                for item in history {
                    connection
                        .write()
                        .await
                        .socket
                        .send(Message::Text(item.content))
                        .await
                        .unwrap();
                }

                loop {
                    let probably_message = connection.write().await.socket.next().await.unwrap();

                    let message = match probably_message {
                        Ok(m) => m,
                        Err(_) => {
                            cloned_self.map.write().await.remove(&address);
                            break;
                        }
                    };

                    if let Message::Text(text) = &message {
                        let eteedir_msg = mongo::Message {
                            content: text.to_owned(),
                        };

                        let id = cloned_self
                            .mongo
                            .insert_message(&eteedir_msg)
                            .await
                            .unwrap();

                        cloned_self.mongo.delete_message(id).await.unwrap();
                    }

                    for each in cloned_self.map.read().await.values() {
                        each.write()
                            .await
                            .socket
                            .send(message.clone())
                            .await
                            .unwrap();
                    }
                }

                Result::<(), tokio_tungstenite::tungstenite::Error>::Ok(())
            });
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(_) = dotenvy::dotenv() {
        eprintln!(".env was not loaded");
    }

    let server_address = std::env::var("ADDRESS").expect("ADDRESS not set");
    let mongo_address = std::env::var("MONGODB").expect("MONGODB not set");

    let server = Arc::new(Server {
        map: RwLock::new(HashMap::new()),
        connection: TcpListener::bind(server_address).await.unwrap(),
        mongo: Arc::new(
            Mongo::new(format!("mongodb://{}/", mongo_address), "eteedir")
                .await
                .expect("can't connect to mongo"),
        ),
    });

    println!("say something just so i know it works");

    server.server_stream().await;
}
