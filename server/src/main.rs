use futures::TryStreamExt;
use futures_util::{SinkExt, StreamExt};
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    content: String,
}

struct Server {
    map: RwLock<HashMap<SocketAddr, Arc<RwLock<Connection>>>>,
    connection: TcpListener,
}

struct Connection {
    stream: TcpStream,
}

impl Server {
    pub async fn server_stream(self: Arc<Server>) {
        loop {
            let (stream, address) = self.connection.accept().await.unwrap();
            let connection = Arc::new(RwLock::new(Connection { stream }));

            self.map.write().await.insert(address, connection.clone());

            let cloned_self = self.clone();

            tokio::spawn(async move {
                let mut writable_connection = connection.write().await;
                let mut socket = accept_async(&mut writable_connection.stream).await.unwrap();

                let history = read_database().await.unwrap();

                for item in history {
                    println!("{}", item);
                    socket.send(Message::Text(item)).await.unwrap();
                }

                loop {
                    let probably_message = socket.next().await.unwrap();

                    let message = match probably_message {
                        Ok(m) => m,
                        Err(_) => {
                            cloned_self.map.write().await.remove(&address);
                            break;
                        }
                    };

                    if message.is_text() {
                        let string_message = message.to_text().unwrap().to_string();

                        insert_message(string_message.to_string()).await.unwrap();
                        socket.send(Message::Text(string_message)).await.unwrap();

                        for each in cloned_self.map.read().await.values() {
                            each.write()
                                .awaitString
                                .stream
                                .send(Message::Text("asdfg".to_string()));
                        }
                    }
                }

                Result::<(), tokio_tungstenite::tungstenite::Error>::Ok(())
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let address = std::env::var("ADDRESS").unwrap_or("0.0.0.0:8080".to_string());
    let server = Arc::new(Server {
        map: RwLock::new(HashMap::new()),
        connection: TcpListener::bind(address).await.unwrap(),
    });

    println!("say something just so i know it works");

    server.server_stream().await;
}

async fn read_database() -> mongodb::error::Result<Vec<String>> {
    let address = std::env::var("MONGODB").unwrap_or("mongodb://mongo:27017".to_string());
    let client = Client::with_uri_str(address).await?;
    let database = client.database("eteedir");

    let messages: Collection<Content> = database.collection("messages");
    let mut cursor = messages.find(None, None).await?;
    let mut vector = Vec::new();

    while let Some(document) = cursor.try_next().await? {
        vector.push(document.content);
    }

    Ok(vector)
}

async fn insert_message(message: String) -> mongodb::error::Result<()> {
    let address = std::env::var("MONGODB").unwrap_or("mongodb://mongo:27017".to_string());
    let client = Client::with_uri_str(address).await?;
    let database = client.database("eteedir");
    let messages = database.collection("messages");

    messages
        .insert_one(doc! { "content": message }, None)
        .await?;

    Ok(())
}
