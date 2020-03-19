use crate::config::Config;
use crate::database::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::utils::communication;
use db::prelude::*;

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
        let config = self.config.clone();
        let action2 = action.clone();
        let conn2 = conn.clone();
        let future = async move { communication::send_async(&action2, &config, conn2.get()).await };
        ExecutorFuture::new(action, conn, Box::pin(future))
    }
}
