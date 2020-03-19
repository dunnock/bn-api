use chrono::prelude::*;
use db::dev::TestProject;
use db::models::*;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let settlement = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection)
    .unwrap();

    let settlement_adjustment = project
        .create_settlement_adjustment()
        .with_amount_in_cents(100)
        .with_settlement(&settlement)
        .finish();

    assert_eq!(settlement_adjustment.amount_in_cents, 100);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement_adjustment = project.create_settlement_adjustment().finish();
    let read_settlement_adjustment = SettlementAdjustment::find(settlement_adjustment.id, connection).unwrap();
    assert_eq!(settlement_adjustment.id, read_settlement_adjustment.id);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement_adjustment = project.create_settlement_adjustment().finish();
    settlement_adjustment.clone().destroy(connection).unwrap();
    assert!(SettlementAdjustment::find(settlement_adjustment.id, connection).is_err());
}
