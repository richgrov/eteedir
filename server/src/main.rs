use futures::TryStreamExt;
use futures_util::SinkExt;
use futures_util::StreamExt;
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    content: String,
}

struct Server {
    vector: Vec<Arc<RwLock<Connection>>>,
    connection: TcpListener,
}

struct Connection {
    stream: TcpStream,
}

impl Server {
    pub async fn server_stream(&mut self) {
        loop {
            let (stream, _) = self.connection.accept().await.unwrap();
            let connection = Arc::new(RwLock::new(Connection { stream }));

            self.vector.push(connection.clone());

            tokio::spawn(async move {
                let mut rustisannoying = connection.write().await;
                let mut socket = accept_async(&mut rustisannoying.stream).await.unwrap();

                let history = read_database().await.unwrap();

                for item in history {
                    println!("{}", item);
                    socket.send(Message::Text(item)).await.unwrap();
                }

                loop {
                    let message = socket.next().await.unwrap().unwrap();

                    if message.is_text() {
                        let string_message = message.to_text().unwrap().to_string();

                        insert_message(string_message.to_string()).await.unwrap();
                        socket.send(Message::Text(string_message)).await.unwrap();
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let address = "0.0.0.0:8080";
    let mut server = Server {
        vector: Vec::new(),
        connection: TcpListener::bind(address).await.unwrap(),
    };

    server.server_stream().await;
}

async fn read_database() -> mongodb::error::Result<Vec<String>> {
    let client = Client::with_uri_str("mongodb://mongo:27017").await?;
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
    let client = Client::with_uri_str("mongodb://mongo:27017").await?;
    let database = client.database("eteedir");
    let messages = database.collection("messages");

    messages
        .insert_one(doc! { "content": message }, None)
        .await?;

    Ok(())
}
