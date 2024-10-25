use futures::TryStreamExt;
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use tokio;
use tokio::sync::broadcast;
use tungstenite::accept;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
}

struct Server {
    connection: TcpListener,
}

impl Server {
    pub async fn server_stream(&self) {
        for stream in self.connection.incoming() {
            tokio::spawn(async move {
                let mut socket = accept(stream.unwrap()).unwrap();

                let history = read_database().await.unwrap();

                for item in history {
                    println!("{}", item);
                    let _ = socket.send(tungstenite::Message::Text(item));
                }

                loop {
                    let message = socket.read().unwrap();

                    if message.is_text() {
                        let string_message = message.to_text().unwrap().to_string();

                        let _ = insert_message(string_message.to_string()).await;
                        let _ = socket.send(tungstenite::Message::Text(string_message));
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let address = "127.0.0.1:9001";
    let server = Server {
        connection: TcpListener::bind(address).unwrap(),
    };

    let _ = server.server_stream().await;

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
}

async fn read_database() -> mongodb::error::Result<Vec<String>> {
    let client = Client::with_uri_str("mongodb://localhost").await?;
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
    let client = Client::with_uri_str("mongodb://localhost").await?;
    let database = client.database("eteedir");
    let messages = database.collection("messages");

    messages
        .insert_one(doc! { "content": message }, None)
        .await?;

    Ok(())
}
