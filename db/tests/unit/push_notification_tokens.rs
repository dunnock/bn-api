use db::dev::TestProject;
use db::models::*;

#[test]
fn create_and_find_by_user_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    //Create two push notification tokens for user
    let mut pnt1_request = NewPushNotificationToken {
        user_id: user.id,
        token_source: "example_token1_source".to_string(),
        token: "example_token1".to_string(),
    };
    let mut pnt2_request = NewPushNotificationToken {
        user_id: user.id,
        token_source: "example_token2_source".to_string(),
        token: "example_token2".to_string(),
    };

    let pnt1 = pnt1_request.commit(user.id, connection).unwrap();
    let pnt2 = pnt2_request.commit(user.id, connection).unwrap();

    // Domain events created for push notifications
    assert_eq!(
        DomainEvent::find(
            Tables::PushNotificationTokens,
            Some(pnt1.id),
            Some(DomainEventTypes::PushNotificationTokenCreated),
            connection,
        )
        .unwrap()
        .len(),
        1
    );

    assert_eq!(
        DomainEvent::find(
            Tables::PushNotificationTokens,
            Some(pnt2.id),
            Some(DomainEventTypes::PushNotificationTokenCreated),
            connection,
        )
        .unwrap()
        .len(),
        1
    );

    //Check stored push notification tokens
    let push_notification_tokens = PushNotificationToken::find_by_user_id(user.id, connection).unwrap();
    assert_eq!(push_notification_tokens.len(), 2);
    if push_notification_tokens[0].token_source != pnt1_request.token_source {
        //Switch order
        let temp = pnt1_request;
        pnt1_request = pnt2_request;
        pnt2_request = temp;
    }
    assert_eq!(push_notification_tokens[0].user_id, pnt1_request.user_id);
    assert_eq!(push_notification_tokens[0].token_source, pnt1_request.token_source);
    assert_eq!(push_notification_tokens[0].token, pnt1_request.token);
    assert_eq!(push_notification_tokens[1].user_id, pnt2_request.user_id);
    assert_eq!(push_notification_tokens[1].token_source, pnt2_request.token_source);
    assert_eq!(push_notification_tokens[1].token, pnt2_request.token);
}

#[test]
fn from() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let push_notification_token = PushNotificationToken::create(user.id, "source".to_string(), "token".to_string())
        .commit(user.id, connection)
        .unwrap();
    let display_push_notification: DisplayPushNotificationToken = push_notification_token.clone().into();
    assert_eq!(
        display_push_notification,
        DisplayPushNotificationToken {
            id: push_notification_token.id,
            token_source: push_notification_token.token_source,
            token: push_notification_token.token,
            last_notification_at: push_notification_token.last_notification_at,
            created_at: push_notification_token.created_at,
        }
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let push_token = PushNotificationToken::create(user.id, "source".to_string(), "token".to_string())
        .commit(user.id, connection)
        .unwrap();

    assert_eq!(
        push_token,
        PushNotificationToken::find(push_token.id, connection).unwrap()
    );
}

#[test]
fn log_domain_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let push_token = PushNotificationToken::create(user.id, "source".to_string(), "token".to_string())
        .commit(user.id, connection)
        .unwrap();

    assert_eq!(
        DomainEvent::find(
            Tables::PushNotificationTokens,
            Some(push_token.id),
            Some(DomainEventTypes::PushNotificationTokenCreated),
            connection,
        )
        .unwrap()
        .len(),
        1
    );

    push_token.log_domain_event(user.id, connection).unwrap();
    assert_eq!(
        DomainEvent::find(
            Tables::PushNotificationTokens,
            Some(push_token.id),
            Some(DomainEventTypes::PushNotificationTokenCreated),
            connection,
        )
        .unwrap()
        .len(),
        2
    );
}

#[test]
fn remove_push_notification_token() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    //Create two push notification tokens for user
    let pnt1_request = NewPushNotificationToken {
        user_id: user.id,
        token_source: "example_token1_source".to_string(),
        token: "example_token1".to_string(),
    };
    let pnt2_request = NewPushNotificationToken {
        user_id: user.id,
        token_source: "example_token2_source".to_string(),
        token: "example_token2".to_string(),
    };
    pnt1_request.commit(user.id, connection).unwrap();
    pnt2_request.commit(user.id, connection).unwrap();
    let stored_push_notification_tokens = PushNotificationToken::find_by_user_id(user.id, connection).unwrap();
    //Remove first created token
    PushNotificationToken::remove(user.id, stored_push_notification_tokens[0].id, connection).unwrap();
    //Check stored push notification tokens
    let updated_push_notification_tokens = PushNotificationToken::find_by_user_id(user.id, connection).unwrap();
    assert_eq!(updated_push_notification_tokens.len(), 1);
    assert_eq!(
        updated_push_notification_tokens[0].user_id,
        stored_push_notification_tokens[1].user_id
    );
    assert_eq!(
        updated_push_notification_tokens[0].token_source,
        stored_push_notification_tokens[1].token_source
    );
    assert_eq!(
        updated_push_notification_tokens[0].token,
        stored_push_notification_tokens[1].token
    );
}
