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
    let client: Client = Client::with_uri_str("mongodb://localhost").await?;
    let messages: Collection<Message> = client.database("eteedir").collection("messages");

    if input.is_empty() {
        println!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA OH NO ERROR ERROR ERROR YOU HAVE ERROR.")
    } else {
        println!("{}", input);

        messages
            .insert_one(Message { content: input }, None)
            .await?;
    };

    Ok(())
}
