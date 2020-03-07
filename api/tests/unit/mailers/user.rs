use bigneon_api::communications::mailers;
use bigneon_api::config::Config;
use bigneon_db::models::concerns::users::password_resetable::PasswordResetable;
use bigneon_db::models::{CommAddress, Environment};
use crate::support::database::TestDatabase;

#[test]
fn password_reset_email() {
    let config = Config::new(Environment::Test);
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let user = user.create_password_reset_token(database.connection.get()).unwrap();

    let password_reset_email = mailers::user::password_reset_email(&config, &user);
    assert_eq!(
        password_reset_email.destinations,
        CommAddress::from(user.email.unwrap().to_string())
    );
    assert_eq!(
        password_reset_email.source,
        Some(CommAddress::from("noreply@bigneon.com".to_string()))
    );
}
