use futures::TryStreamExt;
use rand::Rng;
use scylla::batch::Batch;
use scylla::{FromRow, Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Message {
    pub content: String,
    pub public_key: Vec<u8>,
}

pub struct Cassandra {
    session: Session,
}

impl Cassandra {
    pub async fn new(address: impl AsRef<str>) -> Result<Cassandra, Box<dyn Error>> {
        let uri = address;
        let session = SessionBuilder::new().known_node(uri).build().await?;

        Ok(Cassandra { session })
    }

    // IF
    pub async fn insert_message(&self, message: &Message) -> Result<(), Box<dyn Error>> {
        let id = rand::thread_rng().gen::<i64>();
        self.session
            .query_unpaged(
                "INSERT INTO eteedir.messages (id, message, timestamp) VALUES(?, ?, ToTimeStamp(NOW())) IF NOT EXISTS",
                (id, message.content.clone()),
            )
            .await?;

        Ok(())
    }

    // TTL(Time to Live)
    pub async fn insert_message_ttl(
        &self,
        message: &Message,
        seconds: i32,
    ) -> Result<(), Box<dyn Error>> {
        let id = rand::thread_rng().gen::<i64>();
        self.session
            .query_unpaged(
                "INSERT INTO eteedir.messages (id, message, timestamp) VALUES(?, ?, ToTimeStamp(NOW())) USING TTL ?",
                (id, message.content.clone(), seconds,),
            )
            .await?;

        Ok(())
    }

    pub async fn read_messages(&self) -> Result<Vec<Message>, Box<dyn Error>> {
        let messages = self
            .session
            .query_iter("SELECT message, public_key FROM eteedir.messages", &[])
            .await?
            .into_typed::<Message>();

        let vec: Vec<Message> = messages.try_collect().await?;

        Ok(vec)
    }

    // LIMIT
    pub async fn read_n_messages(&self, limit: i32) -> Result<Vec<Message>, Box<dyn Error>> {
        let messages = self
            .session
            .query_iter("SELECT message FROM eteedir.messages LIMIT ?", (limit,))
            .await?
            .into_typed::<Message>();

        let vec: Vec<Message> = messages.try_collect().await?;

        Ok(vec)
    }

    // ORDER BY
    pub async fn read_by_order(&self, id: i64) -> Result<Vec<Message>, Box<dyn Error>> {
        let messages = self
            .session
            .query_iter(
                "SELECT message FROM eteedir.messages WHERE id = (?) ORDER BY timestamp",
                (id,),
            )
            .await?
            .into_typed::<Message>();

        let vec: Vec<Message> = messages.try_collect().await?;

        Ok(vec)
    }

    // IN
    pub async fn read_by_in(&self, id1: i64, id2: i64) -> Result<Vec<Message>, Box<dyn Error>> {
        let messages = self
            .session
            .query_iter(
                "SELECT message FROM eteedir.messages WHERE id in (?, ?)",
                (id1, id2),
            )
            .await?
            .into_typed::<Message>();

        let vec: Vec<Message> = messages.try_collect().await?;

        Ok(vec)
    }

    pub async fn update_message(
        &self,
        id: i64,
        timestamp: String,
        update: String,
    ) -> Result<(), Box<dyn Error>> {
        let datetime = chrono::DateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S%.f%z")?
            .with_timezone(&chrono::Utc)
            .timestamp_millis();
        let timestamp = scylla::frame::value::CqlTimestamp(datetime);
        self.session
            .query_unpaged(
                "UPDATE eteedir.messages SET message = (?) WHERE id = (?) AND timestamp = (?)",
                (update.clone(), id, timestamp),
            )
            .await?;

        Ok(())
    }

    pub async fn delete_message(&mut self, id: i64) -> Result<(), Box<dyn Error>> {
        self.session
            .query_unpaged("DELETE FROM eteedir.messages WHERE id = (?)", (id,))
            .await?;

        Ok(())
    }

    // BATCH
    pub async fn replace_user(
        &self,
        username: String,
        old_user_id: i64,
    ) -> Result<(), Box<dyn Error>> {
        let id = rand::thread_rng().gen::<i64>();
        let mut batch: Batch = Default::default();

        batch.append_statement("DELETE FROM eteedir.user WHERE id = (?) ");
        batch.append_statement("INSERT INTO eteedir.user (id, username) VALUES (?, ?)");

        let batch_values = ((old_user_id,), (id, username));

        self.session
            .batch(&batch, batch_values)
            .await
            .expect("Batch failed to batch things");

        Ok(())
    }
}
