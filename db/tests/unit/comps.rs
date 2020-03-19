use db::dev::TestProject;
use db::models::*;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn create() {
    let db = TestProject::new();
    let hold = db.create_hold().with_hold_type(HoldTypes::Comp).finish();
    Hold::create_comp_for_person(
        "test".into(),
        None,
        hold.id,
        Some("email@address.com".into()),
        None,
        "redemption".to_string(),
        None,
        None,
        5,
        db.get_connection(),
    )
    .unwrap();
}

#[test]
pub fn create_with_validation_errors() {
    let db = TestProject::new();
    let hold = db.create_hold().with_hold_type(HoldTypes::Discount).finish();
    let result = Hold::create_comp_for_person(
        "test".into(),
        None,
        hold.id,
        Some("invalid".into()),
        None,
        "redemp".to_string(),
        None,
        None,
        11,
        db.get_connection(),
    );

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("email"));
                assert_eq!(errors["email"].len(), 1);
                assert_eq!(errors["email"][0].code, "email");
                assert_eq!(
                    &errors["email"][0].message.clone().unwrap().into_owned(),
                    "Email is invalid"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update() {
    let db = TestProject::new();
    let comp = db.create_comp().finish();

    let update_patch = UpdateHoldAttributes {
        name: Some("New name".to_string()),
        email: Some(Some("new@email.com".to_string())),
        ..Default::default()
    };
    let new_comp = comp.update(update_patch, db.get_connection()).unwrap();
    assert_eq!(new_comp.name, "New name".to_string());
    assert_eq!(new_comp.email, Some("new@email.com".to_string()));
}

#[test]
pub fn update_with_validation_errors() {
    let db = TestProject::new();
    let comp = db.create_comp().finish();

    let update_patch = UpdateHoldAttributes {
        email: Some(Some("invalid".to_string())),
        ..Default::default()
    };

    let result = comp.update(update_patch, db.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("email"));
                assert_eq!(errors["email"].len(), 1);
                assert_eq!(errors["email"][0].code, "email");
                assert_eq!(
                    &errors["email"][0].message.clone().unwrap().into_owned(),
                    "Email is invalid"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    let result = comp.set_quantity(None, 11, db.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("quantity"));
                assert_eq!(errors["quantity"].len(), 1);
                assert_eq!(
                    errors["quantity"][0].code,
                    "Could not reserve tickets, not enough tickets are available"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_by_redemption_code() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let comp = db.create_comp().finish();
    let found_comp = Hold::find_by_redemption_code(&comp.redemption_code.clone().unwrap(), None, connection).unwrap();
    assert_eq!(comp, found_comp);
}

#[test]
fn find_for_hold() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let hold1 = db.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let hold2 = db.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let hold3 = db.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let comp1 = db
        .create_comp()
        .with_hold(&hold1)
        .with_quantity(3)
        .with_name("Comp1".into())
        .finish();
    let comp2 = db
        .create_comp()
        .with_hold(&hold1)
        .with_quantity(1)
        .with_name("Comp2".into())
        .finish();
    let _comp3 = db.create_comp().with_hold(&hold2).with_name("Comp3".into()).finish();

    let update_patch = UpdateHoldAttributes {
        hold_type: Some(HoldTypes::Discount),
        discount_in_cents: Some(Some(0)),
        ..Default::default()
    };
    let _hold2 = hold2.update(update_patch, connection).unwrap();

    let found_comps = Hold::find_by_parent_id(hold1.id, Some(HoldTypes::Comp), 0, 1000, connection).unwrap();
    assert_eq!(vec![comp1, comp2], found_comps.data);

    let found_comps = Hold::find_by_parent_id(hold3.id, Some(HoldTypes::Comp), 0, 1000, connection).unwrap();
    assert!(found_comps.is_empty());
}
