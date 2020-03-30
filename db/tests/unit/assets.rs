use db::dev::TestProject;
use db::prelude::*;
use db::schema::ticket_instances;

use chrono::NaiveDate;
use diesel::prelude::*;
use uuid::Uuid;

#[test]
fn update_asset_blockchain_id() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
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
            TicketTypeType::Token,
            vec![],
            None,
            None,
            None,
            None,
            conn,
        )
        .unwrap();

    let asset = Asset::find_by_ticket_type(ticket_type.id, conn).unwrap();
    let tari_asset_id = Uuid::new_v4().to_string();
    let asset = asset.update_blockchain_id(tari_asset_id.clone(), conn).unwrap();

    assert_eq!(asset.blockchain_asset_id, Some(tari_asset_id));
}

#[test]
fn asset_find() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();

    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);

    event
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
            TicketTypeType::Token,
            vec![],
            None,
            None,
            None,
            None,
            conn,
        )
        .unwrap();

    let ticket_instance = ticket_instances::table
        .filter(ticket_instances::wallet_id.eq(wallet_id))
        .first::<TicketInstance>(conn)
        .unwrap();

    let asset = Asset::find(ticket_instance.asset_id, conn).unwrap();

    assert_eq!(asset.id, ticket_instance.asset_id);

    let found_wallet = Wallet::find(wallet_id, conn).unwrap();
    assert_eq!(found_wallet.id, wallet_id);
}
