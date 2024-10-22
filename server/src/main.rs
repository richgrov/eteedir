use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost").await?;

    let test: Collection<Message> = client.database("eteedir").collection("messages");

    Ok(())
}
