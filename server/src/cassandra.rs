use futures::TryStreamExt;
use scylla::transport::iterator::TypedRowIterator;
use scylla::{IntoTypedRows, Session, SessionBuilder};
use std::error::Error;

pub struct Cassandra {
    pub session: Session,
    pub last_id: i64,
}

impl Cassandra {
    pub async fn new() -> Result<Cassandra, Box<dyn Error>> {
        let uri = std::env::var("").unwrap_or_else(|_| "127.0.0.1:9042".to_string());
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

    pub async fn insert_content(&mut self, content: String) -> Result<(), Box<dyn Error>> {
        self.last_id += 1;
        self.session
            .query_unpaged(
                "INSERT INTO eteedir.messages (id, message) VALUES(?, ?)",
                (self.last_id, content),
            )
            .await?;

        Ok(())
    }

    pub async fn update_content(&self, id: i64, update: String) -> Result<(), Box<dyn Error>> {
        self.session
            .query_unpaged(
                "UPDATE eteedir.messages SET message = (?) WHERE id = (?)",
                (update.clone(), id),
            )
            .await?;

        Ok(())
    }

    pub async fn read_content(&self) -> Result<TypedRowIterator<(String,)>, Box<dyn Error>> {
        let mut messages = self
            .session
            .query_iter("SELECT message FROM eteedir.messages", &[])
            .await?
            .into_typed::<(String,)>();

        while let Some(m) = messages.try_next().await? {
            println!("Message: {}", m.0);
        }

        Ok(messages)
    }

    pub async fn delete_content(&mut self, id: i64) -> Result<(), Box<dyn Error>> {
        self.session
            .query_unpaged("DELETE FROM eteedir.messages WHERE id = (?)", (id,))
            .await?;
        self.last_id = id;

        Ok(())
    }
}
