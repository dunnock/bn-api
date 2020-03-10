use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use bigneon_db::prelude::*;
use futures::future;
use log::Level::Error;
use uuid::Uuid;

pub struct UpdateGenresExecutor {}

#[derive(Deserialize, Serialize)]
pub struct UpdateGenresPayload {
    pub user_id: Uuid,
}

impl DomainActionExecutor for UpdateGenresExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Update genres action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl UpdateGenresExecutor {
    pub fn new() -> UpdateGenresExecutor {
        UpdateGenresExecutor {}
    }

    pub fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        let id = action
            .main_table_id
            .clone()
            .ok_or(ApplicationError::new("No id supplied in the action".to_string()))?;

        let payload: UpdateGenresPayload = serde_json::from_value(action.payload.clone())?;

        match action
            .main_table
            .clone()
            .ok_or(ApplicationError::new("No table supplied in the action".to_string()))?
        {
            Tables::Artists => {
                for event in Artist::find(&id, conn)?.events(conn)? {
                    event.update_genres(Some(payload.user_id), conn)?;
                }
            }
            Tables::Events => {
                Event::find(id, conn)?.update_genres(Some(payload.user_id), conn)?;
            }
            Tables::Users => {
                User::find(id, conn)?.update_genre_info(conn)?;
            }
            _ => return Err(ApplicationError::new("Table not supported".to_string()).into()),
        };

        Ok(())
    }
}
