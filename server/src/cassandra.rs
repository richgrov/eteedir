use futures::TryStreamExt;
use scylla::{IntoTypedRows, Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub content: String,
}

pub struct Cassandra {
    session: Session,
    last_id: i64,
}

impl Cassandra {
    pub async fn new(address: impl AsRef<str>) -> Result<Cassandra, Box<dyn Error>> {
        let uri = address;
        let session = SessionBuilder::new().known_node(uri).build().await?;

        let mut message = session
            .query_unpaged("SELECT MAX(id) FROM eteedir.messages", &[])
            .await?
            .rows
            .expect("Could not get max id")
            .into_typed::<(i64,)>();

        let id: i64 = message
            .next()
            .expect("error")
            .expect("error strikes back")
            .0;

        Ok(Cassandra {
            session,
            last_id: id,
        })
    }

    pub async fn insert_message(&self, message: &Message) -> Result<(), Box<dyn Error>> {
        // self.last_id += 1;
        self.session
            .query_unpaged(
                "INSERT INTO eteedir.messages (id, message) VALUES(?, ?)",
                (self.last_id, message.content.clone()),
            )
            .await?;

        Ok(())
    }

    pub async fn update_message(&self, id: i64, update: String) -> Result<(), Box<dyn Error>> {
        self.session
            .query_unpaged(
                "UPDATE eteedir.messages SET message = (?) WHERE id = (?)",
                (update.clone(), id),
            )
            .await?;

        Ok(())
    }

    pub async fn read_messages(&self) -> Result<Vec<Message>, Box<dyn Error>> {
        let mut messages = self
            .session
            .query_iter("SELECT message FROM eteedir.messages", &[])
            .await?
            .into_typed::<(String,)>();

        let mut vec = Vec::new();
        while let Some(m) = messages.try_next().await? {
            vec.push(Message { content: m.0 });
        }

        Ok(vec)
    }

    pub async fn delete_message(&mut self, id: i64) -> Result<(), Box<dyn Error>> {
        self.session
            .query_unpaged("DELETE FROM eteedir.messages WHERE id = (?)", (id,))
            .await?;

        Ok(())
    }
}
