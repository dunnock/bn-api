use auth::default_token_issuer::DefaultTokenIssuer;
use bigneon_db::prelude::*;
use chrono::Duration;
use diesel::PgConnection;
use domain_events::errors::DomainActionError;
use errors::BigNeonError;
use log::Level::Error;
use serde_json::Value;
use std::collections::HashMap;
use utils::deep_linker::DeepLinker;

pub struct WebhookPublisher {
    pub front_end_url: String,
    pub token_issuer: DefaultTokenIssuer,
    pub deep_linker: Box<dyn DeepLinker>,
}

impl WebhookPublisher {
    pub fn new(front_end_url: String, token_issuer: DefaultTokenIssuer, deep_linker: Box<dyn DeepLinker>) -> Self {
        WebhookPublisher {
            front_end_url,
            token_issuer,
            deep_linker,
        }
    }
    pub fn publish(
        &self,
        domain_event_publisher: &DomainEventPublisher,
        domain_event: &DomainEvent,
        conn: &PgConnection,
    ) -> Result<(), DomainActionError> {
        for webhook_payload in self.create_webhook_payloads(&domain_event, conn)? {
            let mut comms = Communication::new(
                CommunicationType::Webhook,
                "Domain Event Webhook".to_string(),
                Some(json!(webhook_payload).to_string()),
                None,
                CommAddress::from(domain_event_publisher.webhook_url.clone()),
                None,
                None,
                Some(vec!["webhooks"]),
                None,
            );
            comms.main_table = Some(Tables::DomainEventPublishers);
            comms.main_table_id = Some(domain_event_publisher.id);
            comms.queue(conn)?;
        }
        Ok(())
    }

    pub fn create_webhook_payloads(
        &self,
        domain_event: &DomainEvent,
        conn: &PgConnection,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>, BigNeonError> {
        let mut result: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let main_id = domain_event.main_id.ok_or_else(|| {
            DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("Domain event id not present for webhook".to_string()),
            )
        })?;

        match domain_event.event_type {
            DomainEventTypes::UserCreated => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                let user = User::find(main_id, conn)?;
                data.insert("webhook_event_type".to_string(), json!("user_created"));
                data.insert("user_id".to_string(), json!(user.id));
                data.insert("email".to_string(), json!(user.email));
                data.insert("phone".to_string(), json!(user.phone));
                result.push(data);
            }
            DomainEventTypes::TemporaryUserCreated => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                let temporary_user = TemporaryUser::find(main_id, conn)?;
                data.insert("webhook_event_type".to_string(), json!("temporary_user_created"));
                data.insert("user_id".to_string(), json!(temporary_user.id));
                data.insert("email".to_string(), json!(temporary_user.email));
                data.insert("phone".to_string(), json!(temporary_user.phone));
                result.push(data);
            }
            DomainEventTypes::PushNotificationTokenCreated => {
                // Guard against future publisher processing after deletion
                if let Some(push_notification_token) = PushNotificationToken::find(main_id, conn).optional()? {
                    let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                    data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                    data.insert("webhook_event_type".to_string(), json!("user_device_tokens_added"));
                    data.insert("user_id".to_string(), json!(push_notification_token.user_id));
                    data.insert("token_source".to_string(), json!(push_notification_token.token_source));
                    data.insert("token".to_string(), json!(push_notification_token.token));
                    data.insert(
                        "last_used".to_string(),
                        json!(push_notification_token
                            .last_notification_at
                            .unwrap_or(push_notification_token.created_at)
                            .timestamp()),
                    );
                    result.push(data);
                }
            }
            DomainEventTypes::OrderCompleted => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let order = Order::find(main_id, conn)?;
                let user = order.user(conn)?;
                let order_id = order.id;
                self.order_payload_data(conn, &mut data, order)?;
                let magic_link_refresh_token =
                    user.create_magic_link_token(&self.token_issuer, Duration::minutes(15))?;
                let mut custom_data = HashMap::<String, Value>::new();
                custom_data.insert("refresh_token".to_string(), json!(&magic_link_refresh_token));
                custom_data.insert("order_id".to_string(), json!(order_id));
                custom_data.insert("domain_event".to_string(), json!(domain_event.event_type));

                let desktop_url = format!(
                    "{}/send-download-link?refresh_token={}",
                    self.front_end_url, magic_link_refresh_token
                );
                let link = self.deep_linker.create_with_custom_data(&desktop_url, custom_data)?;
                data.insert("magic_link".to_string(), json!(link));
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                data.insert("magic_link_refresh_token".to_string(), json!(magic_link_refresh_token));
                result.push(data);
            }
            DomainEventTypes::OrderRefund => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let order = Order::find(main_id, conn)?;
                self.order_payload_data(conn, &mut data, order)?;
                data.insert("webhook_event_type".to_string(), json!("refund_completed"));
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                result.push(data);
            }
            DomainEventTypes::OrderResendConfirmationTriggered => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let order = Order::find(main_id, conn)?;
                let order_has_refunds = order.has_refunds(conn)?;
                self.order_payload_data(conn, &mut data, order)?;

                if order_has_refunds {
                    data.insert("webhook_event_type".to_string(), json!("refund_completed"));
                }
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                result.push(data);
            }
            DomainEventTypes::OrderRetargetingEmailTriggered => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let order = Order::find(main_id, conn)?;
                if let Some(redemption_code) = order.redemption_code(conn)? {
                    data.insert("redemption_code".to_string(), json!(redemption_code));
                }
                self.order_payload_data(conn, &mut data, order)?;
                data.insert("webhook_event_type".to_string(), json!("abandoned_cart_email"));
                data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                result.push(data);
            }
            DomainEventTypes::TransferTicketStarted
            | DomainEventTypes::TransferTicketCancelled
            | DomainEventTypes::TransferTicketCompleted => {
                // Sender is associated with their main account
                // Receiver is associated with the v3 UUID of their destination address
                // Receiver has a temp account made for them in the customer system on TransferTicketStarted
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let transfer = Transfer::find(main_id, conn).optional()?;
                // There is a historic bug where a transfer did not exist, unfortunately
                // will have to skip those
                if let Some(transfer) = transfer {
                    data.insert("direct_transfer".to_string(), json!(transfer.direct));
                    data.insert(
                        "number_of_tickets_transferred".to_string(),
                        json!(transfer.transfer_ticket_count(conn)?),
                    );

                    data.insert("timestamp".to_string(), json!(domain_event.created_at.timestamp()));
                    let mut events = transfer.events(conn)?;
                    // TODO: lock down transfers to have only one event
                    if let Some(event) = events.pop() {
                        Event::event_payload_data(&event, &self.front_end_url, &mut data, conn)?;
                    }
                    let mut recipient_data = data.clone();
                    let mut transferer_data = data;

                    self.recipient_payload_data(
                        &transfer,
                        &self.front_end_url,
                        domain_event.event_type,
                        &mut recipient_data,
                        conn,
                    )?;
                    result.push(recipient_data);

                    Self::transferer_payload_data(&transfer, domain_event.event_type, &mut transferer_data, conn)?;
                    result.push(transferer_data);
                } else {
                    jlog!(
                        Error,
                        "bigneon-db::models::domain_events",
                        "Could not find transfer for id",
                        { "domain_event": &domain_event }
                    );
                }
            }
            _ => {
                return Err(DatabaseError::new(
                    ErrorCode::BusinessProcessError,
                    Some("Domain event type not supported".to_string()),
                )
                .into());
            }
        }

        Ok(result)
    }

    fn order_payload_data(
        &self,
        conn: &PgConnection,
        data: &mut HashMap<String, Value>,
        order: Order,
    ) -> Result<(), DatabaseError> {
        if let Some(event) = order.events(conn)?.pop() {
            Event::event_payload_data(&event, &self.front_end_url, data, conn)?;
        }
        data.insert("webhook_event_type".to_string(), json!("purchase_ticket"));
        data.insert("order_number".to_string(), json!(order.order_number()));
        let user = order.user(conn)?;
        data.insert("customer_email".to_string(), json!(user.email));
        data.insert("customer_first_name".to_string(), json!(user.first_name));
        data.insert("customer_last_name".to_string(), json!(user.last_name));

        #[derive(Serialize)]
        struct R {
            ticket_type: Option<String>,
            price: i64,
            quantity: i64,
            total: i64,
            refunded_quantity: i64,
            refunded_total: i64,
        };

        let mut count = 0;
        let mut sub_total = 0;
        let mut refunded_sub_total = 0;
        let mut fees_total = 0;
        let mut refunded_fees_total = 0;
        let mut discount_total = 0;
        let mut refunded_discount_total = 0;
        let mut j_items = Vec::<R>::new();
        for item in order.items(conn)? {
            let item_total = item.unit_price_in_cents * item.quantity;
            let refunded_total = item.unit_price_in_cents * item.refunded_quantity;
            j_items.push(R {
                ticket_type: item.ticket_type(conn)?.map(|tt| tt.name),
                price: item.unit_price_in_cents,
                quantity: item.quantity,
                refunded_quantity: item.refunded_quantity,
                total: item_total,
                refunded_total,
            });

            match item.item_type {
                OrderItemTypes::Tickets => {
                    count = count + item.quantity - item.refunded_quantity;
                    sub_total = sub_total + item_total;
                    refunded_sub_total = refunded_sub_total + refunded_total;
                }
                OrderItemTypes::Discount => {
                    discount_total = discount_total + item_total;
                    refunded_discount_total = refunded_discount_total + refunded_total;
                }
                OrderItemTypes::PerUnitFees | OrderItemTypes::EventFees | OrderItemTypes::CreditCardFees => {
                    fees_total = fees_total + item_total;
                    refunded_fees_total = refunded_fees_total + refunded_total;
                }
            }
        }

        data.insert("items".to_string(), json!(j_items));
        data.insert("ticket_count".to_string(), json!(count));
        data.insert("subtotal".to_string(), json!(sub_total));
        data.insert("refunded_subtotal".to_string(), json!(refunded_sub_total));
        data.insert("fees_total".to_string(), json!(fees_total));
        data.insert("refunded_fees_total".to_string(), json!(refunded_fees_total));
        data.insert("discount_total".to_string(), json!(discount_total));
        data.insert("refunded_discount_total".to_string(), json!(refunded_discount_total));

        data.insert(
            "user_id".to_string(),
            json!(order.on_behalf_of_user_id.unwrap_or(order.user_id)),
        );

        Ok(())
    }

    fn transferer_payload_data(
        transfer: &Transfer,
        event_type: DomainEventTypes,
        data: &mut HashMap<String, serde_json::Value>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        data.insert("user_id".to_string(), json!(transfer.source_user_id));
        data.insert(
            "recipient_id".to_string(),
            json!(transfer.destination_temporary_user_id.or(transfer.destination_user_id)),
        );

        let recipient = if let Some(destination_user_id) = transfer.destination_user_id {
            Some(User::find(destination_user_id, conn)?)
        } else {
            None
        };
        let mut email = recipient.clone().map(|r| r.email.clone()).unwrap_or(None);
        if let Some(transfer_message_type) = transfer.transfer_message_type {
            if transfer_message_type == TransferMessageType::Email {
                email = email.or(transfer.transfer_address.clone());
            }
        }
        let mut phone = recipient.clone().map(|r| r.phone.clone()).unwrap_or(None);
        if let Some(transfer_message_type) = transfer.transfer_message_type {
            if transfer_message_type == TransferMessageType::Phone {
                phone = phone.or(transfer.transfer_address.clone());
            }
        }

        data.insert(
            "webhook_event_type".to_string(),
            json!(match event_type {
                DomainEventTypes::TransferTicketCancelled => {
                    if transfer.cancelled_by_user_id == Some(transfer.source_user_id) {
                        "cancel_pending_transfer"
                    } else {
                        "initiated_transfer_declined"
                    }
                }
                DomainEventTypes::TransferTicketCompleted => "initiated_transfer_claimed",
                DomainEventTypes::TransferTicketStarted => "initiate_pending_transfer",
                _ => {
                    return Err(DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Domain event type not supported".to_string()),
                    ));
                }
            }),
        );

        data.insert(
            "recipient_first_name".to_string(),
            json!(recipient.map(|r| r.first_name)),
        );
        data.insert("recipient_email".to_string(), json!(email));
        data.insert("recipient_phone".to_string(), json!(phone));

        let transferer = User::find(transfer.source_user_id, conn)?;

        data.insert("transferer_email".to_string(), json!(transferer.email));
        data.insert("transferer_phone".to_string(), json!(transferer.phone));

        Ok(())
    }

    fn recipient_payload_data(
        &self,
        transfer: &Transfer,
        front_end_url: &str,
        event_type: DomainEventTypes,
        data: &mut HashMap<String, serde_json::Value>,
        conn: &PgConnection,
    ) -> Result<(), BigNeonError> {
        let transferer = User::find(transfer.source_user_id, conn)?;
        let mut receive_tickets_url = transfer.receive_url(&self.front_end_url, conn)?;
        data.insert(
            "user_id".to_string(),
            json!(transfer.destination_temporary_user_id.or(transfer.destination_user_id)),
        );
        if let Some(user_id) = transfer.destination_user_id {
            // TODO: Implement magic link for temp users
            let user = User::find(user_id, conn)?;
            let magic_link_refresh_token = user.create_magic_link_token(&self.token_issuer, Duration::days(90))?;
            let mut custom_data = HashMap::<String, Value>::new();

            custom_data.insert("refresh_token".to_string(), json!(&magic_link_refresh_token));
            custom_data.insert("transfer_id".to_string(), json!(transfer.id));
            custom_data.insert("domain_event".to_string(), json!(event_type));

            let desktop_url = format!("{}/my-events?refresh_token={}", front_end_url, magic_link_refresh_token);
            let link = self.deep_linker.create_with_custom_data(&desktop_url, custom_data)?;
            receive_tickets_url = link
        } else if let Some(temp_id) = transfer.destination_temporary_user_id {
            let token = self.token_issuer.issue_with_limited_scopes(
                temp_id,
                vec![Scopes::TemporaryUserPromote],
                Duration::days(90),
            )?;
            let mut custom_data = HashMap::<String, Value>::new();

            custom_data.insert("refresh_token".to_string(), json!(&token));
            custom_data.insert("transfer_id".to_string(), json!(transfer.id));
            custom_data.insert("domain_event".to_string(), json!(event_type));

            let desktop_url = format!("{}&refresh_token={}", receive_tickets_url, token);
            let link = self.deep_linker.create_with_custom_data(&desktop_url, custom_data)?;
            receive_tickets_url = link
        }

        data.insert("receive_tickets_url".to_string(), json!(receive_tickets_url));
        data.insert("transferer_first_name".to_string(), json!(transferer.first_name));

        data.insert(
            "webhook_event_type".to_string(),
            json!(match event_type {
                DomainEventTypes::TransferTicketCancelled => {
                    if transfer.cancelled_by_user_id == Some(transfer.source_user_id) {
                        "received_transfer_cancelled"
                    } else {
                        "decline_pending_transfer"
                    }
                }
                DomainEventTypes::TransferTicketCompleted => "claim_pending_transfer",
                DomainEventTypes::TransferTicketStarted => "receive_pending_transfer",
                _ => {
                    return Err(DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Domain event type not supported".to_string()),
                    )
                    .into());
                }
            }),
        );

        data.insert("transferer_email".to_string(), json!(transferer.email));
        data.insert("transferer_phone".to_string(), json!(transferer.phone));

        if transfer.transfer_message_type == Some(TransferMessageType::Email) {
            data.insert("recipient_email".to_string(), json!(transfer.transfer_address));
        };
        if transfer.transfer_message_type == Some(TransferMessageType::Phone) {
            data.insert("recipient_phone".to_string(), json!(transfer.transfer_address));
        };
        Ok(())
    }
}
