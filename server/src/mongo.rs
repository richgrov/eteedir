use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub content: String,
}

pub struct Mongo {
    connection: mongodb::Client,
    messages: mongodb::Collection<Message>,
}

impl Mongo {
    pub async fn new(
        address: impl AsRef<str>,
        database_name: &str,
    ) -> Result<Mongo, mongodb::error::Error> {
        let connection = mongodb::Client::with_uri_str(address).await?;
        let database = connection.database(database_name);

        Ok(Mongo {
            connection,
            messages: database.collection("messages"),
        })
    }

    pub async fn all_messages(&self) -> Result<Vec<Message>, mongodb::error::Error> {
        let cursor = self.messages.find(None, None).await?;
        cursor.try_collect().await
    }

    pub async fn insert_message(&self, message: &Message) -> Result<(), mongodb::error::Error> {
        self.messages.insert_one(message, None).await?;
        Ok(())
    }
}
