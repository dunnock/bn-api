use bigneon_db::models::enums::DomainActionTypes;
use bigneon_db::prelude::*;
use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::errors::DomainActionError;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::executors::*;
use std::borrow::Borrow;
use std::collections::HashMap;

pub trait DomainActionExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture;
}

pub struct DomainActionRouter {
    routes: HashMap<DomainActionTypes, Box<dyn DomainActionExecutor>>,
}
impl DomainActionRouter {
    pub fn new() -> DomainActionRouter {
        DomainActionRouter { routes: HashMap::new() }
    }

    pub fn add_executor(
        &mut self,
        action_type: DomainActionTypes,
        executor: Box<dyn DomainActionExecutor>,
    ) -> Result<(), DomainActionError> {
        match self.routes.insert(action_type, executor) {
            Some(_) => Err(DomainActionError::Simple(
                "Action type already has an executor".to_string(),
            )),
            _ => Ok(()),
        }
    }

    pub fn get_executor_for(&self, action_type: DomainActionTypes) -> Option<&dyn DomainActionExecutor> {
        self.routes.get(&action_type).map(|o| (*o).borrow())
    }

    pub fn set_up_executors(&mut self, conf: &Config) {
        use self::DomainActionTypes::*;

        // This method is not necessary, but creates a compile time error
        // by using the `match` to identify DomainActionTypes that have not been catered for.
        // If you disagree with this approach or find a better way, feel free to unroll it.
        let find_executor = |action_type| -> Box<dyn DomainActionExecutor> {
            let conf = conf.clone();
            match action_type {
                Communication => Box::new(SendCommunicationExecutor::new(conf)),
                BroadcastPushNotification => Box::new(BroadcastPushNotificationExecutor::new(&conf)),
                FinalizeSettlements => Box::new(FinalizeSettlementsExecutor::new()),
                PaymentProviderIPN => Box::new(ProcessPaymentIPNExecutor::new(&conf)),
                RegenerateDripActions => Box::new(RegenerateDripActionsExecutor::new(conf)),
                SendPurchaseCompletedCommunication => Box::new(SendOrderCompleteExecutor::new(conf)),
                UpdateGenres => Box::new(UpdateGenresExecutor::new()),
                ProcessSettlementReport => Box::new(ProcessSettlementReportExecutor::new(conf)),
                ProcessTransferDrip => Box::new(ProcessTransferDripEventExecutor::new(conf)),
                RetargetAbandonedOrders => Box::new(RetargetAbandonedOrdersExecutor::new()),
                SendAutomaticReportEmails => Box::new(SendAutomaticReportEmailsExecutor::new(conf)),
                SubmitSitemapToSearchEngines => Box::new(SubmitSitemapToSearchEnginesExecutor::new(
                    conf.api_base_url.clone(),
                    conf.block_external_comms,
                )), //
                    // DO NOT add
                    // _ =>
            }
        };

        self.add_executor(Communication, find_executor(Communication))
            .expect("Configuration error");

        self.add_executor(BroadcastPushNotification, find_executor(BroadcastPushNotification))
            .expect("Configuration error");

        self.add_executor(FinalizeSettlements, find_executor(FinalizeSettlements))
            .expect("Configuration error");

        self.add_executor(PaymentProviderIPN, find_executor(PaymentProviderIPN))
            .expect("Configuration error");

        self.add_executor(ProcessSettlementReport, find_executor(ProcessSettlementReport))
            .expect("Configuration error");

        self.add_executor(ProcessTransferDrip, find_executor(ProcessTransferDrip))
            .expect("Configuration error");

        self.add_executor(RegenerateDripActions, find_executor(RegenerateDripActions))
            .expect("Configuration error");

        self.add_executor(RetargetAbandonedOrders, find_executor(RetargetAbandonedOrders))
            .expect("Configuration error");

        self.add_executor(UpdateGenres, find_executor(UpdateGenres))
            .expect("Configuration error");

        self.add_executor(SendAutomaticReportEmails, find_executor(SendAutomaticReportEmails))
            .expect("Configuration error");

        self.add_executor(
            SendPurchaseCompletedCommunication,
            find_executor(SendPurchaseCompletedCommunication),
        )
        .expect("Configuration error");
        self.add_executor(
            SubmitSitemapToSearchEngines,
            find_executor(SubmitSitemapToSearchEngines),
        )
        .expect("Configuration error")
    }
}
