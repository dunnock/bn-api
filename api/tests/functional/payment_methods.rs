use crate::support;
use crate::support::database::TestDatabase;
use actix_web::{http::StatusCode, HttpResponse};
use bigneon_api::controllers::payment_methods;
use bigneon_db::models::{DisplayPaymentMethod, PaymentProviders, Roles};
use serde_json;

#[actix_rt::test]
async fn index() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let payment_method = database
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    let payment_method2 = database
        .create_payment_method()
        .with_name(PaymentProviders::Stripe)
        .with_user(&user)
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let expected_payment_methods: Vec<DisplayPaymentMethod> = vec![payment_method.into(), payment_method2.into()];
    let payment_methods_expected_json = serde_json::to_string(&expected_payment_methods).unwrap();
    let response: HttpResponse = payment_methods::index((database.connection.into(), auth_user))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, payment_methods_expected_json);
}
