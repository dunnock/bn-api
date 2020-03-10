use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use bigneon_db::prelude::*;
use diesel::PgConnection;
use futures::future;
use itertools::Itertools;
use log::Level::Error;
use validator::HasLen;

pub struct BroadcastPushNotificationExecutor {
    template_id: Option<String>,
}

impl DomainActionExecutor for BroadcastPushNotificationExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Send tickets mail action failed", {"action_id": action.id, "main_table_id":action.main_table_id,  "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl BroadcastPushNotificationExecutor {
    pub fn new(config: &Config) -> BroadcastPushNotificationExecutor {
        BroadcastPushNotificationExecutor {
            template_id: Some(config.email_templates.custom_broadcast.to_string()),
        }
    }

    fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        let broadcast_id = action.main_table_id.ok_or(ApplicationError::new(
            "No broadcast id attached to domain action".to_string(),
        ))?;
        let broadcast = Broadcast::find(broadcast_id, conn)?;
        if broadcast.status == BroadcastStatus::Cancelled {
            return Ok(());
        }

        let broadcast = broadcast.set_in_progress(conn)?;
        let message = broadcast.message.clone();
        let message = message.unwrap_or("".to_string());
        let message = match broadcast.notification_type {
            BroadcastType::LastCall => {
                "🗣LAST CALL! 🍻The bar is closing soon, grab something now before it's too late!"
            }
            BroadcastType::Custom => message.as_str(),
        };

        let audience: Vec<User> = match broadcast.audience {
            BroadcastAudience::PeopleAtTheEvent => Event::checked_in_users(broadcast.event_id, conn)?
                .into_iter()
                .map(|u| (u, Vec::new(), None))
                .collect_vec(),
            BroadcastAudience::OrganizationMembers => Event::find_organization_users(broadcast.event_id, conn)?
                .into_iter()
                .map(|u| (u, Vec::new(), None))
                .collect_vec(),
            BroadcastAudience::TicketHolders => {
                Event::find_all_ticket_holders(broadcast.event_id, conn, TicketHoldersCountType::WithEmailAddress)?
            }
        }
        .into_iter()
        .map(|aud| aud.0)
        .collect_vec();

        //Set a default sent count of the audience length, this is changed if the broadcast channel is an email
        let mut set_count = audience.length() as i64;

        // if preview email, only send and nothing to the audience
        if broadcast.preview_email != None {
            queue_email_notification(
                &broadcast,
                conn,
                self.template_id.clone(),
                message.to_string(),
                "",
                broadcast.preview_email.clone(),
            )?;
            return Ok(());
        }

        match broadcast.channel {
            BroadcastChannel::PushNotification => {
                for user in audience {
                    queue_push_notification(&broadcast, message.to_string(), &user, conn)?;
                }
            }
            BroadcastChannel::Email => {
                let mut emails: Vec<String> = audience.into_iter().filter_map(|u| u.email).collect();
                emails.sort();
                emails.dedup();
                set_count = emails.length() as i64;
                for email_address in emails {
                    queue_email_notification(
                        &broadcast,
                        conn,
                        self.template_id.clone(),
                        message.to_string(),
                        email_address.as_str(),
                        broadcast.preview_email.clone(),
                    )?
                }
            }
        }

        Broadcast::set_sent_count(broadcast_id, set_count, conn)?;

        Ok(())
    }
}

fn queue_push_notification(
    broadcast: &Broadcast,
    message: String,
    user: &User,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let tokens = user
        .push_notification_tokens(conn)?
        .into_iter()
        .map(|pt| pt.token)
        .collect_vec();

    if tokens.len() > 0 {
        DomainAction::create(
            None,
            DomainActionTypes::Communication,
            Some(CommunicationChannelType::Push),
            serde_json::to_value(Communication::new(
                CommunicationType::Push,
                message,
                None,
                None,
                CommAddress::from_vec(tokens),
                None,
                None,
                Some(vec!["broadcast"]),
                Some(
                    [("broadcast_id".to_string(), json!(broadcast.id))]
                        .iter()
                        .cloned()
                        .collect(),
                ),
            ))?,
            Some(Tables::Events),
            Some(broadcast.event_id),
        )
        .commit(conn)?;
    }

    Ok(())
}

fn queue_email_notification(
    broadcast: &Broadcast,
    conn: &PgConnection,
    template_id: Option<String>,
    message: String,
    email_address: &str,
    preview_email: Option<String>,
) -> Result<(), BigNeonError> {
    let email = match preview_email {
        None => CommAddress::from(email_address.to_string()),
        Some(e) => CommAddress::from(e),
    };

    DomainAction::create(
        None,
        DomainActionTypes::Communication,
        Some(CommunicationChannelType::Email),
        serde_json::to_value(Communication::new(
            CommunicationType::EmailTemplate,
            broadcast.subject.as_ref().unwrap_or(&broadcast.name).to_string(),
            Some(message),
            None,
            email,
            template_id,
            None,
            Some(vec!["broadcast"]),
            Some(
                [
                    ("broadcast_id".to_string(), json!(broadcast.id)),
                    ("event_id".to_string(), json!(broadcast.event_id)),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        ))?,
        Some(Tables::Events),
        Some(broadcast.event_id),
    )
    .commit(conn)?;
    Ok(())
}
