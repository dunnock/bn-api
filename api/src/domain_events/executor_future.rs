use crate::database::Connection;
use crate::errors::ApiError;
use chrono::prelude::*;
use db::prelude::*;
use log::Level::*;
use logging::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ExecutorFuture {
    started_at: NaiveDateTime,
    action: DomainAction,
    conn: Connection,
    inner: Pin<Box<dyn Future<Output = Result<(), ApiError>>>>,
}

unsafe impl Send for ExecutorFuture {}

impl ExecutorFuture {
    pub fn new(
        action: DomainAction,
        conn: Connection,
        inner: Pin<Box<dyn Future<Output = Result<(), ApiError>>>>,
    ) -> ExecutorFuture {
        ExecutorFuture {
            action,
            conn,
            inner,
            started_at: Utc::now().naive_utc(),
        }
    }
}

impl Future for ExecutorFuture {
    type Output = Result<(), ApiError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(Ok(r)) => {
                jlog!(Info,
                "bigneon::domain_actions",
                    "Action succeeded",
                    { "domain_action_id": self.action.id,
                    "started_at": self.started_at,
                    "milliseconds_taken": (Utc::now().naive_utc() - self.started_at).num_milliseconds()
                    });
                self.action.set_done(&self.conn.get())?;
                self.conn.commit_transaction()?;
                Poll::Ready(Ok(r))
            }
            Poll::Ready(Err(e)) => {
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
                Poll::Ready(Err(e))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
