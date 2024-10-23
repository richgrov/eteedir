use futures::TryStreamExt;
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::thread::spawn;
use tokio;
use tungstenite::accept;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();

    for stream in server.incoming() {
        spawn(move || {
            let mut socket = accept(stream.unwrap()).unwrap();

            loop {
                let message = socket.read().unwrap();

                if message.is_text() {
                    let string_message = message.to_text().unwrap();

                    print!("{}", string_message);
                }
            }
        });
    }

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

async fn read_database() -> mongodb::error::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost").await?;
    let database = client.database("eteedir");

    let messages: Collection<Message> = database.collection("messages");

    Ok(())
}
