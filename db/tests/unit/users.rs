use std::collections::HashMap;

use chrono::{NaiveDateTime, Utc};
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::RunQueryDsl;
use uuid::Uuid;
use validator::Validate;

use db::dev::TestProject;
use db::prelude::*;
use db::schema::{orders, user_genres};
use db::utils::dates;
use db::utils::errors;
use db::utils::errors::ErrorCode;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn find_for_authentication() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let (found_user, is_public_user) = User::find_for_authentication(user.id, connection).unwrap();
    // User just exists in the system not as an organization member so is a public user
    assert_eq!(found_user, user);
    assert!(is_public_user);

    // User belongs to organization so is no longer a public user
    project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let (found_user, is_public_user) = User::find_for_authentication(user.id, connection).unwrap();
    assert_eq!(found_user, user);
    assert!(!is_public_user);

    // User is admin so is never a public user
    let mut admin = project.create_user().finish();
    admin = admin.add_role(Roles::Admin, connection).unwrap();
    let (found_user, is_public_user) = User::find_for_authentication(admin.id, connection).unwrap();
    assert_eq!(found_user, admin);
    assert!(!is_public_user);
}

#[test]
fn is_attending_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    assert!(!User::is_attending_event(user.id, event.id, connection).unwrap());
    assert!(!User::is_attending_event(user2.id, event.id, connection).unwrap());

    // Purchase user now has ticket
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .quantity(1)
        .finish();
    assert!(User::is_attending_event(user.id, event.id, connection).unwrap());
    assert!(!User::is_attending_event(user2.id, event.id, connection).unwrap());

    // Transfer ticket away, user no longer has a ticket, new user without order is seen as having it
    let ticket = &order.tickets(None, connection).unwrap()[0];
    TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    assert!(!User::is_attending_event(user.id, event.id, connection).unwrap());
    assert!(User::is_attending_event(user2.id, event.id, connection).unwrap());
}

#[test]
fn admins() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut user3 = project.create_user().finish();
    let mut user4 = project.create_user().finish();
    let _organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_member(&user2, Roles::OrgMember)
        .finish();
    user3 = user3.add_role(Roles::Admin, connection).unwrap();
    user4 = user4.add_role(Roles::Super, connection).unwrap();
    let admins = User::admins(connection).unwrap();
    assert!(!admins.contains(&user));
    assert!(!admins.contains(&user2));
    assert!(admins.contains(&user3));
    assert!(admins.contains(&user4));
}

#[test]
fn new_stub() {
    let first_name = "Penny".to_string();
    let last_name = "Quarter".to_string();
    let email = "penny@quarter.com".to_string();
    let phone = "1234567890".to_string();
    let user = User::new_stub(
        Some(first_name.clone()),
        Some(last_name.clone()),
        Some(email.clone()),
        Some(phone.clone()),
    );
    assert_eq!(Some(first_name), user.first_name);
    assert_eq!(Some(last_name), user.last_name);
    assert_eq!(Some(email), user.email);
    assert_eq!(Some(phone), user.phone);
    assert!(!user.hashed_pw.is_empty());
}

#[test]
fn create_stub() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let first_name = "Penny".to_string();
    let last_name = "Quarter".to_string();
    let email = "penny@quarter.com".to_string();
    let phone = "1234567890".to_string();
    let user = User::create_stub(
        first_name.clone(),
        last_name.clone(),
        Some(email.clone()),
        Some(phone.clone()),
        None,
        connection,
    )
    .unwrap();
    assert_eq!(Some(first_name), user.first_name);
    assert_eq!(Some(last_name), user.last_name);
    assert_eq!(Some(email), user.email);
    assert_eq!(Some(phone), user.phone);
    assert!(!user.hashed_pw.is_empty());
    assert!(User::find(user.id, connection).is_ok());
}

#[test]
fn update_genre_info() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let artist = project.create_artist().finish();
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
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    event.update_genres(None, connection).unwrap();
    let user = project.create_user().finish();

    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .quantity(1)
        .finish();
    // Clearing all genres
    diesel::delete(user_genres::table.filter(user_genres::user_id.eq(user.id)))
        .execute(connection)
        .unwrap();

    assert!(user.genres(connection).unwrap().is_empty());

    assert!(user.update_genre_info(connection).is_ok());
    assert_eq!(
        event.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
    assert_eq!(
        user.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
}

#[test]
fn transfer_activity_by_event_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(1).finish())
        .finish();
    let event2 = project
        .create_event()
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(2).finish())
        .finish();
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();
    let order2 = project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();

    let user_tickets = order.tickets(None, connection).unwrap();
    let ticket = &user_tickets[0];
    let ticket2 = &user_tickets[1];
    let ticket3 = &user_tickets[2];

    let user_tickets2 = order2.tickets(None, connection).unwrap();
    let ticket4 = &user_tickets2[0];
    let ticket5 = &user_tickets2[1];
    let ticket6 = &user_tickets2[2];

    // Completed transfer
    let transfer = TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user3.id,
        connection,
    )
    .unwrap();

    // Pending transfer
    let transfer2 = TicketInstance::create_transfer(&user, &[ticket2.id], None, None, false, connection).unwrap();

    // Cancelled transfer
    let transfer3 = TicketInstance::create_transfer(&user, &[ticket3.id], None, None, false, connection).unwrap();
    transfer3.cancel(&user, None, connection).unwrap();

    // Cancelled and retransferred ticket
    let transfer4 = TicketInstance::create_transfer(
        &user,
        &[ticket4.id, ticket5.id, ticket6.id],
        None,
        None,
        false,
        connection,
    )
    .unwrap();
    let transfer4 = transfer4.cancel(&user, None, connection).unwrap();
    // Only ticket 4 and 5 retransferred
    let transfer5 = TicketInstance::create_transfer(&user, &[ticket4.id], None, None, false, connection).unwrap();
    let transfer6 = TicketInstance::create_transfer(&user, &[ticket5.id], None, None, false, connection).unwrap();
    // Ticket 5 is accepted by user2
    let sender_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer6.into_authorization(connection).unwrap(),
        &sender_wallet,
        user2.id,
        receiver_wallet.id,
        connection,
    )
    .unwrap();
    // Ticket 5 is transferred again and accepted by user3
    let transfer7 = TicketInstance::create_transfer(&user2, &[ticket5.id], None, None, false, connection).unwrap();
    let sender_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user3.id, connection).unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer7.into_authorization(connection).unwrap(),
        &sender_wallet,
        user3.id,
        receiver_wallet.id,
        connection,
    )
    .unwrap();

    // Adjust domain events so they order correctly / avoid timing test errors
    diesel::sql_query(
        r#"
        UPDATE domain_events
        SET created_at = $1
        WHERE main_id = $2
        AND event_type = 'TransferTicketStarted';
    "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(5).finish())
    .bind::<sql_types::Uuid, _>(transfer4.id)
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE domain_events
        SET created_at = $1
        WHERE main_id = $2
        AND event_type = 'TransferTicketCancelled';
    "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(10).finish())
    .bind::<sql_types::Uuid, _>(transfer4.id)
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE domain_events
        SET created_at = $1
        WHERE main_id = $2
        AND event_type = 'TransferTicketStarted';
    "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(15).finish())
    .bind::<sql_types::Uuid, _>(transfer5.id)
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE domain_events
        SET created_at = $1
        WHERE main_id = $2
        AND event_type = 'TransferTicketStarted';
    "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(15).finish())
    .bind::<sql_types::Uuid, _>(transfer6.id)
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE domain_events
        SET created_at = $1
        WHERE main_id = $2
        AND event_type = 'TransferTicketCompleted';
    "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(20).finish())
    .bind::<sql_types::Uuid, _>(transfer6.id)
    .execute(connection)
    .unwrap();

    let mut user1_activity = user
        .transfer_activity_by_event_tickets(0, 100, SortingDir::Desc, PastOrUpcoming::Upcoming, connection)
        .unwrap();
    let mut user2_activity = user2
        .transfer_activity_by_event_tickets(0, 100, SortingDir::Desc, PastOrUpcoming::Upcoming, connection)
        .unwrap();
    let user3_activity = user3
        .transfer_activity_by_event_tickets(0, 100, SortingDir::Desc, PastOrUpcoming::Upcoming, connection)
        .unwrap();

    assert_eq!(user1_activity.paging.total, 2);
    let mut user_event2_data = user1_activity.data.remove(0);
    assert_eq!(user_event2_data.event, event2.for_display(connection).unwrap());
    assert_eq!(user_event2_data.ticket_activity_items.len(), 2);
    let mut ticket_activity = user_event2_data.ticket_activity_items.remove(&ticket4.id).unwrap();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer5.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Pending);
        assert_eq!(ticket_ids, vec![ticket4.id]);
    }
    let mut transfer4_tickets = vec![ticket4.id, ticket5.id, ticket6.id];
    transfer4_tickets.sort();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        mut ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        ticket_ids.sort();
        assert_eq!(transfer_id, transfer4.id);
        assert_eq!(action, "Cancelled".to_string());
        assert_eq!(status, TransferStatus::Cancelled);
        assert_eq!(ticket_ids, transfer4_tickets);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        mut ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        ticket_ids.sort();
        assert_eq!(transfer_id, transfer4.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Cancelled);
        assert_eq!(ticket_ids, transfer4_tickets);
    }
    let mut ticket_activity = user_event2_data.ticket_activity_items.remove(&ticket5.id).unwrap();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer6.id);
        assert_eq!(action, "Accepted".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket5.id]);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer6.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket5.id]);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        mut ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        ticket_ids.sort();
        assert_eq!(transfer_id, transfer4.id);
        assert_eq!(action, "Cancelled".to_string());
        assert_eq!(status, TransferStatus::Cancelled);
        assert_eq!(ticket_ids, transfer4_tickets);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        mut ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        ticket_ids.sort();
        assert_eq!(transfer_id, transfer4.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Cancelled);
        assert_eq!(ticket_ids, transfer4_tickets);
    }

    let mut user_event_data = user1_activity.data.remove(0);
    assert_eq!(user_event_data.event, event.for_display(connection).unwrap());
    assert_eq!(user_event_data.ticket_activity_items.len(), 2);
    let mut ticket_activity = user_event_data.ticket_activity_items.remove(&ticket.id).unwrap();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer.id);
        assert_eq!(action, "Accepted".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket.id]);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket.id]);
    }
    let mut ticket_activity = user_event_data.ticket_activity_items.remove(&ticket2.id).unwrap();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer2.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Pending);
        assert_eq!(ticket_ids, vec![ticket2.id]);
    }

    assert_eq!(user2_activity.paging.total, 1);
    let mut user2_event2_data = user2_activity.data.remove(0);
    assert_eq!(user2_event2_data.event, event2.for_display(connection).unwrap());
    assert_eq!(user2_event2_data.ticket_activity_items.len(), 1);
    let mut ticket_activity = user2_event2_data.ticket_activity_items.remove(&ticket5.id).unwrap();
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer7.id);
        assert_eq!(action, "Accepted".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket5.id]);
    }
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = ticket_activity.remove(0)
    {
        assert_eq!(transfer_id, transfer7.id);
        assert_eq!(action, "Started".to_string());
        assert_eq!(status, TransferStatus::Completed);
        assert_eq!(ticket_ids, vec![ticket5.id]);
    }

    assert_eq!(user3_activity.paging.total, 0);
}

#[test]
fn activity() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let organization2 = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .finish();
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
    let order = project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .for_user(&user3)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    diesel::sql_query(
        r#"
        UPDATE orders
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(order.id)
    .execute(connection)
    .unwrap();
    let order = project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();
    diesel::sql_query(
        r#"
        UPDATE orders
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_minutes(-30).finish())
    .bind::<sql_types::Uuid, _>(order.id)
    .execute(connection)
    .unwrap();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    assert_eq!(
        vec![ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }],
        user.activity(
            &organization,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
    );
    assert_eq!(
        vec![ActivitySummary {
            activity_items: ActivityItem::load_for_event(event2.id, user.id, None, connection).unwrap(),
            event: event2.for_display(connection).unwrap(),
        }],
        user.activity(
            &organization2,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
    );

    assert!(user2
        .activity(
            &organization,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
        .is_empty());
    assert!(user2
        .activity(
            &organization2,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
        .is_empty());

    assert_eq!(
        vec![ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }],
        user.activity(
            &organization,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
    );

    // Event is now in the past
    diesel::sql_query(
        r#"
        UPDATE events
        SET event_start = $1,
        event_end = $2
        WHERE id = $3;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-2).finish())
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-1).finish())
    .bind::<sql_types::Uuid, _>(event.id)
    .execute(connection)
    .unwrap();
    let event = Event::find(event.id, connection).unwrap();

    // Is not found via upcoming filter
    assert!(user
        .activity(
            &organization,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Upcoming,
            None,
            connection
        )
        .unwrap()
        .data
        .is_empty());

    // Is found via past filter
    assert_eq!(
        vec![ActivitySummary {
            activity_items: ActivityItem::load_for_event(event.id, user.id, None, connection).unwrap(),
            event: event.for_display(connection).unwrap(),
        }],
        user.activity(
            &organization,
            0,
            100,
            SortingDir::Asc,
            PastOrUpcoming::Past,
            None,
            connection
        )
        .unwrap()
        .data
    );
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut user_ids = vec![user.id, user2.id];
    user_ids.sort();

    let found_users = User::find_by_ids(&user_ids, connection).unwrap();
    let mut found_user_ids: Vec<Uuid> = found_users.into_iter().map(|u| u.id).collect();
    found_user_ids.sort();
    assert_eq!(found_user_ids, user_ids);
}

#[test]
fn genres() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let artist = project.create_artist().finish();
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
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    event.update_genres(None, connection).unwrap();

    // No genres as no purchases yet
    let user = project.create_user().finish();
    assert!(user.genres(connection).unwrap().is_empty());

    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .quantity(1)
        .finish();

    assert_eq!(
        event.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
    assert_eq!(
        user.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
}

#[test]
fn commit() {
    let project = TestProject::new();
    let first_name = Some("Jeff".to_string());
    let last_name = Some("Wilco".to_string());
    let email = Some("jeff@tari.com".to_string());
    let phone_number = Some("555-555-5555".to_string());
    let password = "examplePassword";
    let user = User::create(
        first_name.clone(),
        last_name.clone(),
        email.clone(),
        phone_number.clone(),
        password,
    )
    .commit(None, project.get_connection())
    .unwrap();

    assert_eq!(user.first_name, first_name);
    assert_eq!(user.last_name, last_name);
    assert_eq!(user.email, email);
    assert_eq!(user.phone, phone_number);
    assert_ne!(user.hashed_pw, password);
    assert_eq!(user.hashed_pw.is_empty(), false);
    assert_eq!(user.id.to_string().is_empty(), false);

    let wallets = user.wallets(project.get_connection()).unwrap();
    assert_eq!(wallets.len(), 1);
}

#[test]
fn commit_duplicate_email() {
    let project = TestProject::new();
    let user1 = project.create_user().finish();
    let first_name = Some("Jeff".to_string());
    let last_name = Some("Wilco".to_string());
    let email = user1.email;
    let phone_number = Some("555-555-5555".to_string());
    let password = "examplePassword";
    let result =
        User::create(first_name, last_name, email, phone_number, password).commit(None, project.get_connection());

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.err().unwrap().code,
        errors::get_error_message(&ErrorCode::DuplicateKeyError).0
    );
}

#[test]
fn find_external_login() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No external login for facebook, returns None
    assert_eq!(
        None,
        user.find_external_login(FACEBOOK_SITE, connection).optional().unwrap()
    );

    // With external login present
    let external_login = user
        .add_external_login(
            None,
            "abc".to_string(),
            FACEBOOK_SITE.to_string(),
            "123".to_string(),
            vec!["email".to_string()],
            connection,
        )
        .unwrap();
    assert_eq!(
        Some(external_login),
        user.find_external_login(FACEBOOK_SITE, connection).optional().unwrap()
    );
}

#[test]
fn get_profile_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let user5 = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();

    let event = project
        .create_event()
        .with_event_start(NaiveDateTime::from(dates::now().add_days(1).finish()))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_event_start(NaiveDateTime::from(dates::now().add_days(2).finish()))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event3 = project
        .create_event()
        .with_event_start(NaiveDateTime::from(dates::now().add_days(3).finish()))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type3 = &event3.ticket_types(true, None, connection).unwrap()[0];

    // No purchases / no organization link
    assert_eq!(
        user.get_profile_for_organization(&organization, connection),
        Err(DatabaseError {
            code: 2000,
            message: "No results".into(),
            cause: Some("Could not load profile for organization fan, NotFound".into()),
            error_code: ErrorCode::NoResults,
        })
    );

    // Add facebook login
    user.add_external_login(
        None,
        "abc".to_string(),
        FACEBOOK_SITE.to_string(),
        "123".to_string(),
        vec!["email".to_string()],
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection),
        Err(DatabaseError {
            code: 2000,
            message: "No results".into(),
            cause: Some("Could not load profile for organization fan, NotFound".into()),
            error_code: ErrorCode::NoResults,
        })
    );

    // Add order but do not checkout
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
    assert_eq!(
        user.get_profile_for_organization(&organization, connection),
        Err(DatabaseError {
            code: 2000,
            message: "No results".into(),
            cause: Some("Could not load profile for organization fan, NotFound".into()),
            error_code: ErrorCode::NoResults,
        })
    );

    // Add event interest giving access without orders
    project
        .create_event_interest()
        .with_event(&event)
        .with_user(&user)
        .finish();

    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 0,
            ticket_sales: 0,
            tickets_owned: 0,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
            deleted_at: None
        }
    );

    // Checkout which changes sales data
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
    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 1700,
            ticket_sales: 10,
            tickets_owned: 10,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
            deleted_at: None
        }
    );

    // Redeem tickets from order
    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 1700,
            ticket_sales: 10,
            tickets_owned: 10,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
            deleted_at: None
        }
    );

    // Checkout with a second order same event
    let order = project
        .create_order()
        .for_user(&user)
        .for_event(&event)
        .quantity(1)
        .is_paid()
        .finish();

    // Redeem a ticket from new order
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();

    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 1870,
            ticket_sales: 11,
            tickets_owned: 11,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
            deleted_at: None
        }
    );

    // Checkout with new event increasing event count as well
    let order = project
        .create_order()
        .for_user(&user)
        .for_event(&event2)
        .quantity(1)
        .is_paid()
        .finish();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 2040,
            ticket_sales: 12,
            tickets_owned: 12,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
            deleted_at: None
        }
    );

    // Redeem ticket from new event
    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Transfer ticket to different user removing it from attendance information and moving it to theirs
    TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "example@tari.com",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    // Reload ticket for new redeem key as ticket was transferred
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 2040,
            ticket_sales: 12,
            tickets_owned: 11,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
            deleted_at: None
        }
    );
    assert_eq!(
        user2.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user2.first_name.clone(),
            last_name: user2.last_name.clone(),
            email: user2.email.clone(),
            facebook_linked: false,
            revenue_in_cents: 0,
            ticket_sales: 0,
            tickets_owned: 1,
            profile_pic_url: user2.profile_pic_url.clone(),
            thumb_profile_pic_url: user2.thumb_profile_pic_url.clone(),
            cover_photo_url: user2.cover_photo_url.clone(),
            created_at: user2.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event2.name.clone(),
                event_id: event2.id,
                event_start: event2.event_start
            }],
            deleted_at: None
        }
    );

    // Purchase and redeem from other event without transferring
    let order = project
        .create_order()
        .for_user(&user)
        .for_event(&event2)
        .quantity(1)
        .is_paid()
        .finish();
    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();

    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 2210,
            ticket_sales: 13,
            tickets_owned: 12,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![
                AttendanceInformation {
                    event_name: event.name.clone(),
                    event_id: event.id,
                    event_start: event.event_start
                },
                AttendanceInformation {
                    event_name: event2.name.clone(),
                    event_id: event2.id,
                    event_start: event2.event_start
                }
            ],
            deleted_at: None
        }
    );

    // Purchased by other user and transferred
    let order = project
        .create_order()
        .for_user(&user2)
        .for_event(&event3)
        .quantity(1)
        .is_paid()
        .finish();

    // Redeem ticket from new event
    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type3.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    TicketInstance::direct_transfer(
        &user2,
        &vec![ticket.id],
        "example@tari.com",
        TransferMessageType::Email,
        user4.id,
        connection,
    )
    .unwrap();
    assert_eq!(
        user4.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user4.first_name.clone(),
            last_name: user4.last_name.clone(),
            email: user4.email.clone(),
            facebook_linked: false,
            revenue_in_cents: 0,
            ticket_sales: 0,
            tickets_owned: 1,
            profile_pic_url: user4.profile_pic_url.clone(),
            thumb_profile_pic_url: user4.thumb_profile_pic_url.clone(),
            cover_photo_url: user4.cover_photo_url.clone(),
            created_at: user4.created_at,
            attendance_information: vec![],
            deleted_at: None
        }
    );

    // Transfer ticket again
    TicketInstance::direct_transfer(
        &user4,
        &vec![ticket.id],
        "example@tari.com",
        TransferMessageType::Email,
        user5.id,
        connection,
    )
    .unwrap();

    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user5.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(
        user4.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user4.first_name.clone(),
            last_name: user4.last_name.clone(),
            email: user4.email.clone(),
            facebook_linked: false,
            revenue_in_cents: 0,
            ticket_sales: 0,
            tickets_owned: 0,
            profile_pic_url: user4.profile_pic_url.clone(),
            thumb_profile_pic_url: user4.thumb_profile_pic_url.clone(),
            cover_photo_url: user4.cover_photo_url.clone(),
            created_at: user4.created_at,
            attendance_information: vec![],
            deleted_at: None
        }
    );
    assert_eq!(
        user5.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user5.first_name.clone(),
            last_name: user5.last_name.clone(),
            email: user5.email.clone(),
            facebook_linked: false,
            revenue_in_cents: 0,
            ticket_sales: 0,
            tickets_owned: 1,
            profile_pic_url: user5.profile_pic_url.clone(),
            thumb_profile_pic_url: user5.thumb_profile_pic_url.clone(),
            cover_photo_url: user5.cover_photo_url.clone(),
            created_at: user5.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event3.name.clone(),
                event_id: event3.id,
                event_start: event3.event_start
            }],
            deleted_at: None
        }
    );

    // Box office purchase shows up on new user's profile but not on box office user's
    project
        .create_order()
        .for_user(&user)
        .on_behalf_of_user(&user3)
        .for_event(&event)
        .quantity(1)
        .is_paid()
        .finish();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            revenue_in_cents: 2210,
            ticket_sales: 13,
            tickets_owned: 12,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![
                AttendanceInformation {
                    event_name: event.name.clone(),
                    event_id: event.id,
                    event_start: event.event_start
                },
                AttendanceInformation {
                    event_name: event2.name.clone(),
                    event_id: event2.id,
                    event_start: event2.event_start
                }
            ],
            deleted_at: None
        }
    );
    assert_eq!(
        user3.get_profile_for_organization(&organization, connection).unwrap(),
        FanProfile {
            first_name: user3.first_name.clone(),
            last_name: user3.last_name.clone(),
            email: user3.email.clone(),
            facebook_linked: false,
            revenue_in_cents: 150,
            ticket_sales: 1,
            tickets_owned: 1,
            profile_pic_url: user3.profile_pic_url.clone(),
            thumb_profile_pic_url: user3.thumb_profile_pic_url.clone(),
            cover_photo_url: user3.cover_photo_url.clone(),
            created_at: user3.created_at,
            attendance_information: Vec::new(),
            deleted_at: None
        }
    );
}

#[test]
fn get_history_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // No history to date
    assert!(user
        .get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
        .unwrap()
        .is_empty());

    // User adds item to cart but does not checkout so no history
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
    assert!(user
        .get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
        .unwrap()
        .is_empty());

    // User checks out so has a paid order so history exists
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

    let mut paging = Paging::new(0, 100);
    paging.dir = SortingDir::Desc;
    let mut payload = Payload::new(
        vec![HistoryItem::Purchase {
            order_id: cart.id,
            order_date: cart.order_date,
            event_name: event.name.clone(),
            ticket_sales: 10,
            revenue_in_cents: 1700,
        }],
        paging,
    );
    payload.paging.total = 1;
    assert_eq!(
        user.get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
            .unwrap(),
        payload
    );

    // User makes a second order
    let mut cart2 = Order::find_or_create_cart(&user, connection).unwrap();
    cart2
        .update_quantities(
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

    // Update cart2 to a future date to avoid test timing errors
    let mut cart2 = diesel::update(orders::table.filter(orders::id.eq(cart2.id)))
        .set(orders::order_date.eq(dates::now().add_seconds(1).finish()))
        .get_result::<Order>(connection)
        .unwrap();

    assert_eq!(cart2.calculate_total(connection).unwrap(), 170);
    cart2
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            170,
            connection,
        )
        .unwrap();
    assert_eq!(cart2.status, OrderStatus::Paid);

    let mut paging = Paging::new(0, 100);
    paging.dir = SortingDir::Desc;
    let mut payload = Payload::new(
        vec![
            HistoryItem::Purchase {
                order_id: cart2.id,
                order_date: cart2.order_date,
                event_name: event.name.clone(),
                ticket_sales: 1,
                revenue_in_cents: 170,
            },
            HistoryItem::Purchase {
                order_id: cart.id,
                order_date: cart.order_date,
                event_name: event.name.clone(),
                ticket_sales: 10,
                revenue_in_cents: 1700,
            },
        ],
        paging,
    );
    payload.paging.total = 2;
    assert_eq!(
        user.get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
            .unwrap(),
        payload
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    let found_user = User::find(user.id, project.get_connection()).expect("User was not found");
    assert_eq!(found_user.id, user.id);
    assert_eq!(found_user.email, user.email);

    assert!(
        match User::find(Uuid::new_v4(), project.get_connection()) {
            Ok(_user) => false,
            Err(_e) => true,
        },
        "User incorrectly returned when id invalid"
    );
}

#[test]
fn event_users() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    assert_eq!(user.event_users(connection).unwrap(), vec![event_user]);
}

#[test]
fn get_event_ids_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No results
    assert_eq!(
        (HashMap::new(), HashMap::new()),
        user.get_event_ids_by_organization(connection).unwrap()
    );

    let organization = project.create_organization().with_name("Organization1".into()).finish();
    let organization2 = project.create_organization().with_name("Organization2".into()).finish();
    let organization3 = project
        .create_organization()
        .with_name("Organization3".into())
        .with_member(&user, Roles::OrgAdmin)
        .finish();

    let event = project.create_event().with_organization(&organization).finish();
    let event2 = project.create_event().with_organization(&organization2).finish();
    let event3 = project.create_event().with_organization(&organization2).finish();

    organization
        .add_user(user.id, vec![Roles::PromoterReadOnly], vec![event.id], connection)
        .unwrap();
    organization2
        .add_user(user.id, vec![Roles::Promoter], vec![event2.id, event3.id], connection)
        .unwrap();

    let (events_by_organization, readonly_events_by_organization) =
        user.get_event_ids_by_organization(connection).unwrap();
    assert!(events_by_organization.get(&organization.id).unwrap().is_empty());
    assert!(readonly_events_by_organization
        .get(&organization2.id)
        .unwrap()
        .is_empty());
    let organization_results = readonly_events_by_organization.get(&organization.id).unwrap();
    assert_eq!(&vec![event.id], organization_results);
    let mut organization2_results = events_by_organization.get(&organization2.id).unwrap().clone();
    organization2_results.sort();
    let mut expected_organization2 = vec![event2.id, event3.id];
    expected_organization2.sort();
    assert_eq!(&expected_organization2, &organization2_results);

    // get_event_ids_for_organization
    assert_eq!(
        vec![event.id],
        user.get_event_ids_for_organization(organization.id, connection)
            .unwrap()
    );
    let mut organization2_results = user
        .get_event_ids_for_organization(organization2.id, connection)
        .unwrap();
    organization2_results.sort();
    assert_eq!(&expected_organization2, &organization2_results);

    assert!(user
        .get_event_ids_for_organization(organization3.id, connection)
        .unwrap()
        .is_empty());
}

#[test]
fn payment_method() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    assert!(user
        .payment_method(PaymentProviders::External, project.get_connection())
        .is_err());

    let payment_method = project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert_eq!(
        payment_method,
        user.payment_method(payment_method.name.clone(), project.get_connection())
            .unwrap(),
    );
}

#[test]
fn default_payment_method() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No payment methods set
    assert!(user.default_payment_method(connection).is_err());

    // Payment method exists but not default
    project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert!(user.default_payment_method(connection).is_err());

    // Default set
    let payment_method2 = project
        .create_payment_method()
        .with_name(PaymentProviders::Stripe)
        .with_user(&user)
        .make_default()
        .finish();
    let default_payment_method = user.default_payment_method(connection).unwrap();
    assert_eq!(payment_method2, default_payment_method);
}

#[test]
fn payment_methods() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    assert!(user.payment_methods(connection).unwrap().is_empty());

    let payment_method = project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert_eq!(vec![payment_method.clone()], user.payment_methods(connection).unwrap(),);

    let payment_method2 = project
        .create_payment_method()
        .with_name(PaymentProviders::Stripe)
        .with_user(&user)
        .finish();
    assert_eq!(
        vec![payment_method, payment_method2],
        user.payment_methods(connection).unwrap(),
    );
}

#[test]
fn full_name() {
    let project = TestProject::new();

    let first_name = "Bob".to_string();
    let last_name = "Jones".to_string();

    let user = project
        .create_user()
        .with_first_name(&first_name)
        .with_last_name(&last_name)
        .finish();
    assert_eq!(user.full_name(), format!("{} {}", first_name, last_name));
}

#[test]
fn find_by_email() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    let found_user =
        User::find_by_email(&user.email.clone().unwrap(), false, project.get_connection()).expect("User was not found");
    assert_eq!(found_user, user);

    let not_found = User::find_by_email("not@real.com", false, project.get_connection());
    let error = not_found.unwrap_err();
    assert_eq!(
        error.to_string(),
        "[2000] No results\nCaused by: Error loading user, NotFound"
    );
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut attributes: UserEditableAttributes = Default::default();
    let email = "new_email@tari.com";
    attributes.email = Some(email.to_string());

    let updated_user = user.update(attributes.into(), None, connection).unwrap();
    assert_eq!(updated_user.email, Some(email.into()));
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    let mut attributes: UserEditableAttributes = Default::default();
    let email = user2.email.clone();
    attributes.email = email;

    let result = user.update(attributes.into(), None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("email"));
                assert_eq!(errors["email"].len(), 1);
                assert_eq!(errors["email"][0].code, "uniqueness");
                assert_eq!(
                    &errors["email"][0].message.clone().unwrap().into_owned(),
                    "Email is already in use"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Ignores case
    let mut attributes: UserEditableAttributes = Default::default();
    let email = user2.email.clone().map(|e| e.to_uppercase());
    attributes.email = email;

    let result = user.update(attributes.into(), None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("email"));
                assert_eq!(errors["email"].len(), 1);
                assert_eq!(errors["email"][0].code, "uniqueness");
                assert_eq!(
                    &errors["email"][0].message.clone().unwrap().into_owned(),
                    "Email is already in use"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn new_user_validate() {
    let email = "abc";
    let user = User::create(
        Some("First".to_string()),
        Some("Last".to_string()),
        Some(email.to_string()),
        Some("123".to_string()),
        &"Password",
    );
    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("email"));
    assert_eq!(errors["email"].len(), 1);
    assert_eq!(errors["email"][0].code, "email");
    assert_eq!(
        &errors["email"][0].message.clone().unwrap().into_owned(),
        "Email is invalid"
    );
}

#[test]
fn user_editable_attributes_validate() {
    let mut user_parameters: UserEditableAttributes = Default::default();
    user_parameters.email = Some("abc".into());

    let result = user_parameters.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("email"));
    assert_eq!(errors["email"].len(), 1);
    assert_eq!(errors["email"][0].code, "email");
    assert_eq!(
        &errors["email"][0].message.clone().unwrap().into_owned(),
        "Email is invalid"
    );
}

#[test]
fn create_from_external_login() {
    let project = TestProject::new();
    let external_id = "123";
    let first_name = "Dennis";
    let last_name = "Miguel";
    let email = "dennis@tari.com";
    let site = "facebook.com";
    let access_token = "abc-123";

    let user = User::create_from_external_login(
        external_id.to_string(),
        first_name.to_string(),
        last_name.to_string(),
        Some(email.to_string()),
        site.to_string(),
        access_token.to_string(),
        vec!["email".to_string()],
        None,
        project.get_connection(),
    )
    .unwrap();

    let external_login = ExternalLogin::find_user(external_id, site, project.get_connection())
        .unwrap()
        .unwrap();

    assert_eq!(user.id, external_login.user_id);
    assert_eq!(access_token, external_login.access_token);
    assert_eq!(site, external_login.site);
    assert_eq!(external_id, external_login.external_user_id);

    assert_eq!(Some(email.to_string()), user.email);
    assert_eq!(Some(first_name.to_string()), user.first_name);
    assert_eq!(Some(last_name.to_string()), user.last_name);
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user_id = user.id.clone();
    let display_user = user.for_display().unwrap();

    assert_eq!(display_user.id, user_id);
}

#[test]
fn organizations() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project.create_organization().with_name("Organization3".into()).finish();

    assert_eq!(
        vec![organization, organization2],
        user.organizations(connection).unwrap()
    );
}

#[test]
fn find_events_with_access_to_scan() {
    //create event
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let owner = project.create_user().finish();
    let scanner = project.create_user().finish();
    let _normal_user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&scanner, Roles::OrgMember)
        .finish();
    let _draft_event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_event_start(Utc::now().naive_utc())
        .with_name("DraftEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let published_event = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_event_start(Utc::now().naive_utc())
        .with_name("PublishedEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _published_external_event = project
        .create_event()
        .with_status(EventStatus::Published)
        .external()
        .with_event_start(Utc::now().naive_utc())
        .with_name("PublishedExternalEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    let owner_events = owner.find_events_with_access_to_scan(connection).unwrap();
    let scanner_events = scanner.find_events_with_access_to_scan(connection).unwrap();
    let normal_user_events = _normal_user.find_events_with_access_to_scan(connection).unwrap();

    assert_eq!(owner_events, vec![published_event.clone()]);
    assert_eq!(scanner_events, vec![published_event]);
    assert!(normal_user_events.is_empty());
}

#[test]
fn get_roles_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project.create_organization().with_name("Organization3".into()).finish();

    let mut expected_results: HashMap<Uuid, (Vec<Roles>, Option<AdditionalOrgMemberScopes>)> = HashMap::new();
    expected_results.insert(organization.id.clone(), (vec![Roles::OrgOwner], None));
    expected_results.insert(organization2.id.clone(), (vec![Roles::OrgMember], None));

    assert_eq!(user.get_roles_by_organization(connection).unwrap(), expected_results);
}

#[test]
fn get_scopes_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project.create_organization().with_name("Organization3".into()).finish();

    let mut expected_results = HashMap::new();
    expected_results.insert(
        organization.id,
        vec![
            Scopes::AnnouncementEngagementWrite,
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventBroadcast,
            Scopes::EventCancel,
            Scopes::EventClone,
            Scopes::EventDataRead,
            Scopes::EventDelete,
            Scopes::EventFinancialReports,
            Scopes::EventInterest,
            Scopes::EventReportSubscriberDelete,
            Scopes::EventReportSubscriberRead,
            Scopes::EventReportSubscriberWrite,
            Scopes::EventReports,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::ListingWrite,
            Scopes::LootBoxWrite,
            Scopes::NoteDelete,
            Scopes::NoteRead,
            Scopes::NoteWrite,
            Scopes::OrderMakeExternalPayment,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrderResendConfirmation,
            Scopes::OrgAdminUsers,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::OrgReadEvents,
            Scopes::OrgReports,
            Scopes::OrgUsers,
            Scopes::OrgWrite,
            Scopes::RarityWrite,
            Scopes::RedeemTicket,
            Scopes::ScanReportRead,
            Scopes::SettlementRead,
            Scopes::TransferCancel,
            Scopes::TransferCancelOwn,
            Scopes::TransferRead,
            Scopes::TransferReadOwn,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketWrite,
            Scopes::TicketWriteOwn,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::UserRead,
            Scopes::VenueWrite,
            Scopes::WebSocketInitiate,
        ],
    );
    expected_results.insert(
        organization2.id,
        vec![
            Scopes::AnnouncementEngagementWrite,
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventBroadcast,
            Scopes::EventCancel,
            Scopes::EventClone,
            Scopes::EventDelete,
            Scopes::EventInterest,
            Scopes::EventReportSubscriberDelete,
            Scopes::EventReportSubscriberRead,
            Scopes::EventReportSubscriberWrite,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::ListingWrite,
            Scopes::LootBoxWrite,
            Scopes::NoteRead,
            Scopes::NoteWrite,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrderResendConfirmation,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::OrgReadEvents,
            Scopes::RarityWrite,
            Scopes::RedeemTicket,
            Scopes::ScanReportRead,
            Scopes::TransferCancel,
            Scopes::TransferCancelOwn,
            Scopes::TransferRead,
            Scopes::TransferReadOwn,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketWriteOwn,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::VenueWrite,
            Scopes::WebSocketInitiate,
        ],
    );

    assert_eq!(user.get_scopes_by_organization(connection).unwrap(), expected_results);
}

#[test]
fn get_global_scopes() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut user3 = project.create_user().finish();
    let mut user4 = project.create_user().finish();
    let _organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_member(&user2, Roles::OrgMember)
        .finish();
    user3 = user3.add_role(Roles::Admin, connection).unwrap();
    user4 = user4.add_role(Roles::Super, connection).unwrap();

    assert_eq!(
        user.get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec![
            "event:interest",
            "listing:write",
            "order:read-own",
            "transfer:cancel-own",
            "transfer:read-own",
            "ticket:write-own",
            "ticket:transfer"
        ]
    );
    assert_eq!(
        user2
            .get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec![
            "event:interest",
            "listing:write",
            "order:read-own",
            "transfer:cancel-own",
            "transfer:read-own",
            "ticket:write-own",
            "ticket:transfer"
        ]
    );
    assert_equiv!(
        user3
            .get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec![
            "announcement:delete",
            "announcement:read",
            "announcement:write",
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "listing:write",
            "loot-box:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:refund-override",
            "order:resend-confirmation",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:modify-settlement-type",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "org-venue:delete",
            "org-venue:read",
            "org-venue:write",
            "rarity:write",
            "redeem:ticket",
            "region:write",
            "report:admin",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement-adjustment:write",
            "settlement:read",
            "settlement:read-early",
            "settlement:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "ticket-type:read",
            "ticket-type:write",
            "transfer:cancel-accepted",
            "transfer:cancel-own",
            "transfer:cancel",
            "transfer:read-own",
            "transfer:read",
            "user:delete",
            "user:read",
            "venue:write",
            "websocket:initiate"
        ]
    );
    assert_equiv!(
        user4
            .get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec![
            "announcement:delete",
            "announcement:read",
            "announcement:write",
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "listing:write",
            "loot-box:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:refund-override",
            "order:resend-confirmation",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:modify-settlement-type",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "org-venue:delete",
            "org-venue:read",
            "org-venue:write",
            "rarity:write",
            "redeem:ticket",
            "region:write",
            "report:admin",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement-adjustment:write",
            "settlement:read",
            "settlement:read-early",
            "settlement:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "ticket-type:read",
            "ticket-type:write",
            "transfer:cancel-accepted",
            "transfer:cancel-own",
            "transfer:cancel",
            "transfer:read-own",
            "transfer:read",
            "user:delete",
            "user:read",
            "venue:write",
            "websocket:initiate"
        ]
    );
}

#[test]
fn add_role() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    user.add_role(Roles::Admin, project.get_connection()).unwrap();
    //Try adding a duplicate role to check that it isnt duplicated.
    user.add_role(Roles::Admin, project.get_connection()).unwrap();

    let user2 = User::find(user.id, project.get_connection()).unwrap();
    assert_eq!(user2.role, vec![Roles::User, Roles::Admin]);
}
