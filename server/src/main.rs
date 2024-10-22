use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let input = std::env::args().skip(1).collect::<Vec<String>>().join(" ");

    if input.is_empty() {
        println!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA OH NO ERROR ERROR ERROR YOU HAVE ERROR.")
    } else {
        println!("{}", input)
    };

    let client = Client::with_uri_str("mongodb://localhost").await?;

    let test: Collection<Message> = client.database("eteedir").collection("messages");

    Ok(())
}
