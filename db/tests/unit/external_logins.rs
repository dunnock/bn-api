use db::dev::TestProject;
use db::prelude::*;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let site = "site".to_string();
    let access_token = "token".to_string();
    let external_user_id = "external-id".to_string();
    let scope = "user:read".to_string();

    let external_login = ExternalLogin::create(
        external_user_id.clone(),
        site.clone(),
        user.id,
        access_token.clone(),
        vec![scope.clone()],
    )
    .commit(Some(user.id), connection)
    .unwrap();
    assert_eq!(external_login.user_id, user.id);
    assert_eq!(external_login.external_user_id, external_user_id);
    assert_eq!(external_login.access_token, access_token);
    assert_eq!(external_login.site, site);
    assert_eq!(external_login.scopes, vec![scope]);

    assert_eq!(
        DomainEvent::find(
            Tables::ExternalLogins,
            Some(external_login.id),
            Some(DomainEventTypes::ExternalLoginCreated),
            connection,
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn find_for_site() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let site = "site".to_string();

    let external_login = ExternalLogin::create(
        "external-id".to_string(),
        site.clone(),
        user.id,
        "token".to_string(),
        vec!["user:read".to_string()],
    )
    .commit(Some(user.id), connection)
    .unwrap();

    let result = ExternalLogin::find_for_site(user.id, &site, connection).unwrap();
    assert_eq!(result, external_login);

    assert!(ExternalLogin::find_for_site(user.id, "fake", connection).is_err());
}

#[test]
fn find_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let site = "site".to_string();
    let site2 = "site2".to_string();
    let external_user_id = "external-id".to_string();
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site, connection).unwrap(),
        None
    );
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site2, connection).unwrap(),
        None
    );

    let external_login = ExternalLogin::create(
        external_user_id.clone(),
        site.clone(),
        user.id,
        "token".to_string(),
        vec!["user:read".to_string()],
    )
    .commit(Some(user.id), connection)
    .unwrap();
    let external_login2 = ExternalLogin::create(
        external_user_id.clone(),
        site2.clone(),
        user.id,
        "token".to_string(),
        vec!["user:read".to_string()],
    )
    .commit(Some(user.id), connection)
    .unwrap();
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site, connection).unwrap(),
        Some(external_login)
    );
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site2, connection).unwrap(),
        Some(external_login2)
    );
}

#[test]
fn delete() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let site = "site".to_string();
    let external_user_id = "external-id".to_string();

    let external_login = ExternalLogin::create(
        external_user_id.clone(),
        site.clone(),
        user.id,
        "token".to_string(),
        vec!["user:read".to_string()],
    )
    .commit(Some(user.id), connection)
    .unwrap();
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site, connection).unwrap(),
        Some(external_login.clone())
    );
    assert_eq!(
        DomainEvent::find(
            Tables::ExternalLogins,
            Some(external_login.id),
            Some(DomainEventTypes::ExternalLoginDeleted),
            connection,
        )
        .unwrap()
        .len(),
        0
    );

    external_login.clone().delete(Some(user.id), connection).unwrap();
    assert_eq!(
        ExternalLogin::find_user(&external_user_id, &site, connection).unwrap(),
        None
    );
    assert_eq!(
        DomainEvent::find(
            Tables::ExternalLogins,
            Some(external_login.id),
            Some(DomainEventTypes::ExternalLoginDeleted),
            connection,
        )
        .unwrap()
        .len(),
        1
    );
}
