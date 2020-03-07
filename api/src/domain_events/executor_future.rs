use bigneon_db::prelude::*;
use chrono::prelude::*;
use crate::db::Connection;
use crate::errors::BigNeonError;
use futures::Async;
use futures::Future;

use log::Level::*;
use logging::*;

pub struct ExecutorFuture {
    started_at: NaiveDateTime,
    action: DomainAction,
    conn: Connection,
    inner: Box<dyn Future<Item = (), Error = BigNeonError>>,
}

unsafe impl Send for ExecutorFuture {}

impl ExecutorFuture {
    pub fn new(
        action: DomainAction,
        conn: Connection,
        future: Box<dyn Future<Item = (), Error = BigNeonError>>,
    ) -> ExecutorFuture {
        ExecutorFuture {
            action,
            conn,
            started_at: Utc::now().naive_utc(),
            inner: future,
        }
    }
}

impl Future for ExecutorFuture {
    type Item = ();
    type Error = BigNeonError;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        match self.inner.poll() {
            Ok(inner) => match inner {
                Async::Ready(r) => {
                    jlog!(Info,
                    "bigneon::domain_actions",
                     "Action succeeded",
                     { "domain_action_id": self.action.id,
                      "started_at": self.started_at,
                       "milliseconds_taken": (Utc::now().naive_utc() - self.started_at).num_milliseconds()
                        });
                    self.action.set_done(&self.conn.get())?;
                    self.conn.commit_transaction()?;
                    return Ok(Async::Ready(r));
                }
                Async::NotReady => return Ok(Async::NotReady),
            },
            Err(e) => {
                let desc = e.to_string();
                jlog!(Error,
                "bigneon::domain_actions",
                "Action failed",
                 { "domain_action_id": self.action.id,
                  "started_at": self.started_at,
                  "milliseconds_taken": (Utc::now().naive_utc() - self.started_at).num_milliseconds(),
                  "error": &desc
                   });
                jlog!(Error,
                "bigneon::domain_actions",
                "Rolling back transaction",
                 { "domain_action_id": self.action.id
                   });
                self.conn.rollback_transaction()?;

                self.action.set_failed(&desc, self.conn.get())?;
                return Err(e);
            }
        }
    }
}
