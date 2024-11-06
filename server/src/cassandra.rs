use chrono::Utc;
use futures::TryStreamExt;
use scylla::transport::iterator::TypedRowIterator;
use scylla::{IntoTypedRows, Session, SessionBuilder};
use std::error::Error;
use uuid::Uuid;

pub struct Cassandra {
    pub session: Session,
}

impl Cassandra {
    pub async fn new() -> Result<Cassandra, Box<dyn Error>> {
        let uri = std::env::var("").unwrap_or_else(|_| "127.0.0.1:9042".to_string());
        let session = SessionBuilder::new().known_node(uri).build().await?;

        Ok(Cassandra { session })
    }
    pub async fn insert_content(&self, content: String) -> Result<(), Box<dyn Error>> {
        let id = Uuid::new_v4();

        self.session
            .query_unpaged(
                "INSERT INTO eteedir.messages (id, message) VALUES(?, ?)",
                (id, content),
            )
            .await?;

        Ok(())
    }
    pub async fn update_content(
        &self,
        orginal: String,
        update: String,
    ) -> Result<(), Box<dyn Error>> {
        let column = self
            .session
            .query_unpaged(
                "SELECT id FROM eteedir.messages WHERE message = (?) ALLOW FILTERING",
                (orginal,),
            )
            .await?
            .rows;

        if let Some(rows) = column {
            for row in rows.into_typed::<(Uuid,)>() {
                let uuid = row?.0;
                self.session
                    .query_unpaged(
                        "UPDATE eteedir.messages SET message = (?) WHERE id = (?)",
                        (update.clone(), uuid),
                    )
                    .await?;
            }
        }

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
    pub async fn delete_content(&self, content: String) -> Result<(), Box<dyn Error>> {
        let column = self
            .session
            .query_unpaged(
                "SELECT id FROM eteedir.messages WHERE message = (?) ALLOW FILTERING",
                (content,),
            )
            .await?
            .rows;

        if let Some(rows) = column {
            for row in rows.into_typed::<(Uuid,)>() {
                let uuid = row?.0;
                self.session
                    .query_unpaged("DELETE FROM eteedir.messages WHERE id = (?)", (uuid,))
                    .await?;
            }
        }

        Ok(())
    }
}
