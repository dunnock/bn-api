use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::schema::{orders, refunds};
use bigneon_db::services::CountryLookup;
use bigneon_db::utils::dates;
use bigneon_db::utils::errors::DatabaseError;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn associated_with_active_orders() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().with_ticket_type_count(2).finish();
    let ticket_types = &event.ticket_types(true, None, project.get_connection()).unwrap();
    let ticket_type = &ticket_types[0];
    assert!(!event.associated_with_active_orders(connection).unwrap());

    // Unpaid order but in cart, not yet expired
    let order = project.create_order().for_tickets(ticket_type.id).finish();
    assert!(event.associated_with_active_orders(connection).unwrap());

    // Expire the order
    diesel::sql_query(
        r#"
        UPDATE orders
        SET expires_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(order.id)
    .execute(connection)
    .unwrap();
    assert!(!event.associated_with_active_orders(connection).unwrap());

    // Paid order
    let order = project.create_order().for_tickets(ticket_type.id).is_paid().finish();
    assert!(event.associated_with_active_orders(connection).unwrap());

    // Expire the order but it's paid so still counts
    diesel::sql_query(
        r#"
        UPDATE orders
        SET expires_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(order.id)
    .execute(connection)
    .unwrap();
    assert!(event.associated_with_active_orders(connection).unwrap());
}

#[test]
fn clone_record() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Clone event with multiple artists and ticket types
    let artist = project.create_artist().with_name("Artist 1".to_string()).finish();
    let artist2 = project.create_artist().with_name("Artist 2".to_string()).finish();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_name("Original Name".to_string())
        .as_private("SECRET".to_string())
        .with_additional_info("Additional Info".to_string())
        .with_event_type(EventTypes::Sports)
        .with_venue(&venue)
        .with_door_time(dates::now().add_days(7).add_hours(-1).finish())
        .with_event_start(dates::now().add_days(7).finish())
        .with_event_end(dates::now().add_days(8).finish())
        .with_tickets()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    assert!(event.cloned_from_event_id.is_none());
    let ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let child_ticket_type = event
        .add_ticket_type(
            "Child ticket type".to_string(),
            None,
            105,
            None,
            Some(dates::now().add_hours(-1).finish()),
            TicketTypeEndDateType::Manual,
            Some(event.issuer_wallet(connection).unwrap().id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            Some(ticket_type.id),
            0,
            true,
            true,
            true,
            None,
            connection,
        )
        .unwrap();

    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist2)
        .finish();

    let clone_fields = CloneFields {
        name: "New Event Name".to_string(),
        event_start: dates::now().add_days(14).finish(),
        event_end: dates::now().add_days(15).finish(),
    };

    let cloned_event = event.clone_record(&clone_fields, None, connection).unwrap();

    // Newly cloned event uses fields provided by clone fields struct
    assert_ne!(cloned_event.id, event.id);
    assert_eq!(&cloned_event.name, &clone_fields.name);
    assert_eq!(
        cloned_event.event_start.unwrap().timestamp(),
        clone_fields.event_start.timestamp()
    );
    assert_eq!(
        cloned_event.event_end.unwrap().timestamp(),
        clone_fields.event_end.timestamp()
    );
    assert_eq!(cloned_event.cloned_from_event_id, Some(event.id));

    // Calculated from the original door time difference (-1 hour before event start)
    let door_time_from_opening = event
        .door_time
        .unwrap()
        .signed_duration_since(event.event_start.unwrap())
        .num_seconds();
    assert_eq!(
        cloned_event.door_time.unwrap().timestamp(),
        (clone_fields.event_start + Duration::seconds(door_time_from_opening)).timestamp()
    );

    // Additional fields have been copied over
    assert_eq!(cloned_event.additional_info, event.additional_info);
    assert_eq!(cloned_event.promo_image_url, event.promo_image_url);
    assert_eq!(cloned_event.cover_image_url, event.cover_image_url);
    assert_eq!(cloned_event.event_type, event.event_type);
    assert_eq!(cloned_event.age_limit, event.age_limit);
    assert_eq!(cloned_event.top_line_info, event.top_line_info);
    assert_eq!(cloned_event.video_url, event.video_url);

    // New slug since it's a new event
    assert_ne!(cloned_event.slug(connection).unwrap(), event.slug(connection).unwrap());

    // Ticket types
    let cloned_event_ticket_types = event.ticket_types(true, None, connection).unwrap();
    assert_eq!(cloned_event_ticket_types.len(), 3);
    // Names are cloned over
    let cloned_ticket_type = cloned_event_ticket_types
        .iter()
        .find(|tt| tt.name == ticket_type.name)
        .unwrap();
    let cloned_ticket_type2 = cloned_event_ticket_types
        .iter()
        .find(|tt| tt.name == ticket_type2.name)
        .unwrap();
    let cloned_child_ticket_type = cloned_event_ticket_types
        .iter()
        .find(|tt| tt.parent_id.is_some())
        .unwrap();

    // Child should belong to cloned_ticket_type since its original was the original child's parent
    assert_eq!(cloned_child_ticket_type.parent_id, Some(cloned_ticket_type.id));
    assert_eq!(cloned_child_ticket_type.description, child_ticket_type.description);
    assert_eq!(
        cloned_child_ticket_type.valid_ticket_count(connection).unwrap(),
        child_ticket_type.valid_ticket_count(connection).unwrap()
    );
    assert_eq!(
        cloned_child_ticket_type.limit_per_person,
        child_ticket_type.limit_per_person
    );
    assert_eq!(
        cloned_child_ticket_type.price_in_cents,
        child_ticket_type.price_in_cents
    );
    assert_eq!(cloned_child_ticket_type.visibility, child_ticket_type.visibility);

    assert_eq!(cloned_ticket_type.parent_id, None);
    assert_eq!(cloned_ticket_type.description, ticket_type.description);
    assert_eq!(
        cloned_ticket_type.valid_ticket_count(connection).unwrap(),
        ticket_type.valid_ticket_count(connection).unwrap()
    );
    assert_eq!(cloned_ticket_type.limit_per_person, ticket_type.limit_per_person);
    assert_eq!(cloned_ticket_type.price_in_cents, ticket_type.price_in_cents);
    assert_eq!(cloned_ticket_type.visibility, ticket_type.visibility);

    assert_eq!(cloned_ticket_type2.parent_id, None);
    assert_eq!(cloned_ticket_type2.description, ticket_type2.description);
    assert_eq!(
        cloned_ticket_type2.valid_ticket_count(connection).unwrap(),
        ticket_type2.valid_ticket_count(connection).unwrap()
    );
    assert_eq!(cloned_ticket_type2.limit_per_person, ticket_type2.limit_per_person);
    assert_eq!(cloned_ticket_type2.price_in_cents, ticket_type2.price_in_cents);
    assert_eq!(cloned_ticket_type2.visibility, ticket_type2.visibility);

    // Event artists
    let event_artists = EventArtist::find_all_from_event(event.id, connection).unwrap();
    let cloned_event_artists = EventArtist::find_all_from_event(cloned_event.id, connection).unwrap();
    assert_eq!(event_artists.len(), cloned_event_artists.len());
    for x in 0..event_artists.len() {
        assert_eq!(event_artists[x].event_id, event.id);
        assert_eq!(cloned_event_artists[x].event_id, cloned_event.id);
        assert_eq!(event_artists[x].artist.id, cloned_event_artists[x].artist.id);
        assert_eq!(event_artists[x].rank, cloned_event_artists[x].rank);
        assert_eq!(event_artists[x].set_time, cloned_event_artists[x].set_time);
        assert_eq!(event_artists[x].importance, cloned_event_artists[x].importance);
        assert_eq!(event_artists[x].stage_id, cloned_event_artists[x].stage_id);
    }

    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::EventCloned),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(cloned_event.id),
        Some(DomainEventTypes::EventCloned),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());
}

#[test]
fn slug() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let slug = Slug::primary_slug(event.id, Tables::Events, connection).unwrap();
    assert_eq!(event.slug(connection).unwrap(), slug.slug);
}

#[test]
fn mark_settled() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    assert!(event.settled_at.is_none());

    let event = event.mark_settled(connection).unwrap();
    assert!(event.settled_at.is_some());
}

#[test]
fn find_organization_users() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization_owner = project.create_user().finish();
    let organization_user = project.create_user().finish();
    let organization2_owner = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, connection)
        .unwrap();
    let organization = project
        .create_organization()
        .with_member(&organization_owner, Roles::OrgOwner)
        .with_member(&organization_user, Roles::OrgMember)
        .finish();
    let organization2 = project
        .create_organization()
        .with_member(&organization2_owner, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let event2 = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization2)
        .finish();
    let organization_users = Event::find_organization_users(event.id, connection).unwrap();
    assert!(organization_users.contains(&organization_owner));
    assert!(organization_users.contains(&organization_user));
    assert!(organization_users.contains(&admin));
    assert!(!organization_users.contains(&organization2_owner));
    assert!(!organization_users.contains(&user));

    let organization_users = Event::find_organization_users(event2.id, connection).unwrap();
    assert!(!organization_users.contains(&organization_owner));
    assert!(!organization_users.contains(&organization_user));
    assert!(organization_users.contains(&admin));
    assert!(organization_users.contains(&organization2_owner));
    assert!(!organization_users.contains(&user));
}

#[test]
fn get_all_events_with_transactions_between() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let organization2 = project.create_organization().finish();
    let organization_event = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let organization_event2 = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let organization_event3 = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let _no_orders_event = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let other_organization_event = project
        .create_event()
        .with_ticket_pricing()
        .with_organization(&organization2)
        .finish();

    for event in vec![&organization_event, &organization_event2, &other_organization_event] {
        project.create_order().for_event(&event).quantity(1).is_paid().finish();
    }

    // Order with two refunds
    let mut order = project
        .create_order()
        .for_event(&organization_event3)
        .quantity(2)
        .for_user(&user)
        .is_paid()
        .finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::paid_at.eq(Utc::now().naive_utc() + Duration::days(-6)))
        .execute(connection)
        .unwrap();
    let ticket_type = &organization_event3.ticket_types(true, None, connection).unwrap()[0];
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(tickets[0].id),
    }];
    let (refund, _) = order.refund(&refund_items, user.id, None, false, connection).unwrap();
    diesel::update(refunds::table.filter(refunds::id.eq(refund.id)))
        .set(refunds::created_at.eq(Utc::now().naive_utc() + Duration::days(6)))
        .execute(connection)
        .unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(tickets[1].id),
    }];
    let (refund2, _) = order.refund(&refund_items, user.id, None, false, connection).unwrap();
    diesel::update(refunds::table.filter(refunds::id.eq(refund2.id)))
        .set(refunds::created_at.eq(Utc::now().naive_utc() + Duration::days(8)))
        .execute(connection)
        .unwrap();

    // Organization events with sales
    let found_events = Event::get_all_events_with_transactions_between(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(5).finish(),
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![organization_event.clone(), organization_event2.clone()]
    );

    // Timeframe includes order so included in result set
    let found_events = Event::get_all_events_with_transactions_between(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().add_days(5).finish(),
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![
            organization_event.clone(),
            organization_event2.clone(),
            organization_event3.clone()
        ]
    );

    // Timeframe includes refund so included in result set
    let found_events = Event::get_all_events_with_transactions_between(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(7).finish(),
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![
            organization_event.clone(),
            organization_event2.clone(),
            organization_event3.clone()
        ]
    );

    // Timeframe includes both order and refunds, only one event returned in data
    let found_events = Event::get_all_events_with_transactions_between(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().add_days(9).finish(),
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![
            organization_event.clone(),
            organization_event2.clone(),
            organization_event3.clone()
        ]
    );

    // Other organization
    let found_events = Event::get_all_events_with_transactions_between(
        organization2.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(5).finish(),
        connection,
    )
    .unwrap();
    assert_eq!(found_events, vec![other_organization_event.clone()]);

    // Outside of window for sale
    let found_events = Event::get_all_events_with_transactions_between(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(-4).finish(),
        connection,
    )
    .unwrap();
    assert!(found_events.is_empty());
}

#[test]
fn default() {
    let event: NewEvent = Default::default();
    assert_eq!(event.status, NewEvent::default_status());
    assert_eq!(event.is_external, NewEvent::default_is_external());
}

#[test]
fn get_all_events_ending_between() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let user = project.create_user().finish();
    let published_event = project.create_event().with_organization(&organization).finish();
    let _other_organization_published_event = project.create_event().finish();
    let draft_event = project
        .create_event()
        .with_organization(&organization)
        .with_status(EventStatus::Draft)
        .finish();
    let deleted_event = project.create_event().with_organization(&organization).finish();
    deleted_event.delete(user.id, connection).unwrap();
    let published_event_ending_before_window = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-14).finish())
        .finish();
    let published_event_ending_after_window = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(dates::now().add_days(-10).finish())
        .with_event_end(dates::now().add_days(14).finish())
        .finish();
    let _published_external_event = project
        .create_event()
        .external()
        .with_organization(&organization)
        .external()
        .finish();

    // Get published events in window
    let found_events = Event::get_all_events_ending_between(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(5).finish(),
        EventStatus::Published,
        connection,
    )
    .unwrap();
    assert_eq!(found_events, vec![published_event.clone()]);

    // Increasing the window to include later event
    let found_events = Event::get_all_events_ending_between(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(15).finish(),
        EventStatus::Published,
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![published_event.clone(), published_event_ending_after_window.clone()]
    );

    // Increasing the window to include all published event
    let found_events = Event::get_all_events_ending_between(
        organization.id,
        dates::now().add_days(-15).finish(),
        dates::now().add_days(15).finish(),
        EventStatus::Published,
        connection,
    )
    .unwrap();
    assert_eq!(
        found_events,
        vec![
            published_event_ending_before_window.clone(),
            published_event.clone(),
            published_event_ending_after_window.clone()
        ]
    );

    // Looking at draft status
    let found_events = Event::get_all_events_ending_between(
        organization.id,
        dates::now().add_days(-15).finish(),
        dates::now().add_days(15).finish(),
        EventStatus::Draft,
        connection,
    )
    .unwrap();
    assert_eq!(found_events, vec![draft_event.clone()]);
}

#[test]
fn eligible_for_deletion() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    assert!(event.eligible_for_deletion(connection).unwrap());

    // In a cart so no longer eligible for deletion
    let user = project.create_user().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(!event.eligible_for_deletion(connection).unwrap());

    //Set the expires_at time for the cart to the past
    // Expire the order
    diesel::sql_query(
        r#"
        UPDATE orders
        SET expires_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(cart.id)
    .execute(connection)
    .unwrap();
    assert!(event.eligible_for_deletion(connection).unwrap());
    cart.clear_cart(user.id, connection).unwrap();

    // Cleared the cart which removed the last link to this event
    assert!(event.eligible_for_deletion(connection).unwrap());
}

#[test]
fn create_next_transfer_drip_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_days(7).finish())
        .with_event_end(dates::now().add_days(14).finish())
        .with_ticket_pricing()
        .finish();

    let next_drip_date = event.next_drip_date(Environment::Test).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    let domain_action = &DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()[0];
    assert_eq!(domain_action.main_table_id, Some(event.id));
    assert_eq!(domain_action.main_table, Some(Tables::Events));

    let payload: ProcessTransferDripPayload = serde_json::from_value(domain_action.payload.clone()).unwrap();
    assert_eq!(
        payload,
        ProcessTransferDripPayload {
            event_id: event.id,
            source_or_destination: SourceOrDestination::Destination
        }
    );
    // Drip day is 1 days from the event start
    let drip_in_days = event
        .event_start
        .unwrap()
        .signed_duration_since(next_drip_date)
        .num_days();
    assert_eq!(drip_in_days, 1);
    domain_action.set_done(connection).unwrap();

    // Drip day is 3 hours from the event start as event is 1 day away
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_days(1).add_minutes(-1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    let next_drip_date = event.next_drip_date(Environment::Test).unwrap();
    let drip_in_hours = event
        .event_start
        .unwrap()
        .signed_duration_since(next_drip_date)
        .num_hours();
    assert_eq!(drip_in_hours, 3);
    domain_action.set_done(connection).unwrap();

    // Hours before event (used for the 0 days remaining drip date) should return no future dates
    let parameters = EventEditableAttributes {
        event_start: Some(
            dates::now()
                .add_hours(TRANSFER_DRIP_NOTIFICATION_HOURS_PRIOR_TO_EVENT)
                .finish(),
        ),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    assert!(
        DomainAction::find_pending(Some(DomainActionTypes::ProcessTransferDrip), connection)
            .unwrap()
            .is_empty()
    );

    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_days(-1).finish()),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    assert!(
        DomainAction::find_pending(Some(DomainActionTypes::ProcessTransferDrip), connection)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn create_next_transfer_drip_action_staging() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_minutes(7).finish())
        .with_event_end(dates::now().add_minutes(14).finish())
        .with_ticket_pricing()
        .finish();

    let next_drip_date = event.next_drip_date(Environment::Staging).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Staging, connection)
        .unwrap();
    let domain_action = &DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()[0];
    assert_eq!(domain_action.main_table_id, Some(event.id));
    assert_eq!(domain_action.main_table, Some(Tables::Events));

    let payload: ProcessTransferDripPayload = serde_json::from_value(domain_action.payload.clone()).unwrap();
    assert_eq!(
        payload,
        ProcessTransferDripPayload {
            event_id: event.id,
            source_or_destination: SourceOrDestination::Destination
        }
    );
    // Drip day is 1 minutes from the event start
    let drip_in_minutes = event
        .event_start
        .unwrap()
        .signed_duration_since(next_drip_date)
        .num_minutes();
    assert_eq!(drip_in_minutes, 1);
    domain_action.set_done(connection).unwrap();

    // Drip day is 1 minute from the event start as event is 1 minute away
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Staging, connection)
        .unwrap();
    let next_drip_date = event.next_drip_date(Environment::Staging).unwrap();
    let drip_in_minutes = event
        .event_start
        .unwrap()
        .signed_duration_since(next_drip_date)
        .num_minutes();
    assert_eq!(drip_in_minutes, 0);
    domain_action.set_done(connection).unwrap();

    // 0 minutes remaining drip date should return no future dates
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(0).finish()),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Staging, connection)
        .unwrap();
    assert!(
        DomainAction::find_pending(Some(DomainActionTypes::ProcessTransferDrip), connection)
            .unwrap()
            .is_empty()
    );

    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(-1).finish()),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Staging, connection)
        .unwrap();
    assert!(
        DomainAction::find_pending(Some(DomainActionTypes::ProcessTransferDrip), connection)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn regenerate_drip_actions() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_days(14).finish())
        .finish();
    assert!(DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::RegenerateDripActions,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());

    event.regenerate_drip_actions(connection).unwrap();
    assert!(!DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::RegenerateDripActions,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());
}

#[test]
fn minutes_until_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_minutes(7).finish())
        .with_event_end(dates::now().add_minutes(14).finish())
        .with_ticket_pricing()
        .finish();
    assert_eq!(event.minutes_until_event(), Some(7));

    // 1 minute away
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(event.minutes_until_event(), Some(1));

    // Event already started
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_seconds(-1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(event.minutes_until_event(), Some(0));
}

#[test]
fn days_until_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_days(7).finish())
        .with_event_end(dates::now().add_days(14).finish())
        .with_ticket_pricing()
        .finish();
    assert_eq!(event.days_until_event(), Some(7));

    // 1 day away with some wiggle room
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_hours(23).add_minutes(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(event.days_until_event(), Some(1));

    // Event already started
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(-1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(event.days_until_event(), Some(0));
}

#[test]
fn next_drip_date() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_days(7).finish())
        .with_event_end(dates::now().add_days(14).finish())
        .with_ticket_pricing()
        .finish();

    // Event 7 days away, next drip day in -1 from event start
    assert_eq!(
        event.next_drip_date(Environment::Test),
        Some(event.event_start.unwrap() + Duration::days(-1))
    );

    // Event is 3 days away next event in -1 from event start
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_days(3).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.next_drip_date(Environment::Test),
        Some(event.event_start.unwrap() + Duration::days(-1))
    );

    // Event is tomorrow, next drip day is tomorrow
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_days(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.next_drip_date(Environment::Test),
        Some(event.event_start.unwrap() - Duration::hours(TRANSFER_DRIP_NOTIFICATION_HOURS_PRIOR_TO_EVENT))
    );

    // Event is today, next drip day given there's wiggle room
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_hours(23).add_minutes(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.next_drip_date(Environment::Test),
        Some(event.event_start.unwrap() - Duration::hours(TRANSFER_DRIP_NOTIFICATION_HOURS_PRIOR_TO_EVENT))
    );

    // Event is today, no next drip day
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_hours(20).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Test).is_none());

    // Event is now, no next drip
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Test).is_none());

    // Event has started no drip days
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_days(-1).finish()),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Test).is_none());
}

#[test]
fn next_drip_date_staging() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_event_start(dates::now().add_minutes(7).finish())
        .with_event_end(dates::now().add_minutes(14).finish())
        .with_ticket_pricing()
        .finish();

    // Event 7 minutes away, next drip day in -1 from event start
    assert_eq!(
        event.next_drip_date(Environment::Staging),
        Some(event.event_start.unwrap() + Duration::minutes(-1))
    );

    // Event is 3 minutes away next event in -1 from event start
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(3).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.next_drip_date(Environment::Staging),
        Some(event.event_start.unwrap() + Duration::minutes(-1))
    );

    // Event is in 1 minute, next drip day is in 1 minute
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(1).finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.next_drip_date(Environment::Staging),
        Some(event.event_start.unwrap())
    );

    // Event is today, no next drip day
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Staging).is_none());

    // Event is now, no next drip
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().finish()),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Staging).is_none());

    // Event has started no drips
    let parameters = EventEditableAttributes {
        event_start: Some(dates::now().add_minutes(-1).finish()),
        ..Default::default()
    };
    event.update(None, parameters, connection).unwrap();
    assert!(event.next_drip_date(Environment::Staging).is_none());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    assert_eq!(Event::find(event.id, connection).unwrap(), event.clone());
    event.clone().delete(user.id, connection).unwrap();
    assert!(Event::find(event.id, connection).is_err());
}

#[test]
fn delete() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::EventDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    assert!(event.clone().delete(user.id, connection).is_ok());
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::EventDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert!(Event::find(event.id, connection).is_err());

    // Can't delete as event has associated order
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(event.clone().delete(user.id, connection).is_err());
    assert!(Event::find(event.id, connection).is_ok());
}

#[test]
fn summary() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    assert!(event.summary(connection).is_ok());
    event.clone().delete(user.id, connection).unwrap();
    assert_eq!(
        event.summary(connection),
        DatabaseError::business_process_error("Unable to display summary, summary data not available for event",)
    );
}

#[test]
fn activity_summary() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project.create_event().with_ticket_pricing().finish();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(event.ticket_types(true, None, connection).unwrap()[0].id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event2)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&event2.ticket_types(true, None, connection).unwrap()[0])
        .with_discount_in_cents(Some(10))
        .finish();
    project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .for_user(&user3)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    assert_eq!(
        event.activity_summary(user.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }
    );
    assert_eq!(
        event.activity_summary(user2.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user2.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }
    );
    assert_eq!(
        event.activity_summary(user3.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user3.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }
    );
    assert_eq!(
        event2.activity_summary(user.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event2.id, user.id, None, connection).unwrap(),
            event: event2.for_display(connection).unwrap(),
        }
    );
    assert_eq!(
        event2.activity_summary(user2.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event2.id, user2.id, None, connection).unwrap(),
            event: event2.for_display(connection).unwrap(),
        }
    );
    assert_eq!(
        event2.activity_summary(user3.id, None, connection).unwrap(),
        ActivitySummary {
            activity_items: ActivityItem::load_for_event(event2.id, user3.id, None, connection).unwrap(),
            event: event2.for_display(connection).unwrap(),
        }
    );
}

#[test]
fn genres() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let artist = project.create_artist().with_name("Artist 1".to_string()).finish();
    let artist2 = project.create_artist().with_name("Artist 2".to_string()).finish();

    let event = project.create_event().finish();
    let event2 = project.create_event().finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist2)
        .finish();
    project
        .create_event_artist()
        .with_event(&event2)
        .with_artist(&artist2)
        .finish();

    // No genres set
    assert!(event.update_genres(Some(creator.id), connection).is_ok());
    assert!(event2.update_genres(Some(creator.id), connection).is_ok());

    assert!(artist.genres(connection).unwrap().is_empty());
    assert!(artist2.genres(connection).unwrap().is_empty());
    assert!(event.genres(connection).unwrap().is_empty());
    assert!(event2.genres(connection).unwrap().is_empty());

    artist
        .set_genres(
            &vec!["emo".to_string(), "test".to_string(), "Hard Rock".to_string()],
            None,
            connection,
        )
        .unwrap();
    assert!(event.update_genres(Some(creator.id), connection).is_ok());
    assert!(event2.update_genres(Some(creator.id), connection).is_ok());

    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert!(artist2.genres(connection).unwrap().is_empty());
    assert_eq!(
        event.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert!(event2.genres(connection).unwrap().is_empty());

    artist2
        .set_genres(&vec!["emo".to_string(), "happy".to_string()], None, connection)
        .unwrap();
    assert!(event.update_genres(Some(creator.id), connection).is_ok());
    assert!(event2.update_genres(Some(creator.id), connection).is_ok());

    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert_eq!(
        artist2.genres(connection).unwrap(),
        vec!["emo".to_string(), "happy".to_string()]
    );
    assert_eq!(
        event.genres(connection).unwrap(),
        vec![
            "emo".to_string(),
            "happy".to_string(),
            "hard-rock".to_string(),
            "test".to_string()
        ]
    );
    assert_eq!(
        event2.genres(connection).unwrap(),
        vec!["emo".to_string(), "happy".to_string()]
    );
}

#[test]
fn find_all_ticket_holders_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user_with_email = project.create_user().finish();
    let user_with_no_email = project.create_user().with_no_email().finish();
    let user_with_no_tel = project.create_user().with_no_phone().finish();
    let event = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_user(&user_with_email)
        .for_event(&event)
        .quantity(5)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&user_with_no_email)
        .for_event(&event)
        .quantity(2)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&user_with_no_tel)
        .for_event(&event)
        .quantity(2)
        .is_paid()
        .finish();

    let all_ticket_holders =
        Event::find_all_ticket_holders_count(event.id, connection, TicketHoldersCountType::All).unwrap();
    let no_email_ticket_holders =
        Event::find_all_ticket_holders_count(event.id, connection, TicketHoldersCountType::WithEmailAddress).unwrap();
    let no_tel_ticket_holders =
        Event::find_all_ticket_holders_count(event.id, connection, TicketHoldersCountType::WithPhoneNumber).unwrap();

    assert_eq!(all_ticket_holders, 3);
    assert_eq!(no_email_ticket_holders, 2);
    assert_eq!(no_tel_ticket_holders, 2);
}

#[test]
fn pending_transfers() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_user(&user)
        .for_event(&event)
        .quantity(2)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    let transfer2 = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    assert_eq!(event.pending_transfers(connection).unwrap().len(), 0);

    transfer.add_transfer_ticket(tickets[0].id, connection).unwrap();
    assert_equiv!(event.pending_transfers(connection).unwrap(), [transfer.clone()]);

    transfer2.add_transfer_ticket(tickets[1].id, connection).unwrap();
    assert_equiv!(event.pending_transfers(connection).unwrap(), [transfer, transfer2]);
}

#[test]
fn update_genres() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let artist = project.create_artist().finish();
    artist
        .set_genres(&vec!["emo".to_string(), "happy".to_string()], None, connection)
        .unwrap();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();

    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .quantity(1)
        .finish();

    assert!(event.genres(connection).unwrap().is_empty());
    assert!(user.genres(connection).unwrap().is_empty());
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::GenresUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    assert!(event.update_genres(Some(creator.id), connection).is_ok());
    assert_eq!(
        event.genres(connection).unwrap(),
        vec!["emo".to_string(), "happy".to_string()]
    );
    assert_eq!(
        user.genres(connection).unwrap(),
        vec!["emo".to_string(), "happy".to_string()]
    );
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::GenresUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(
        domain_events[0].event_data,
        Some(json!({ "genres": vec!["emo", "happy"] }))
    )
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.venue_id, Some(venue.id));
    assert_eq!(event.organization_id, organization.id);
    assert_eq!(event.id.to_string().is_empty(), false);
    assert!(event.slug_id.is_some());
    let slug = Slug::primary_slug(event.id, Tables::Events, connection).unwrap();
    assert_eq!(slug.main_table_id, event.id);
    assert_eq!(slug.main_table, Tables::Events);
    assert_eq!(slug.slug_type, SlugTypes::Event);

    // Create without an event start does not auto set event_end and door_time
    let event = Event::create(
        "name",
        EventStatus::Draft,
        organization.id,
        None,
        None,
        None,
        None,
        None,
    )
    .commit(None, connection)
    .unwrap();
    assert_eq!(event.event_start, None);
    assert_eq!(event.door_time, None);
    assert_eq!(event.event_end, None);

    // Create with an event start does auto set event_end and door_time
    let event_start = dates::now().add_days(3).finish();
    let expected_event_end = NaiveDateTime::from(event_start + Duration::days(1));
    let expected_door_time = NaiveDateTime::from(event_start - Duration::hours(1));
    let event = Event::create(
        "name",
        EventStatus::Draft,
        organization.id,
        None,
        Some(event_start),
        None,
        None,
        None,
    )
    .commit(None, connection)
    .unwrap();
    assert_eq!(event.event_start.map(|t| t.timestamp()), Some(event_start.timestamp()));
    assert_eq!(
        event.door_time.map(|t| t.timestamp()),
        Some(expected_door_time.timestamp())
    );
    assert_eq!(
        event.event_end.map(|t| t.timestamp()),
        Some(expected_event_end.timestamp())
    );
}

#[test]
fn is_published() {
    let project = TestProject::new();
    let mut event = project.create_event().finish();
    event.publish_date = None;
    assert!(!event.is_published());

    event.publish_date = Some(dates::now().add_minutes(1).finish());
    assert!(!event.is_published());

    event.publish_date = Some(dates::now().add_minutes(-1).finish());
    assert!(event.is_published());
}

#[test]
fn update() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    //Edit event
    let parameters = EventEditableAttributes {
        private_access_code: Some(Some("PRIVATE".to_string())),
        door_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11)),
        ..Default::default()
    };
    let event = event.update(None, parameters, project.get_connection()).unwrap();
    assert_eq!(
        event.door_time,
        Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11))
    );
    assert_eq!(event.private_access_code, Some("private".to_string()));
}

#[test]
fn update_changing_event_start() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let old_event_start = dates::now().add_days(3).finish();
    let new_event_start = dates::now().add_days(7).finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(old_event_start)
        .with_event_end(dates::now().add_days(14).finish())
        .finish();

    let event = event.publish(None, connection).unwrap();
    let domain_action = &DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::RegenerateDripActions,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()[0];
    // Remove existing domain action created via publishing
    domain_action.set_done(connection).unwrap();

    let parameters = EventEditableAttributes {
        event_start: Some(new_event_start),
        ..Default::default()
    };
    let event = event.update(None, parameters, connection).unwrap();
    assert_eq!(
        event.event_start.unwrap().round_subsecs(4),
        new_event_start.round_subsecs(4)
    );
    // New domain action is added as a result of the start time changes
    assert_eq!(
        DomainAction::find_by_resource(
            Some(Tables::Events),
            Some(event.id),
            DomainActionTypes::RegenerateDripActions,
            DomainActionStatus::Pending,
            connection,
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn guest_list() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project
        .create_user()
        .with_first_name("Alex")
        .with_last_name("Test")
        .finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let user5 = project.create_user().finish();
    let user6 = project.create_user().finish();

    // 1 normal order, 2 orders made on behalf of users by box office user 2
    let normal_order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user3)
        .quantity(1)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user4)
        .quantity(1)
        .is_paid()
        .finish();

    let guest_list = event
        .guest_list(Some("Alex".to_string()), &None, None, connection)
        .unwrap()
        .0;
    let guest_list_length = guest_list.len().clone();
    let first_ticket = guest_list.first().unwrap().ticket.clone();
    assert_eq!(1, guest_list_length);
    assert_eq!(first_ticket.user_id.clone(), Some(user.id));

    let guest_list = event
        .guest_list(Some("Test".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    // Partial match last name, first name
    let guest_list = event
        .guest_list(Some("tes al".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    // With commas that are ignored
    let guest_list = event
        .guest_list(Some("test, al".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    // Partial match first name, full last name
    let guest_list = event
        .guest_list(Some("Al Test".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    let guest_list = event
        .guest_list(Some("ex T".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    // Update ticket for user.id to override name
    let ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .pop()
        .unwrap();
    ticket
        .update(
            UpdateTicketInstanceAttributes {
                first_name_override: Some(Some("First".to_string())),
                last_name_override: Some(Some("Last".to_string())),
            },
            user.id,
            &project.connection,
        )
        .unwrap();

    let guest_list = event
        .guest_list(Some("Alex".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert!(guest_list.is_empty());

    let guest_list = event
        .guest_list(Some("Test".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert!(guest_list.is_empty());

    let guest_list = event
        .guest_list(Some("ex T".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert!(guest_list.is_empty());

    let guest_list = event
        .guest_list(Some("First".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    let guest_list = event
        .guest_list(Some("Last".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    let guest_list = event
        .guest_list(Some("st la".to_string()), &None, None, connection)
        .unwrap()
        .0;
    assert_eq!(1, guest_list.len());
    assert_eq!(guest_list.first().unwrap().ticket.user_id, Some(user.id));

    let guest_list = event.guest_list(None, &None, None, connection).unwrap().0;
    assert_eq!(3, guest_list.len());
    let guest_ids = guest_list
        .iter()
        .map(|r| r.ticket.user_id)
        .collect::<Vec<Option<Uuid>>>();
    assert!(guest_ids.contains(&Some(user.id)));
    assert!(!guest_ids.contains(&Some(user2.id)));
    assert!(guest_ids.contains(&Some(user3.id)));
    assert!(guest_ids.contains(&Some(user4.id)));

    let guest_list_user_record = guest_list.iter().find(|gl| gl.ticket.user_id == Some(user.id)).unwrap();
    assert_eq!(guest_list_user_record.ticket.first_name, Some("First".to_string()));
    assert_eq!(guest_list_user_record.ticket.last_name, Some("Last".to_string()));
    let guest_list_user_record = guest_list
        .iter()
        .find(|gl| gl.ticket.user_id == Some(user3.id))
        .unwrap();
    assert_eq!(guest_list_user_record.ticket.first_name, user3.first_name);
    assert_eq!(guest_list_user_record.ticket.last_name, user3.last_name);

    // User 2 (the box office user) purchases a ticket for themselves
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .quantity(1)
        .is_paid()
        .finish();
    let guest_list = event.guest_list(None, &None, None, connection).unwrap().0;
    assert_eq!(4, guest_list.len());
    let guest_ids = guest_list
        .iter()
        .map(|r| r.ticket.user_id)
        .collect::<Vec<Option<Uuid>>>();
    assert!(guest_ids.contains(&Some(user.id)));
    assert!(guest_ids.contains(&Some(user2.id)));
    assert!(guest_ids.contains(&Some(user3.id)));
    assert!(guest_ids.contains(&Some(user4.id)));

    //Check the updated_at filter from 100 seconds ago
    let hundred_seconds_ago = Utc::now().naive_utc() + Duration::seconds(-100);
    let guest_list = event
        .guest_list(None, &Some(hundred_seconds_ago), None, connection)
        .unwrap()
        .0;
    assert_eq!(4, guest_list.len());

    //Check the updated_at filter in 100 seconds time
    let hundred_seconds_later = Utc::now().naive_utc() + Duration::seconds(100);
    let guest_list = event
        .guest_list(None, &Some(hundred_seconds_later), None, connection)
        .unwrap()
        .0;
    assert_eq!(0, guest_list.len());

    // Transfer tickets for users 1 and 4 changing guest list to show the new users
    for (from_user, new_user) in vec![(&user, &user5), (&user4, &user6)] {
        let ticket_ids: Vec<Uuid> = TicketInstance::find_for_user(from_user.id, connection)
            .unwrap()
            .into_iter()
            .map(|ti| ti.id)
            .collect();
        TicketInstance::direct_transfer(
            &from_user,
            &ticket_ids,
            "nowhere",
            TransferMessageType::Email,
            new_user.id,
            connection,
        )
        .unwrap();
    }
    let guest_list = event.guest_list(None, &None, None, connection).unwrap().0;
    assert_eq!(4, guest_list.len());
    let guest_ids = guest_list
        .iter()
        .map(|r| r.ticket.user_id)
        .collect::<Vec<Option<Uuid>>>();
    assert!(!guest_ids.contains(&Some(user.id)));
    assert!(guest_ids.contains(&Some(user2.id)));
    assert!(guest_ids.contains(&Some(user3.id)));
    assert!(!guest_ids.contains(&Some(user4.id)));
    assert!(guest_ids.contains(&Some(user5.id)));
    assert!(guest_ids.contains(&Some(user6.id)));

    let guest_list_user_record = guest_list
        .iter()
        .find(|gl| gl.ticket.user_id == Some(user5.id))
        .unwrap();
    assert_eq!(guest_list_user_record.ticket.first_name, user5.first_name);
    assert_eq!(guest_list_user_record.ticket.last_name, user5.last_name);

    //Test the pagination
    let paging = Paging::new(0, 3);
    let guest_list = event.guest_list(None, &None, Some(&paging), connection).unwrap();
    assert_eq!(3, guest_list.0.len());
    assert_eq!(4, guest_list.1);
    let paging = Paging::new(1, 3);
    let guest_list = event.guest_list(None, &None, Some(&paging), connection).unwrap();
    assert_eq!(4, guest_list.1);
    assert_eq!(1, guest_list.0.len());

    //Search by an order id
    let order_id = normal_order.id.to_string();
    let order_id = order_id[&order_id.len() - 8..].to_string();
    let guest_list = event
        .guest_list(Some(order_id.clone()), &None, None, connection)
        .unwrap();
    let guest_list_item = guest_list.0.first().unwrap();
    assert_eq!(1, guest_list.1);
    assert_eq!(normal_order.id, guest_list_item.ticket.order_id);
    //Search by ticket_instance id
    let ticket_id = first_ticket.id.to_string();
    let ticket_id = ticket_id[&ticket_id.len() - 8..].to_string();
    let guest_list = event
        .guest_list(Some(ticket_id.clone()), &None, None, connection)
        .unwrap();
    let guest_list_item = guest_list.0.first().unwrap();
    assert_eq!(1, guest_list.1);
    assert_eq!(first_ticket.id, guest_list_item.ticket.id);
}

#[test]
fn update_fails_to_move_event_into_past() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();

    // Can move into the past when no sales have occurred
    assert!(event
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_minutes(-2).finish()),
                event_end: Some(dates::now().add_minutes(-1).finish()),
                ..Default::default()
            },
            connection
        )
        .is_ok());

    // Can move back to the future
    assert!(event
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_minutes(1).finish()),
                event_end: Some(dates::now().add_minutes(2).finish()),
                ..Default::default()
            },
            connection
        )
        .is_ok());

    // Once a sale has been made can no longer move to the past or out of the past
    let event = event
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_minutes(-2).finish()),
                event_end: Some(dates::now().add_minutes(-1).finish()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    project.create_order().for_event(&event).finish();

    // Attempt to move to alter the dates
    let result = event.update(
        None,
        EventEditableAttributes {
            event_start: Some(dates::now().add_minutes(-3).finish()),
            event_end: Some(dates::now().add_minutes(-2).finish()),
            ..Default::default()
        },
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("event.event_start"));
                assert_eq!(errors["event.event_start"].len(), 1);
                assert_eq!(errors["event.event_start"][0].code, "cannot_move_event_dates_in_past");
                assert_eq!(
                    &errors["event.event_start"][0].message.clone().unwrap().into_owned(),
                    "Event with sales cannot move to past date."
                );

                assert!(errors.contains_key("event.event_end"));
                assert_eq!(errors["event.event_end"].len(), 1);
                assert_eq!(errors["event.event_end"][0].code, "cannot_move_event_dates_in_past");
                assert_eq!(
                    &errors["event.event_end"][0].message.clone().unwrap().into_owned(),
                    "Event with sales cannot move to past date."
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // If the dates sent are the same as those now, it does not error instead ignoring those fields
    let event = event
        .update(
            None,
            EventEditableAttributes {
                event_start: event.event_start,
                event_end: event.event_end,
                name: Some("New Name".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    assert_eq!("New Name".to_string(), event.name);

    // Can also use future dates still for event moving it out from the past
    assert!(event
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_days(1).finish()),
                event_end: Some(dates::now().add_days(2).finish()),
                ..Default::default()
            },
            connection
        )
        .is_ok());
}

#[test]
fn publish_fails_without_required_fields() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut event = project.create_event().with_status(EventStatus::Draft).finish();
    event.promo_image_url = None;
    let result = event.publish(None, connection);
    match result {
        Ok(_) => {
            panic!("Expected error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("venue_id"));
                assert_eq!(errors["venue_id"].len(), 1);
                assert_eq!(errors["venue_id"][0].code, "required");
                assert_eq!(
                    &errors["venue_id"][0].message.clone().unwrap().into_owned(),
                    "Event can't be published without a venue"
                );

                assert!(errors.contains_key("promo_image_url"));
                assert_eq!(errors["promo_image_url"].len(), 1);
                assert_eq!(errors["promo_image_url"][0].code, "required");
                assert_eq!(
                    &errors["promo_image_url"][0].message.clone().unwrap().into_owned(),
                    "Event can't be published without a promo image"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn publish() {
    //create event
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let event = event.publish(None, connection).unwrap();

    assert_eq!(event.status, EventStatus::Published);
    assert!(event.publish_date.is_some());

    assert_eq!(
        DomainAction::find_by_resource(
            Some(Tables::Events),
            Some(event.id),
            DomainActionTypes::RegenerateDripActions,
            DomainActionStatus::Pending,
            connection,
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn clear_pending_drip_actions() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_venue(&venue)
        .with_status(EventStatus::Draft)
        .finish();

    // Create next drip action
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    assert!(!DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());

    // Method removes it
    event.clear_pending_drip_actions(connection).unwrap();
    assert!(DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());
}

#[test]
fn publish_in_future() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };
    let event = event.update(None, parameters, project.get_connection()).unwrap();

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);
    assert_eq!(
        event.publish_date,
        Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))
    );
}

#[test]
fn publish_change_publish_date() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let now = Utc::now().naive_utc();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };
    let event = event.update(None, parameters, project.get_connection()).unwrap();

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2041, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };

    let event = event.update(None, parameters, project.get_connection()).unwrap();

    assert_eq!(
        event.publish_date,
        Some(NaiveDate::from_ymd(2041, 7, 8).and_hms(4, 10, 11))
    );

    let parameters = EventEditableAttributes {
        publish_date: Some(None),
        ..Default::default()
    };

    let event = event.update(None, parameters, project.get_connection()).unwrap();

    assert!(event.publish_date.unwrap() > now);

    assert!(event.publish_date.unwrap() < Utc::now().naive_utc());
}

#[test]
fn unpublish() {
    //create event
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let event = event.publish(None, connection).unwrap();
    event
        .create_next_transfer_drip_action(Environment::Test, connection)
        .unwrap();
    assert_eq!(event.status, EventStatus::Published);
    assert!(event.publish_date.is_some());
    assert!(!DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());

    let event = event.unpublish(None, connection).unwrap();
    assert_eq!(event.status, EventStatus::Draft);
    assert!(event.publish_date.is_none());
    assert!(DomainAction::find_by_resource(
        Some(Tables::Events),
        Some(event.id),
        DomainActionTypes::ProcessTransferDrip,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap()
    .is_empty());
}

#[test]
fn cannot_unpublish_unpublished_event() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);
    assert!(event.unpublish(None, project.get_connection()).is_err());
}

#[test]
fn cancel() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    let event = event.cancel(None, &project.get_connection()).unwrap();
    assert!(!event.cancelled_at.is_none());
}

#[test]
fn get_sales_by_date_range() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    // user purchases 10 tickets
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        1700,
        connection,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    // Other user does not checkout
    let mut cart2 = Order::find_or_create_cart(&user2, connection).unwrap();
    cart2
        .update_quantities(
            user2.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 5,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    // A day ago to today
    let start_utc = Utc::now().naive_utc().date() - Duration::days(1);
    let end_utc = Utc::now().naive_utc().date();
    let results = event.get_sales_by_date_range(start_utc, end_utc, connection).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results,
        vec![
            DayStats {
                date: start_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            },
            DayStats {
                date: end_utc,
                revenue_in_cents: 1500,
                ticket_sales: 10,
            }
        ]
    );

    // Just today
    let start_utc = Utc::now().naive_utc().date();
    let end_utc = Utc::now().naive_utc().date();
    let results = event.get_sales_by_date_range(start_utc, end_utc, connection).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results,
        vec![DayStats {
            date: start_utc,
            revenue_in_cents: 1500,
            ticket_sales: 10,
        }]
    );
    // Two days ago to yesterday
    let start_utc = Utc::now().naive_utc().date() - Duration::days(2);
    let end_utc = Utc::now().naive_utc().date() - Duration::days(1);
    let results = event.get_sales_by_date_range(start_utc, end_utc, connection).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results,
        vec![
            DayStats {
                date: start_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            },
            DayStats {
                date: end_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            }
        ]
    );

    // Error as start date is not earlier than end date
    let results = event.get_sales_by_date_range(end_utc, start_utc, connection);
    assert!(results.is_err());
    assert_eq!(
        "Sales data start date must come before end date",
        results.unwrap_err().cause.unwrap().to_string()
    );
}

#[test]
fn find_incl_org_venue_fees() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let result = Event::find_incl_org_venue_fees(event.id, connection).unwrap();
    assert_eq!(result, (event, organization, Some(venue), fee_schedule));
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().with_name("Event1".into()).finish();
    let event2 = project.create_event().with_name("Event2".into()).finish();

    assert_eq!(
        vec![event.clone()],
        Event::find_by_ids(vec![event.id], connection).unwrap()
    );
    assert_eq!(
        vec![event2.clone()],
        Event::find_by_ids(vec![event2.id], connection).unwrap()
    );
    assert_equiv!(
        Event::find_by_ids(vec![event.id, event2.id], connection).unwrap(),
        vec![event.clone(), event2.clone()]
    );

    event.clone().delete(user.id, connection).unwrap();
    assert_equiv!(
        Event::find_by_ids(vec![event.id, event2.id], connection).unwrap(),
        vec![event2]
    );
}

#[test]
fn find_by_order_item_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_fees()
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let mut cart2 = Order::find_or_create_cart(&user, connection).unwrap();
    cart2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    let items = cart.items(&connection).unwrap();
    let items2 = cart2.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let order_item2 = items2
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();

    // Ticket belonging to only first event / organization
    let events = Event::find_by_order_item_ids(&vec![order_item.id], connection).unwrap();
    assert_eq!(events, vec![event.clone()]);

    // Ticket belonging to only second event / organization
    let events = Event::find_by_order_item_ids(&vec![order_item2.id], connection).unwrap();
    assert_eq!(events, vec![event2.clone()]);

    // Ticket belonging to both events
    let events = Event::find_by_order_item_ids(&vec![order_item.id, order_item2.id], connection).unwrap();
    assert_equiv!(events, vec![event, event2]);
}

#[test]
fn find_individuals() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_venue(&venue)
        .finish();

    let parameters = EventEditableAttributes {
        door_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11)),
        ..Default::default()
    };
    let event = event.update(None, parameters, project.get_connection()).unwrap();

    let found_event = Event::find(event.id, project.get_connection()).unwrap();
    assert_eq!(found_event, event);

    //find event via venue
    let found_event_via_venue =
        Event::find_all_active_events_for_venue(&event.venue_id.unwrap(), project.get_connection()).unwrap();
    assert_eq!(found_event_via_venue[0], event);
}

#[test]
fn find_all_events_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let past_event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();
    let current_event = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(NaiveDateTime::parse_from_str("2018-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2814-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();
    let future_event = project
        .create_event()
        .with_name("Event3".into())
        .with_event_start(NaiveDateTime::parse_from_str("2918-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2919-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();

    // Past events
    let events = Event::find_all_events_for_organization(
        organization.id,
        Some(PastOrUpcoming::Past),
        None,
        false,
        0,
        100,
        connection,
    )
    .unwrap();
    assert_eq!(events.data, vec![past_event.summary(connection).unwrap()]);
    assert_eq!(events.paging.total, 1);

    // Upcoming (current, future) events
    let events = Event::find_all_events_for_organization(
        organization.id,
        Some(PastOrUpcoming::Upcoming),
        None,
        false,
        0,
        100,
        connection,
    )
    .unwrap();
    assert_eq!(
        events.data,
        vec![
            current_event.summary(connection).unwrap(),
            future_event.summary(connection).unwrap()
        ]
    );
    assert_eq!(events.paging.total, 2);

    // No filter on past or upcoming returns all events
    let events =
        Event::find_all_events_for_organization(organization.id, None, None, false, 0, 100, connection).unwrap();
    assert_equiv!(
        events.data,
        vec![
            past_event.summary(connection).unwrap(),
            current_event.summary(connection).unwrap(),
            future_event.summary(connection).unwrap(),
        ]
    );
    assert_eq!(events.paging.total, 3);
}

#[test]
fn search_finds_events_in_both_matching_country_and_state() {
    // Search for MA == Morocco (Country) and Massachusetts (US State) returns events for both
    let project = TestProject::new();
    let connection = project.get_connection();
    let country_lookup = CountryLookup::new().unwrap();
    let paging: &Paging = &Paging {
        page: 0,
        limit: 10,
        sort: "".to_string(),
        dir: SortingDir::Asc,
        total: 0,
        tags: HashMap::new(),
    };
    let venue = project.create_venue().finish();
    let venue = venue
        .update(
            VenueEditableAttributes {
                state: Some("MA".to_string()),
                country: Some("US".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let venue2 = project.create_venue().finish();
    let venue2 = venue2
        .update(
            VenueEditableAttributes {
                country: Some("MA".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let event = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .finish();
    let event2 = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .finish();

    // Search by code
    let all_found_events = Event::search(
        Some("MA".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);

    // Search by names
    let all_found_events = Event::search(
        Some("Massachusetts".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_found_events.0[0], event);
    let all_found_events = Event::search(
        Some("Morocco".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_found_events.0[0], event2);

    // Update to use full names
    venue
        .update(
            VenueEditableAttributes {
                state: Some("Massachusetts".to_string()),
                country: Some("United States".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    venue2
        .update(
            VenueEditableAttributes {
                country: Some("Morocco".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    // Search by country code
    let all_found_events = Event::search(
        Some("MA".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);

    // Search by names
    let all_found_events = Event::search(
        Some("Massachusetts".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_found_events.0[0], event);
    let all_found_events = Event::search(
        Some("Morocco".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_found_events.0[0], event2);
}

#[test]
fn search() {
    //create event
    let project = TestProject::new();
    let country_lookup = CountryLookup::new().unwrap();
    let creator = project.create_user().finish();
    let connection = project.get_connection();
    let region1 = project.create_region().finish();
    let region2 = project.create_region().finish();
    let city = "Dangerville city".to_string();
    let state = "Alaska".to_string();
    let venue1 = project
        .create_venue()
        .with_name("Venue1".into())
        .with_region(&region1)
        .finish();
    let venue1 = venue1
        .update(
            VenueEditableAttributes {
                city: Some(city.clone()),
                state: Some(state.clone()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let venue2 = project
        .create_venue()
        .with_name("Venue2".into())
        .with_region(&region2)
        .finish();
    let venue2 = venue2
        .update(
            VenueEditableAttributes {
                country: Some("IE".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let artist1 = project.create_artist().with_name("Artist1".into()).finish();
    let artist2 = project.create_artist().with_name("Artist2".into()).finish();
    let organization_owner = project.create_user().finish();
    let organization_user = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, connection)
        .unwrap();
    let organization = project
        .create_organization()
        .with_member(&organization_owner, Roles::OrgOwner)
        .with_member(&organization_user, Roles::OrgMember)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_name("OldEvent".into())
        .with_organization(&organization)
        .with_venue(&venue1)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .finish();

    event.add_artist(None, artist1.id, connection).unwrap();
    event.add_artist(None, artist2.id, connection).unwrap();

    //find more than one event
    let event2 = project
        .create_event()
        .with_status(EventStatus::Closed)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_tickets()
        .with_ticket_pricing()
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    event2.add_artist(None, artist1.id, connection).unwrap();

    let event3 = project
        .create_event()
        .with_name("NewEvent2".into())
        .with_status(EventStatus::Offline)
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event draft, not returned except for organization user or owner
    let event4 = project
        .create_event()
        .with_name("NewEventDraft".into())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event draft belonging to other organization
    let event5 = project
        .create_event()
        .with_name("NewEventDraft2".into())
        .with_status(EventStatus::Draft)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event with publish date in the future, not returned except for organization user or owner
    let event6 = project
        .create_event()
        .with_name("NewEventFuturePublish".into())
        .with_status(EventStatus::Published)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2999, 7, 8).and_hms(9, 10, 11))
        .finish();

    artist1
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    artist2
        .set_genres(&vec!["emo".to_string(), "rock".to_string()], None, connection)
        .unwrap();

    assert!(event.update_genres(Some(creator.id), connection).is_ok());
    assert!(event2.update_genres(Some(creator.id), connection).is_ok());

    let all_events = vec![event, event2, event3];
    let mut all_events_for_organization = all_events.clone();
    all_events_for_organization.push(event4);
    let mut all_events_for_admin = all_events_for_organization.clone();
    all_events_for_admin.push(event5);
    all_events_for_admin.push(event6);

    let paging: &Paging = &Paging {
        page: 0,
        limit: 10,
        sort: "".to_string(),
        dir: SortingDir::Asc,
        total: 0,
        tags: HashMap::new(),
    };
    // All events unauthorized user
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events.0);

    // All events organization owner
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(organization_owner),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events_for_organization, all_found_events.0);

    // All events organization user
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(organization_user),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events_for_organization, all_found_events.0);

    // All events normal user not part of event organization
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(user.clone()),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events.0);

    // All events for admin
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(admin),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events_for_admin, all_found_events.0);

    // No name specified
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events.0);

    // Limited by publicly accessible and specific to an organization
    let all_found_events = Event::search(
        None,
        None,
        Some(organization.id),
        None,
        None,
        None,
        None,
        Some(vec![EventStatus::Published]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Limited by just Published and Offline events
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(vec![EventStatus::Published, EventStatus::Offline]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[0], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    // Limited by just Closed events
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(vec![EventStatus::Closed]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[1], all_found_events.0[0]);

    // Event name search
    let all_found_events = Event::search(
        Some("New".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[1], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    // Venue name search
    let all_found_events = Event::search(
        Some("Venue1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Venue id  search
    let all_found_events = Event::search(
        None,
        None,
        None,
        Some(vec![venue1.id]),
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Artist name search for artist in both events
    let all_found_events = Event::search(
        Some("Artist1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[0], all_found_events.0[0]);
    assert_eq!(all_events[1], all_found_events.0[1]);

    // Artist name search for artist at only one event
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Match names Venue2 and Artist2 returning all events
    let all_found_events = Event::search(
        Some("2".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events.0);

    // Match events belonging to given region
    let all_found_events = Event::search(
        None,
        Some(region1.id.into()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Match events belonging to other region
    let all_found_events = Event::search(
        None,
        Some(region2.id.into()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[1], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    // Combination of query and region resulting in no records
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        Some(region2.id.into()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 0);

    // Combination of query and region resulting in records
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        Some(region1.id.into()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[1], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Genre search showing two events tagged
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        Some(vec!["Emo".to_string()]),
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[0], all_found_events.0[0]);
    assert_eq!(all_events[1], all_found_events.0[1]);

    // Genre requiring both genres
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        Some(vec!["Rock".to_string(), "Emo".to_string()]),
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Genre search finding none
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        Some(vec!["Happy".to_string()]),
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 0);

    // City search
    let all_found_events = Event::search(
        Some(city.clone()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // State search
    let all_found_events = Event::search(
        Some("alaskA".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // State abbreviation search
    let all_found_events = Event::search(
        Some("AK".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // city state search
    let all_found_events = Event::search(
        Some("dangerville city, alaskA".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // city state abbreviation search
    let all_found_events = Event::search(
        Some("dangerville city, aK".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // city state abbreviation search
    let all_found_events = Event::search(
        Some("dangerville city AK".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Update state to New Hampshire to confirm space splitting properly finds state for abbreviation swap
    venue1
        .update(
            VenueEditableAttributes {
                state: Some("NH".to_string()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let all_found_events = Event::search(
        Some("dangerville city NeW HampshiRe".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    let all_found_events = Event::search(
        Some("dangerville city Nh".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Extra spaces
    let all_found_events = Event::search(
        Some("dangerville      city         Nh".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // City, State, Country code
    let all_found_events = Event::search(
        Some("Dangerville City NH US".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Invalid city, State, Country code
    let all_found_events = Event::search(
        Some("Dangerville Circle NH US".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 0);

    // Name
    let all_found_events = Event::search(
        Some("Ireland".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[1], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    // Country code
    let all_found_events = Event::search(
        Some("IE".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 2);
    assert_eq!(all_events[1], all_found_events.0[0]);
    assert_eq!(all_events[2], all_found_events.0[1]);

    // New Hampshire does not work as IE Country code
    let all_found_events = Event::search(
        Some("New Hampshire, IE".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 0);

    // New Hampshire does work alone as logic defaults to US if no country provided for search and none on file
    let all_found_events = Event::search(
        Some("New Hampshire".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(user.clone()),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);

    // User uses US country code
    let all_found_events = Event::search(
        Some("New Hampshire, US".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(user.clone()),
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Logged out behavior is still normal defaulting to US
    let all_found_events = Event::search(
        Some("New Hampshire".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 1);
    assert_eq!(all_events[0], all_found_events.0[0]);

    // Invalid state, Country code
    let all_found_events = Event::search(
        Some("NL IE".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(all_found_events.0.len(), 0);
}

#[test]
fn filter_events_by_event_type() {
    //create event
    let project = TestProject::new();
    let connection = project.get_connection();
    let country_lookup = CountryLookup::new().unwrap();
    let region1 = project.create_region().finish();
    let venue1 = project
        .create_venue()
        .with_name("Venue1".into())
        .with_region(&region1)
        .finish();

    let artist1 = project.create_artist().with_name("Artist1".into()).finish();
    let organization_owner = project.create_user().finish();
    let organization_user = project.create_user().finish();
    let _admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, connection)
        .unwrap();
    let organization = project
        .create_organization()
        .with_member(&organization_owner, Roles::OrgOwner)
        .with_member(&organization_user, Roles::OrgMember)
        .finish();
    let event_music = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_name("MusicEvent".into())
        .with_organization(&organization)
        .with_venue(&venue1)
        .with_event_start(NaiveDate::from_ymd(2030, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2030, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_type(EventTypes::Music)
        .finish();

    event_music.add_artist(None, artist1.id, connection).unwrap();

    //find more than one event
    let event_art = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_name("ArtEvent".into())
        .with_organization(&organization)
        .with_venue(&venue1)
        .with_event_start(NaiveDate::from_ymd(2030, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2030, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_type(EventTypes::Art)
        .finish();

    event_art.add_artist(None, artist1.id, connection).unwrap();

    let paging: &Paging = &Paging {
        page: 0,
        limit: 10,
        sort: "".to_string(),
        dir: SortingDir::Asc,
        total: 0,
        tags: HashMap::new(),
    };
    // All events unauthorized user
    let all_music_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Upcoming,
        Some(EventTypes::Music),
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(1, all_music_events.1);
    assert_eq!("MusicEvent".to_string(), all_music_events.0[0].name);

    // All events organization owner
    let all_art_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Upcoming,
        Some(EventTypes::Art),
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();
    assert_eq!(1, all_art_events.1);
    assert_eq!("ArtEvent".to_string(), all_art_events.0[0].name);
}

#[test]
fn current_ticket_pricing_range() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_sales_starting(dates::now().add_hours(-2).finish())
        .with_sales_ending(dates::now().add_hours(-1).finish())
        .with_tickets()
        .with_ticket_type_count(2)
        .finish();

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event.ticket_types(true, None, connection).unwrap()[1];

    // No current pricing set
    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(false, connection).unwrap();
    assert_eq!(None, min_ticket_price);
    assert_eq!(None, max_ticket_price);

    // Future pricing
    ticket_type
        .add_ticket_pricing(
            "Pricing1".into(),
            NaiveDate::from_ymd(2055, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            3000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(false, connection).unwrap();
    assert_eq!(None, min_ticket_price);
    assert_eq!(None, max_ticket_price);

    // Current pricing
    ticket_type
        .add_ticket_pricing(
            "Pricing2".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            8000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(false, connection).unwrap();
    assert_eq!(Some(8000), min_ticket_price);
    assert_eq!(Some(8000), max_ticket_price);

    ticket_type2
        .add_ticket_pricing(
            "Pricing2".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            20000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(false, connection).unwrap();
    assert_eq!(Some(8000), min_ticket_price);
    assert_eq!(Some(20000), max_ticket_price);

    // Box office pricing, present
    ticket_type
        .add_ticket_pricing(
            "Box office1".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            None,
            connection,
        )
        .unwrap();

    // Box office pricing, date in future so won't activate
    ticket_type2
        .add_ticket_pricing(
            "Box office2".into(),
            NaiveDate::from_ymd(2055, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            1000,
            true,
            None,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(true, connection).unwrap();
    assert_eq!(Some(5000), min_ticket_price);
    assert_eq!(Some(20000), max_ticket_price);
}

#[test]
fn find_for_organization() {
    //create event
    let project = TestProject::new();
    let venue1 = project.create_venue().finish();
    let venue2 = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();

    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue1)
        .finish();
    event.add_artist(None, artist1.id, project.get_connection()).unwrap();
    event.add_artist(None, artist2.id, project.get_connection()).unwrap();

    //find more than one event
    let event2 = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();
    event2.add_artist(None, artist1.id, project.get_connection()).unwrap();

    let all_events = vec![event2.id, event.id];

    //find all events via organization
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        Some(PastOrUpcoming::Past),
        None,
        false,
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        all_events
    );

    // Restrict to just a subset of events
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        Some(PastOrUpcoming::Past),
        Some(vec![event.id]),
        false,
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        vec![event.id]
    );

    // Returning both events
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        Some(PastOrUpcoming::Past),
        Some(vec![event.id, event2.id]),
        false,
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        all_events
    );
}

#[test]
fn find_active_for_venue() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    //create two events
    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event.add_artist(None, artist1.id, project.get_connection()).unwrap();
    event.add_artist(None, artist2.id, project.get_connection()).unwrap();
    let event2 = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event2.add_artist(None, artist1.id, project.get_connection()).unwrap();
    //Cancel first event
    event.cancel(None, connection).unwrap();

    //find all active events via venue
    let found_events = Event::find_all_active_events_for_venue(&venue.id, project.get_connection()).unwrap();

    assert_eq!(found_events.len(), 1);
    assert_eq!(found_events[0].id, event2.id);
}

#[test]
fn find_for_venue() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let country_lookup = CountryLookup::new().unwrap();
    let venue = project.create_venue().with_name("Venue'1".to_string()).finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    //create two events
    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event.add_artist(None, artist1.id, project.get_connection()).unwrap();
    event.add_artist(None, artist2.id, project.get_connection()).unwrap();
    let event2 = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event2.add_artist(None, artist1.id, project.get_connection()).unwrap();

    let paging: &Paging = &Paging {
        page: 0,
        limit: 10,
        sort: "".to_string(),
        dir: SortingDir::Asc,
        total: 0,
        tags: HashMap::new(),
    };
    //find all active events via venue
    let all_found_events = Event::search(
        Some("Venue1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();

    assert_eq!(all_found_events.1, 2);

    //find all active events via venue
    let all_found_events = Event::search(
        Some("Venue'1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();

    assert_eq!(all_found_events.1, 2);

    //find all active events via venue
    let all_found_events = Event::search(
        Some("Venue 1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        None,
        paging,
        &country_lookup,
        connection,
    )
    .unwrap();

    assert_eq!(all_found_events.1, 0);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();

    assert_eq!(event.organization(project.get_connection()).unwrap(), organization);
}

#[test]
fn venue() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_venue(&venue)
        .finish();
    assert_eq!(event.venue(project.get_connection()).unwrap(), Some(venue));

    let event = project.create_event().with_name("NewEvent".into()).finish();
    assert_eq!(event.venue(project.get_connection()).unwrap(), None);
}

#[test]
fn add_ticket_type() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "General Admission".to_string(),
            None,
            100,
            Some(sd),
            Some(ed),
            TicketTypeEndDateType::Manual,
            Some(wallet_id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "General Admission".to_string());
}

#[test]
fn ticket_types() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type_ga = event
        .add_ticket_type(
            "General Admission".to_string(),
            None,
            100,
            Some(sd),
            Some(ed),
            TicketTypeEndDateType::Manual,
            Some(wallet_id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();
    let ticket_type_vip = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            Some(sd),
            Some(ed),
            TicketTypeEndDateType::Manual,
            Some(wallet_id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();

    let ticket_types = event.ticket_types(true, None, conn).unwrap();

    assert_equiv!(ticket_types, vec![ticket_type_ga, ticket_type_vip]);
}

#[test]
fn localized_time() {
    let utc_time = NaiveDateTime::parse_from_str("2019-01-01 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap();
    let localized_time = Event::localized_time(Some(utc_time), Some("Africa/Johannesburg")).unwrap();
    assert_eq!(localized_time.to_rfc2822(), "Tue,  1 Jan 2019 14:00:00 +0200");

    let invalid_localized_time = Event::localized_time(None, Some("Africa/Johannesburg"));
    assert_eq!(invalid_localized_time, None);

    let invalid_localized_time = Event::localized_time(Some(utc_time), None);
    assert_eq!(invalid_localized_time, None);

    let invalid_localized_time = Event::localized_time(None, None);
    assert_eq!(invalid_localized_time, None);
}

#[test]
fn get_all_localized_times() {
    let project = TestProject::new();
    //    let conn = project.get_connection();
    let venue = project
        .create_venue()
        .with_timezone("Africa/Johannesburg".to_string())
        .finish();
    let utc_time = NaiveDateTime::parse_from_str("2019-01-01 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap();
    let event = project
        .create_event()
        .with_event_start(utc_time.clone())
        .with_venue(&venue)
        .finish();

    let localized_times: EventLocalizedTimes = event.get_all_localized_times(Some(&venue));
    assert_eq!(
        localized_times.event_start.unwrap().to_rfc2822(),
        "Tue,  1 Jan 2019 14:00:00 +0200"
    );
    assert_eq!(
        localized_times.event_end.unwrap().to_rfc2822(),
        "Thu,  3 Jan 2019 14:00:00 +0200"
    );
    assert_eq!(
        localized_times.door_time.unwrap().to_rfc2822(),
        "Tue,  1 Jan 2019 13:00:00 +0200"
    );
}

#[test]
fn search_fans() {
    let project = TestProject::new();
    let organization = project.create_organization().finish();
    let order_user = project.create_user().finish();
    let order_user2 = project.create_user().finish();
    let order_user3 = project.create_user().finish();
    let order_user4 = project.create_user().finish();
    let box_office_user = project.create_user().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    project
        .create_event_interest()
        .with_user(&order_user2)
        .with_event(&event)
        .finish();
    project
        .create_order()
        .for_user(&order_user)
        .for_event(&event)
        .quantity(5)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&order_user2)
        .for_event(&event)
        .quantity(5)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&box_office_user)
        .on_behalf_of_user(&order_user2)
        .for_event(&event)
        .quantity(5)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&box_office_user)
        .on_behalf_of_user(&order_user3)
        .for_event(&event)
        .quantity(5)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_user(&order_user4)
        .for_event(&event2)
        .quantity(5)
        .is_paid()
        .finish();

    let search_results = organization
        .search_fans(
            None,
            0,
            100,
            FanSortField::FirstName,
            SortingDir::Asc,
            &project.connection,
        )
        .unwrap();
    let expected_results = vec![order_user.id, order_user2.id, order_user3.id, order_user4.id];
    let results: Vec<Uuid> = search_results.data.iter().map(|f| f.user_id).collect();
    assert_equiv!(results, expected_results);
}

#[test]
fn checked_in_users() {
    let project = TestProject::new();
    let admin = project.create_user().finish();

    let connection = project.get_connection();

    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    let result2 = TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.unwrap(),
        admin.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(result2, RedeemResults::TicketRedeemSuccess);
    let users = Event::checked_in_users(event.id, connection).unwrap();
    assert_eq!(users[0], user);
    //User purchases another ticket, there should still only by one user
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let users = Event::checked_in_users(event.id, connection).unwrap();
    assert_eq!(users.len(), 1);
}
