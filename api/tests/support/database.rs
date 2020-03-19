use api::config::Config;
use api::database::Connection as DbConnection;
use db::dev::*;
use db::prelude::*;

use diesel::Connection;
use diesel::PgConnection;
use std::error::Error;
use uuid::Uuid;

#[derive(Clone)]
pub struct TestDatabase {
    pub connection: DbConnection,
}

#[allow(dead_code)]
impl TestDatabase {
    pub fn new() -> TestDatabase {
        let config = Config::new(Environment::Test);

        let connection = PgConnection::establish(&config.database_url).unwrap_or_else(|e| {
            panic!(
                "Connection to {} could not be established:{}",
                config.database_url,
                e.description()
            )
        });

        connection.begin_test_transaction().unwrap();

        TestDatabase {
            connection: connection.into(),
        }
    }

    pub fn create_organization_with_user(&self, user: &User, owner: bool) -> OrganizationBuilder {
        let organization_builder = self.create_organization();
        if owner {
            organization_builder.with_member(&user, Roles::OrgOwner)
        } else {
            organization_builder.with_member(&user, Roles::OrgMember)
        }
    }

    pub fn create_announcement(&self) -> AnnouncementBuilder {
        AnnouncementBuilder::new(self.connection.get())
    }

    pub fn create_announcement_engagement(&self) -> AnnouncementEngagementBuilder {
        AnnouncementEngagementBuilder::new(self.connection.get())
    }

    pub fn create_artist(&self) -> ArtistBuilder {
        ArtistBuilder::new(self.connection.get())
    }

    pub fn create_cart(&self) -> OrderBuilder {
        OrderBuilder::new(self.connection.get())
    }

    pub fn create_code(&self) -> CodeBuilder {
        CodeBuilder::new(self.connection.get())
    }

    pub fn create_comp(&self) -> CompBuilder {
        CompBuilder::new(self.connection.get())
    }

    pub fn create_event(&self) -> EventBuilder {
        EventBuilder::new(self.connection.get())
    }

    pub fn create_domain_action(&self) -> DomainActionBuilder {
        DomainActionBuilder::new(self.connection.get())
    }

    pub fn create_domain_event_publisher(&self) -> DomainEventPublisherBuilder {
        DomainEventPublisherBuilder::new(self.connection.get())
    }

    pub fn create_hold(&self) -> HoldBuilder {
        HoldBuilder::new(self.connection.get())
    }

    pub fn create_note(&self) -> NoteBuilder {
        NoteBuilder::new(self.connection.get())
    }

    pub fn create_order(&self) -> OrderBuilder {
        OrderBuilder::new(self.connection.get())
    }

    pub fn create_refund(&self) -> RefundBuilder {
        RefundBuilder::new(self.connection.get())
    }

    pub fn create_organization(&self) -> OrganizationBuilder {
        OrganizationBuilder::new(self.connection.get())
    }

    pub fn create_organization_invite(&self) -> OrgInviteBuilder {
        OrgInviteBuilder::new(self.connection.get())
    }

    pub fn create_payment_method(&self) -> PaymentMethodBuilder {
        PaymentMethodBuilder::new(self.connection.get())
    }

    pub fn create_slug(&self) -> SlugBuilder {
        SlugBuilder::new(self.connection.get())
    }

    pub fn create_region(&self) -> RegionBuilder {
        RegionBuilder::new(self.connection.get())
    }

    pub fn create_user(&self) -> UserBuilder {
        UserBuilder::new(self.connection.get())
    }

    pub fn create_venue(&self) -> VenueBuilder {
        VenueBuilder::new(self.connection.get())
    }

    pub fn create_event_artist(&self) -> EventArtistBuilder {
        EventArtistBuilder::new(self.connection.get())
    }

    pub fn create_event_interest(&self) -> EventInterestBuilder {
        EventInterestBuilder::new(self.connection.get())
    }

    pub fn create_event_report_subscriber(&self) -> EventReportSubscriberBuilder {
        EventReportSubscriberBuilder::new(self.connection.get())
    }

    pub fn create_stage(&self) -> StageBuilder {
        StageBuilder::new(self.connection.get())
    }

    pub fn create_settlement_entry(&self) -> SettlementEntryBuilder {
        SettlementEntryBuilder::new(self.connection.get())
    }

    pub fn create_settlement_adjustment(&self) -> SettlementAdjustmentBuilder {
        SettlementAdjustmentBuilder::new(self.connection.get())
    }

    pub fn create_settlement(&self) -> SettlementBuilder {
        SettlementBuilder::new(self.connection.get())
    }

    pub fn create_fee_schedule(&self) -> FeeScheduleBuilder {
        FeeScheduleBuilder::new(self.connection.get())
    }

    pub fn create_purchased_tickets(&self, user: &User, ticket_type_id: Uuid, quantity: u32) -> Vec<TicketInstance> {
        let mut cart = Order::find_or_create_cart(user, self.connection.get()).unwrap();
        cart.update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id,
                quantity,
                redemption_code: None,
            }],
            false,
            false,
            self.connection.get(),
        )
        .unwrap();

        let total = cart.calculate_total(self.connection.get()).unwrap();
        cart.add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            self.connection.get(),
        )
        .unwrap();
        TicketInstance::find_for_user(user.id, self.connection.get()).unwrap()
    }
}
