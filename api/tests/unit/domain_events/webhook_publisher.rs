use crate::support::database::TestDatabase;
use bigneon_api::auth::default_token_issuer::DefaultTokenIssuer;
use bigneon_api::config::Config;
use bigneon_api::domain_events::webhook_publisher::WebhookPublisher;
use bigneon_api::utils::deep_linker::BranchDeepLinker;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn webhook_payloads() {
    let project = TestDatabase::new();
    let connection = project.connection.get();
    let organization = project.create_organization().with_fees().finish();
    let config = Config::new(Environment::Test);
    let publisher = WebhookPublisher::new(
        "http://localhost:5432".to_string(),
        DefaultTokenIssuer::new("asdf".into(), "asdf".into()),
        Box::new(BranchDeepLinker::new(
            config.branch_io_base_url.clone(),
            config.branch_io_branch_key.clone(),
            500,
        )),
    );
    //let domain_event_publisher = project.create_domain_event_publisher().finish();
    let event_start = NaiveDateTime::parse_from_str("2055-06-14 16:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_venue(&venue)
        .with_organization(&organization)
        .with_event_start(event_start)
        .with_ticket_pricing()
        .finish();
    let email = "transfer-user@tari.com".to_string();
    let phone = "1-411-111-1111".to_string();
    let first_name = "Billy".to_string();
    let user = project
        .create_user()
        .with_email(email.clone())
        .with_phone(phone.clone())
        .with_first_name(&first_name)
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    // Missing main ID
    let domain_event = DomainEvent::create(
        DomainEventTypes::TransferTicketStarted,
        "Nothing to see here".to_string(),
        Tables::Transfers,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();
    assert!(publisher.create_webhook_payloads(&domain_event, connection).is_err());

    // With main ID
    let transfer = TicketInstance::create_transfer(
        &user,
        &[ticket.id],
        Some("test@tari.com"),
        Some(TransferMessageType::Email),
        false,
        connection,
    )
    .unwrap();
    let domain_event = DomainEvent::create(
        DomainEventTypes::TransferTicketStarted,
        "Nothing to see here".to_string(),
        Tables::Transfers,
        Some(transfer.id),
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let transfer_payloads = publisher.create_webhook_payloads(&domain_event, connection).unwrap();

    let transferer_payload = transfer_payloads
        .clone()
        .into_iter()
        .find(|p| fetch_from_payload::<String>(&p, "webhook_event_type") == "initiate_pending_transfer".to_string())
        .clone()
        .unwrap();
    assert_eq!(
        fetch_from_payload::<String>(&transferer_payload, "recipient_email"),
        "test@tari.com".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&transferer_payload, "show_start_date"),
        "Monday, 14 June 2055".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&transferer_payload, "show_start_time"),
        "9:00 AM PDT".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&transferer_payload, "show_doors_open_time"),
        "8:00 AM PDT".to_string()
    );
    assert_eq!(
        fetch_from_payload::<Option<Uuid>>(&transferer_payload, "recipient_id"),
        transfer.destination_temporary_user_id
    );
    assert!(!fetch_from_payload::<bool>(&transferer_payload, "direct_transfer"));
    assert_eq!(
        fetch_from_payload::<i64>(&transferer_payload, "number_of_tickets_transferred"),
        1
    );

    let recipient_payload = transfer_payloads
        .clone()
        .into_iter()
        .find(|p| fetch_from_payload::<String>(&p, "webhook_event_type") == "receive_pending_transfer".to_string())
        .clone()
        .unwrap();

    assert_eq!(
        fetch_from_payload::<Option<Uuid>>(&recipient_payload, "user_id"),
        transfer.destination_temporary_user_id
    );
    assert_eq!(
        fetch_from_payload::<Option<String>>(&recipient_payload, "transferer_first_name"),
        Some(first_name)
    );
    assert_eq!(
        fetch_from_payload::<Option<String>>(&recipient_payload, "transferer_email"),
        Some(email)
    );
    assert_eq!(
        fetch_from_payload::<Option<String>>(&recipient_payload, "transferer_phone"),
        Some(phone)
    );
    assert_eq!(
        fetch_from_payload::<i64>(&recipient_payload, "number_of_tickets_transferred"),
        1
    );
    assert_eq!(
        fetch_from_payload::<String>(&recipient_payload, "show_start_date"),
        "Monday, 14 June 2055".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&recipient_payload, "show_start_time"),
        "9:00 AM PDT".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&recipient_payload, "show_doors_open_time"),
        "8:00 AM PDT".to_string()
    );

    for transfer_payload in transfer_payloads {
        assert_eq!(fetch_from_payload::<Uuid>(&transfer_payload, "show_id"), event.id);
        assert_eq!(
            fetch_from_payload::<String>(&transfer_payload, "show_event_name"),
            event.name.clone()
        );
        assert_eq!(
            fetch_from_payload::<Uuid>(&transfer_payload, "organization_id"),
            organization.id
        );
    }

    let email = "test@tari.com".to_string();
    let phone = "1-000-000-0000".to_string();
    let user = project
        .create_user()
        .with_email(email.clone())
        .with_phone(phone.clone())
        .finish();

    let user_domain_event = DomainEvent::create(
        DomainEventTypes::UserCreated,
        "Nothing to see here".to_string(),
        Tables::Users,
        Some(user.id),
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let mut user_payloads = publisher
        .create_webhook_payloads(&user_domain_event, connection)
        .unwrap();
    assert_eq!(user_payloads.len(), 1);
    let user_payload = user_payloads.remove(0);
    assert_eq!(
        fetch_from_payload::<String>(&user_payload, "webhook_event_type"),
        "user_created".to_string()
    );
    assert_eq!(fetch_from_payload::<Uuid>(&user_payload, "user_id"), user.id);
    assert_eq!(
        fetch_from_payload::<i64>(&user_payload, "timestamp"),
        domain_event.created_at.timestamp()
    );
    assert_eq!(fetch_from_payload::<String>(&user_payload, "email"), email);
    assert_eq!(fetch_from_payload::<String>(&user_payload, "phone"), phone);

    let timezone = "Africa/Johannesburg".to_string();
    let venue = project.create_venue().with_timezone(timezone.clone()).finish();
    let event = project
        .create_event()
        .with_event_start(event_start)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let order = project
        .create_order()
        .for_user(&user)
        .for_event(&event)
        .quantity(2)
        .is_paid()
        .finish();
    let domain_event = DomainEvent::create(
        DomainEventTypes::OrderCompleted,
        "Nothing to see here".to_string(),
        Tables::Orders,
        Some(order.id),
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let mut order_payloads = publisher.create_webhook_payloads(&domain_event, connection).unwrap();

    assert_eq!(order_payloads.len(), 1);
    let order_payload = order_payloads.remove(0);

    assert_eq!(
        fetch_from_payload::<String>(&order_payload, "webhook_event_type"),
        "purchase_ticket".to_string()
    );
    assert_eq!(fetch_from_payload::<Uuid>(&order_payload, "user_id"), user.id);
    assert_eq!(
        fetch_from_payload::<i64>(&order_payload, "timestamp"),
        domain_event.created_at.timestamp()
    );
    assert_eq!(
        fetch_from_payload::<String>(&order_payload, "show_start_date"),
        "Monday, 14 June 2055".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&order_payload, "show_start_time"),
        "6:00 PM SAST".to_string()
    );
    assert_eq!(
        fetch_from_payload::<String>(&order_payload, "show_doors_open_time"),
        "5:00 PM SAST".to_string()
    );

    let email = "test@tari.com".to_string();
    let phone = "1-000-000-0000".to_string();
    let temporary_user = TemporaryUser::create(Uuid::new_v4(), Some(email.clone()), Some(phone.clone()))
        .commit(user.id, connection)
        .unwrap();

    let temporary_user_domain_event = DomainEvent::create(
        DomainEventTypes::TemporaryUserCreated,
        "Nothing to see here".to_string(),
        Tables::TemporaryUsers,
        Some(temporary_user.id),
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let mut temporary_user_payloads = publisher
        .create_webhook_payloads(&temporary_user_domain_event, connection)
        .unwrap();
    assert_eq!(temporary_user_payloads.len(), 1);
    let temporary_user_payload = temporary_user_payloads.remove(0);
    assert_eq!(
        fetch_from_payload::<String>(&temporary_user_payload, "webhook_event_type"),
        "temporary_user_created".to_string()
    );
    assert_eq!(
        fetch_from_payload::<Uuid>(&temporary_user_payload, "user_id"),
        temporary_user.id
    );
    assert_eq!(
        fetch_from_payload::<i64>(&temporary_user_payload, "timestamp"),
        domain_event.created_at.timestamp()
    );
    assert_eq!(fetch_from_payload::<String>(&temporary_user_payload, "email"), email);
    assert_eq!(fetch_from_payload::<String>(&temporary_user_payload, "phone"), phone);

    let push_token = PushNotificationToken::create(user.id, "source".to_string(), "token".to_string())
        .commit(user.id, connection)
        .unwrap();
    let push_token_domain_event = DomainEvent::create(
        DomainEventTypes::PushNotificationTokenCreated,
        "Nothing to see here".to_string(),
        Tables::PushNotificationTokens,
        Some(push_token.id),
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let mut push_token_payloads = publisher
        .create_webhook_payloads(&push_token_domain_event, connection)
        .unwrap();
    assert_eq!(push_token_payloads.len(), 1);
    let push_token_payload = push_token_payloads.remove(0);
    assert_eq!(
        fetch_from_payload::<String>(&push_token_payload, "webhook_event_type"),
        "user_device_tokens_added".to_string()
    );
    assert_eq!(fetch_from_payload::<Uuid>(&push_token_payload, "user_id"), user.id);
    assert_eq!(
        fetch_from_payload::<i64>(&push_token_payload, "timestamp"),
        push_token_domain_event.created_at.timestamp()
    );
    assert_eq!(
        fetch_from_payload::<String>(&push_token_payload, "token_source"),
        push_token.token_source.clone()
    );
    assert_eq!(
        fetch_from_payload::<String>(&push_token_payload, "token"),
        push_token.token.clone()
    );
    assert_eq!(
        fetch_from_payload::<i64>(&push_token_payload, "last_used"),
        push_token_domain_event.created_at.timestamp()
    );
}

fn fetch_from_payload<T>(payload: &HashMap<String, serde_json::Value>, key: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    serde_json::from_value::<T>(payload.get(&key.to_string()).unwrap().clone()).unwrap()
}
