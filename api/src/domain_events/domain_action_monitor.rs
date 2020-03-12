use std::error::Error;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{cmp, thread};

use log::Level::*;

use crate::config::Config;
use crate::db::*;
use crate::domain_events::errors::DomainActionError;
use crate::domain_events::routing::{DomainActionExecutor, DomainActionRouter};
use bigneon_db::prelude::*;
use futures::future::TryFutureExt;
use logging::*;
use tokio::runtime::Runtime;
use tokio::time::timeout;

pub struct DomainActionMonitor {
    config: Config,
    database: Database,
    worker_threads: Vec<(Sender<()>, JoinHandle<Result<(), DomainActionError>>)>,
    interval: u64,
}

impl DomainActionMonitor {
    pub fn new(conf: Config, database: Database, poll_period_in_secs: u64) -> DomainActionMonitor {
        DomainActionMonitor {
            config: conf,
            database,
            worker_threads: vec![],
            interval: poll_period_in_secs,
        }
    }

    pub async fn run_til_empty(&self) -> Result<(), DomainActionError> {
        let router = DomainActionMonitor::create_router(&self.config);

        loop {
            let mut num_processed = 0;

            let futures = DomainActionMonitor::find_actions(
                &self.database,
                &router,
                cmp::max(1, self.config.connection_pool.max / 2) as usize,
            )?;

            for (executor, domain_action, connection) in futures {
                if let Err(err) = timeout(Duration::from_secs(55), executor.execute(domain_action, connection)).await {
                    jlog! {Error,"bigneon::domain_actions", "Action: failed", {"error": err.to_string()}};
                }
                num_processed += 1;
            }

            if num_processed == 0 {
                break;
            }
        }
        Ok(())
    }

    fn find_and_publish_events(config: &Config, database: &Database) -> Result<usize, DomainActionError> {
        let conn = database.get_connection()?;

        let connection = conn.get();

        let mut domain_event_publishers = DomainEventPublisher::find_all(connection)?;

        if domain_event_publishers.len() == 0 {
            return Ok(0);
        };
        let mut events_published = 0;

        let last_seq_pub = domain_event_publishers
            .first()
            .unwrap()
            .last_domain_event_seq
            .unwrap_or(-1);

        let domain_events = DomainEvent::find_after_seq(last_seq_pub, 500, connection)?;

        if domain_events.len() > 0 {
            for publisher in domain_event_publishers.iter_mut() {
                match &mut publisher.acquire_lock(60, connection) {
                    Ok(_) => {
                        for event in &domain_events {
                            conn.begin_transaction()?;
                            if publisher.last_domain_event_seq.unwrap_or(-1) < event.seq
                                && publisher.event_types.contains(&event.event_type)
                                && (publisher.organization_id.is_none()
                                    || publisher.organization_id == event.organization_id)
                            {
                                jlog!(Info, "bigneon::domain_events", "Publishing event", {"publisher_id": publisher.id, "event_type": &event.event_type, "organization_id": event.organization_id, "event": &event});
                                publisher.publish(&event, &config.front_end_url, connection)?;
                            }
                            publisher.update_last_domain_event_seq(event.seq, connection)?;
                            conn.commit_transaction()?;
                            events_published += 1;
                            publisher.renew_lock(60, connection)?
                        }
                        publisher.release_lock(connection)?;
                    }
                    _ => continue,
                }
            }
        }

        Ok(events_published)
    }

    pub fn publish_events_to_actions(
        config: Config,
        database: Database,
        interval: u64,
        rx: Receiver<()>,
    ) -> Result<(), DomainActionError> {
        loop {
            if rx.try_recv().is_ok() {
                jlog!(Info, "bigneon::domain_actions", "Stopping events processor", {});
                break;
            }

            // Domain Monitor main loop
            if DomainActionMonitor::find_and_publish_events(&config, &database)? == 0 {
                //                jlog!(Info, "bigneon::domain_events", "No events founds, sleeping", {});
                thread::sleep(Duration::from_secs(interval));
            }
        }
        Ok(())
    }

    fn create_router(conf: &Config) -> DomainActionRouter {
        let mut router = DomainActionRouter::new();

        router.set_up_executors(conf);
        router
    }

    fn find_actions<'a>(
        database: &Database,
        router: &'a DomainActionRouter,
        limit: usize,
    ) -> Result<Vec<(&'a dyn DomainActionExecutor, DomainAction, Connection)>, DomainActionError> {
        let connection = database.get_connection()?;

        let pending_actions = DomainAction::find_pending(None, connection.get())?;

        if pending_actions.len() == 0 {
            jlog!(Trace, "bigneon::domain_actions", "Found no actions to process", {});
            return Ok(vec![]);
        }

        jlog!(
        Debug,
        "bigneon::domain_actions",
        "Found actions to process",
        { "action_count": pending_actions.len() }
        );

        let mut result = vec![];

        // //Process actions
        let len = pending_actions.len();
        for (index, action) in pending_actions.into_iter().enumerate() {
            if limit < index {
                break;
            }
            jlog! {Info, &format!("Pending Action: {}", action.domain_action_type), {"id":action.id, "domain_action_type": action.domain_action_type}};
            let connection = connection.get();
            let per_action_connection = match database.get_connection() {
                Ok(conn) => conn,
                Err(e) => {
                    // Assume connection pool is full
                    jlog!(
                    Info,
                    "bigneon::domain_actions",
                    "Hit connection pool maximum",
                    { "number_of_connections_used": index, "pending_actions": len, "connection_error": e.description() }
                    );

                    break;
                }
            };

            match action.set_busy(60, connection) {
                Ok(_) => {}
                Err(e) => match e.error_code {
                    ErrorCode::ConcurrencyError => {
                        jlog! {Debug, &format!("Action was already checked out to another process: {}", action.id)};
                        continue;
                    }
                    _ => return Err(e.into()),
                },
            };
            let command = router.get_executor_for(action.domain_action_type);
            if command.is_none() {
                action.set_errored("Not executor has been created for this action type", &connection)?;

                return Err(DomainActionError::Simple(format!(
                    "Could not find executor for this action type:{}",
                    action.domain_action_type
                )));
            }
            let command = command.unwrap();

            per_action_connection.begin_transaction()?;
            // let f = command.execute(action, per_action_connection);
            result.push((command, action, per_action_connection));
        }

        Ok(result)
    }

    #[allow(unreachable_code)]
    pub fn run_actions(
        conf: Config,
        database: Database,
        interval: u64,
        rx: Receiver<()>,
    ) -> Result<(), DomainActionError> {
        let router = DomainActionMonitor::create_router(&conf);

        let runtime = Runtime::new()?;

        loop {
            if rx.try_recv().is_ok() {
                jlog!(Info, "bigneon::domain_actions", "Stopping actions processor", {});
                break;
            }
            //Check for actions that are due to be processed

            let actions = DomainActionMonitor::find_actions(
                &database,
                &router,
                cmp::max(1, conf.connection_pool.max / 2) as usize,
            )?;

            if actions.len() == 0 {
                thread::sleep(Duration::from_secs(interval));
            } else {
                for (command, action, connection) in actions {
                    let timeout = timeout(Duration::from_secs(55), command.execute(action, connection));

                    runtime.spawn(timeout.or_else(|err| async move {
                        jlog! {Error,"bigneon::domain_actions", "Action:  failed", {"error": err.to_string()}};
                        Err(())
                    }));
                }
            }
        }
        Ok(())
    }

    pub fn start(&mut self, run_actions: bool, run_events: bool) {
        // Use this channel to tell the server to shut down
        let (actions_tx, actions_rx) = mpsc::channel::<()>();

        let (events_tx, events_rx) = mpsc::channel::<()>();

        let actions_stop_signals = vec![events_tx.clone()];

        let events_stop_signals = vec![actions_tx.clone()];
        if run_actions {
            jlog!(Info, "bigneon::domain_actions", "Domain action monitor starting", {});
            let config = self.config.clone();
            let database = self.database.clone();
            let interval = self.interval;

            // Create a worker thread to run domain actions
            self.worker_threads.push((
                actions_tx,
                thread::spawn(move || {
                    let res = DomainActionMonitor::run_actions(config, database, interval, actions_rx).map_err(|e| {
                        jlog!(
                            Error,
                            "bigneon::domain_actions",
                            "Domain event publisher failed", {"error": e.to_string()}
                        );
                        e
                    });

                    for signal in actions_stop_signals {
                        match signal.send(()) {
                            Ok(_) => (),
                            Err(_) => {
                                // Other thread may have failed
                                ()
                            }
                        }
                    }
                    res
                }),
            ));
        }

        if run_events {
            jlog!(Info, "bigneon::domain_actions", "Domain event publisher starting", {});
            let database = self.database.clone();
            let interval = self.interval;

            let config = self.config.clone();

            // Create a worker thread to publish events to subscribers
            self.worker_threads.push((
                events_tx,
                thread::spawn(move || {
                    let res = DomainActionMonitor::publish_events_to_actions(config, database, interval, events_rx)
                        .map_err(|e| {
                            jlog!(
                                Error,
                                "bigneon::domain_actions",
                                "Domain event publisher failed", {"error": e.to_string()}
                            );
                            e
                        });

                    for signal in events_stop_signals {
                        match signal.send(()) {
                            Ok(_) => (),
                            Err(_) => {
                                // Other thread may have failed
                                ()
                            }
                        }
                    }

                    res
                }),
            ));
        }
    }

    pub fn wait_for_end(&mut self) {
        let mut results = vec![];
        for w in self.worker_threads.drain(..) {
            results.push(w.1.join());
        }
        for r in results {
            r.expect("Thread did not end successfully")
                .expect("Error returned from thread");
        }
    }

    pub fn stop(&mut self) {
        for w in self.worker_threads.drain(..) {
            w.0.send(()).unwrap();
            w.1.join().unwrap().unwrap();
        }
    }
}
