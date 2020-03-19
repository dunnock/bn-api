use db::dev::TestProject;
use db::prelude::*;
use uuid::Uuid;

#[test]
fn fee_schedule_create() {
    let project = TestProject::new();
    let fee_schedule = FeeSchedule::create(
        Uuid::nil(),
        "default".to_string(),
        vec![
            NewFeeScheduleRange {
                min_price_in_cents: 20,
                company_fee_in_cents: 4,
                client_fee_in_cents: 6,
            },
            NewFeeScheduleRange {
                min_price_in_cents: 1000,
                company_fee_in_cents: 40,
                client_fee_in_cents: 60,
            },
        ],
    )
    .commit(None, project.get_connection())
    .unwrap();

    let ranges = fee_schedule.ranges(project.get_connection()).unwrap();
    assert_eq!(
        vec![ranges[0].min_price_in_cents, ranges[1].min_price_in_cents],
        vec![20, 1000]
    );
    assert_eq!(vec![ranges[0].fee_in_cents, ranges[1].fee_in_cents], vec![10, 100]);

    let fee_schedule2 = FeeSchedule::create(
        Uuid::nil(),
        "default".to_string(),
        vec![
            NewFeeScheduleRange {
                min_price_in_cents: 20,
                company_fee_in_cents: 4,
                client_fee_in_cents: 6,
            },
            NewFeeScheduleRange {
                min_price_in_cents: 1000,
                company_fee_in_cents: 40,
                client_fee_in_cents: 60,
            },
        ],
    )
    .commit(None, project.get_connection())
    .unwrap();

    assert_eq!(fee_schedule2.version, 1);
}

#[test]
fn get_fee_schedule_range() {
    let project = TestProject::new();
    let fee_schedule = FeeSchedule::create(
        Uuid::nil(),
        "default".to_string(),
        vec![
            NewFeeScheduleRange {
                min_price_in_cents: 20,
                company_fee_in_cents: 4,
                client_fee_in_cents: 6,
            },
            NewFeeScheduleRange {
                min_price_in_cents: 100,
                company_fee_in_cents: 8,
                client_fee_in_cents: 12,
            },
        ],
    )
    .commit(None, project.get_connection())
    .unwrap();

    let fee_schedule_range1 = fee_schedule.get_range(30, project.get_connection()).unwrap();
    let fee_schedule_range2 = fee_schedule.get_range(150, project.get_connection()).unwrap();
    let fee_schedule_range3 = fee_schedule.get_range(10, project.get_connection());

    assert_eq!(fee_schedule_range1.fee_in_cents, 10);
    assert_eq!(fee_schedule_range2.fee_in_cents, 20);
    assert!(fee_schedule_range3.is_err());
}
