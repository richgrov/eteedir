use futures::TryStreamExt;
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock;
use tokio;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
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
            let (stream, address) = self.connection.accept().await.unwrap();

            let connection = Arc::new(RwLock::new(Connection { stream: stream }));

            self.vector.push(connection.clone());

            tokio::spawn(async move {
                let mut socket = accept_async(connection.get_mut().unwrap().stream)
                    .await
                    .unwrap();

                let history = read_database().await.unwrap();

                for item in history {
                    println!("{}", item);
                    socket.write(tungstenite::Message::Text(item)).unwrap();
                }

                loop {
                    let message = socket.read().unwrap();

                    if message.is_text() {
                        let string_message = message.to_text().unwrap().to_string();

                        insert_message(string_message.to_string()).await.unwrap();
                        socket
                            .send(tungstenite::Message::Text(string_message))
                            .unwrap();
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

    /*
    let input = std::env::args().skip(1).collect::<Vec<String>>().join(" ");
    let client: Client = Client::with_uri_str("mongodb://localhost").await?;
    let messages: Collection<Message> = client.database("eteedir").collection("messages");

    if input.is_empty() {
        let mut cursor = messages.find(None, None).await?;
        while let Some(doc) = cursor.try_next().await? {
            println!("{}", doc.content);
        }
    } else {
        println!("{}", input);

        messages
            .insert_one(Message { content: input }, None)
            .await?;
    };
    */
    Ok(())
}

async fn read_database() -> mongodb::error::Result<Vec<String>> {
    let client = Client::with_uri_str("mongodb://mongo:27017").await?;
    let database = client.database("eteedir");

    let messages: Collection<Message> = database.collection("messages");
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
