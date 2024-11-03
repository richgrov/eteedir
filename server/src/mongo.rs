use bson::doc;
use bson::oid::ObjectId;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub content: String,
}

pub struct Query {
    pub operation: String,
    pub value: String,
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

    pub async fn get_messages(&self, query: Query) -> Result<Vec<Message>, mongodb::error::Error> {
        let document = doc! { query.operation: query.value };
        let cursor = self.messages.find(document, None).await?;
        cursor.try_collect().await
    }

    pub async fn insert_message(
        &self,
        message: &Message,
    ) -> Result<ObjectId, mongodb::error::Error> {
        let doc = self.messages.insert_one(message, None).await?;
        Ok(doc.inserted_id.as_object_id().unwrap())
    }

    pub async fn edit_message(
        &self,
        message_id: ObjectId,
        new_content: String,
    ) -> Result<(), mongodb::error::Error> {
        self.messages
            .update_one(
                doc! { "_id": message_id },
                doc! { "$set": { "content": new_content } },
                None,
            )
            .await?;

        Ok(())
    }

    pub async fn delete_message(&self, message_id: ObjectId) -> Result<(), mongodb::error::Error> {
        self.messages
            .delete_one(doc! { "_id": message_id }, None)
            .await?;
        Ok(())
    }
}
