use diesel::dsl::sql;
use diesel::sql_types::Bool;
use diesel::{select, Connection, PgConnection, RunQueryDsl};
use std::env;
use test::builders::*;
use test::dotenv::dotenv;

pub struct TestProject {
    pub connection: PgConnection,
    admin: PgConnection,
}

#[allow(dead_code)]
impl TestProject {
    pub fn new() -> Self {
        dotenv().ok();
        let conn_str = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be defined.");
        let admin_str = env::var("TEST_DATABASE_ADMIN_URL").expect("TEST_DATABASE_ADMIN_URL must be defined.");
        let connection = PgConnection::establish(&conn_str).expect("Could not get access to test database");
        let admin = PgConnection::establish(&admin_str).expect("Could not get admin access to admin test database");
        connection
            .begin_test_transaction()
            .expect("Could not start testing transaction");
        TestProject { connection, admin }
    }

    pub fn new_without_rollback() -> Self {
        dotenv().ok();
        let conn_str = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be defined.");
        let admin_str = env::var("TEST_DATABASE_ADMIN_URL").expect("TEST_DATABASE_ADMIN_URL must be defined.");
        let connection = PgConnection::establish(&conn_str).expect("Could not get access to test database");
        let admin = PgConnection::establish(&admin_str).expect("Could not get admin access to admin test database");

        TestProject { connection, admin }
    }

    pub fn db_exists(&self, name: &str) -> bool {
        select(sql::<Bool>(&format!(
            "EXISTS(SELECT 1 FROM pg_database WHERE datname='{}')",
            name
        )))
        .get_result(&self.admin)
        .unwrap()
    }

    pub fn table_exists(&self, table: &str) -> bool {
        select(sql::<Bool>(&format!(
            "EXISTS \
             (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = '{}')",
            table
        )))
        .get_result(&self.admin)
        .unwrap()
    }

    pub fn create_announcement(&self) -> AnnouncementBuilder {
        AnnouncementBuilder::new(&self.connection)
    }

    pub fn create_announcement_engagement(&self) -> AnnouncementEngagementBuilder {
        AnnouncementEngagementBuilder::new(&self.connection)
    }

    pub fn create_artist(&self) -> ArtistBuilder {
        ArtistBuilder::new(&self.connection)
    }

    pub fn create_broadcast(&self) -> BroadcastBuilder {
        BroadcastBuilder::new(&self.connection)
    }

    pub fn create_code(&self) -> CodeBuilder {
        CodeBuilder::new(&self.connection)
    }

    pub fn create_comp(&self) -> CompBuilder {
        CompBuilder::new(&self.connection)
    }

    pub fn create_domain_action(&self) -> DomainActionBuilder {
        DomainActionBuilder::new(&self.connection)
    }

    pub fn create_domain_event_publisher(&self) -> DomainEventPublisherBuilder {
        DomainEventPublisherBuilder::new(&self.connection)
    }

    pub fn create_event(&self) -> EventBuilder {
        EventBuilder::new(&self.connection)
    }

    pub fn create_genre(&self) -> GenreBuilder {
        GenreBuilder::new(&self.connection)
    }

    pub fn create_hold(&self) -> HoldBuilder {
        HoldBuilder::new(&self.connection)
    }

    pub fn create_note(&self) -> NoteBuilder {
        NoteBuilder::new(&self.connection)
    }

    pub fn create_order(&self) -> OrderBuilder {
        OrderBuilder::new(&self.connection)
    }

    pub fn create_refund(&self) -> RefundBuilder {
        RefundBuilder::new(&self.connection)
    }

    pub fn create_organization(&self) -> OrganizationBuilder {
        OrganizationBuilder::new(&self.connection)
    }

    pub fn create_organization_invite(&self) -> OrgInviteBuilder {
        OrgInviteBuilder::new(&self.connection)
    }

    pub fn create_payment_method(&self) -> PaymentMethodBuilder {
        PaymentMethodBuilder::new(&self.connection)
    }

    pub fn create_payment(&self) -> PaymentBuilder {
        PaymentBuilder::new(&self.connection)
    }

    pub fn create_region(&self) -> RegionBuilder {
        RegionBuilder::new(&self.connection)
    }

    pub fn create_slug(&self) -> SlugBuilder {
        SlugBuilder::new(&self.connection)
    }

    pub fn create_user(&self) -> UserBuilder {
        UserBuilder::new(&self.connection)
    }

    pub fn create_venue(&self) -> VenueBuilder {
        VenueBuilder::new(&self.connection)
    }

    pub fn create_stage(&self) -> StageBuilder {
        StageBuilder::new(&self.connection)
    }

    pub fn create_event_artist(&self) -> EventArtistBuilder {
        EventArtistBuilder::new(&self.connection)
    }

    pub fn create_event_interest(&self) -> EventInterestBuilder {
        EventInterestBuilder::new(&self.connection)
    }

    pub fn create_event_report_subscriber(&self) -> EventReportSubscriberBuilder {
        EventReportSubscriberBuilder::new(&self.connection)
    }

    pub fn create_fee_schedule(&self) -> FeeScheduleBuilder {
        FeeScheduleBuilder::new(&self.connection)
    }

    pub fn create_settlement_entry(&self) -> SettlementEntryBuilder {
        SettlementEntryBuilder::new(&self.connection)
    }

    pub fn create_settlement_adjustment(&self) -> SettlementAdjustmentBuilder {
        SettlementAdjustmentBuilder::new(&self.connection)
    }

    pub fn create_settlement(&self) -> SettlementBuilder {
        SettlementBuilder::new(&self.connection)
    }

    pub fn get_connection(&self) -> &PgConnection {
        &self.connection
    }
}
