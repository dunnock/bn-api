use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::utils::communication;
use bigneon_db::prelude::*;

pub struct SendCommunicationExecutor {
    config: Config,
}

impl SendCommunicationExecutor {
    pub fn new(config: Config) -> SendCommunicationExecutor {
        SendCommunicationExecutor { config }
    }
}

impl DomainActionExecutor for SendCommunicationExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        let future = communication::send_async(&action, &self.config, conn.get());
        ExecutorFuture::new(action, conn, Box::new(future))
    }
}
