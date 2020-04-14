use chrono::prelude::*;
use diesel::prelude::*;
use models::TicketTypeVisibility;
use prelude::*;
use std::cmp;
use test::builders::*;
use test::times;
use utils::dates::IntoDateBuilder;
use utils::rand::random_alpha_string;
use uuid::Uuid;

pub struct EventBuilder<'a> {
    name: String,
    status: EventStatus,
    organization_id: Option<Uuid>,
    venue_id: Option<Uuid>,
    event_start: Option<NaiveDateTime>,
    event_end: Option<NaiveDateTime>,
    door_time: Option<NaiveDateTime>,
    pub connection: &'a PgConnection,
    with_tickets: bool,
    with_ticket_pricing: bool,
    ticket_quantity: u32,
    ticket_type_count: i64,
    is_external: bool,
    publish_date: Option<NaiveDateTime>,
    sales_start: Option<NaiveDateTime>,
    sales_end: Option<NaiveDateTime>,
    private_access_code: Option<String>,
    event_type: Option<EventTypes>,
    additional_info: Option<String>,
    top_line_info: Option<String>,
}

impl<'a> EventBuilder<'a> {
    pub fn new(connection: &PgConnection) -> EventBuilder {
        EventBuilder {
            name: format!("Event {}", random_alpha_string(4)),
            status: EventStatus::Published,
            organization_id: None,
            venue_id: None,
            event_start: None,
            event_end: None,
            door_time: None,
            connection,
            with_tickets: false,
            with_ticket_pricing: false,
            ticket_quantity: 100,
            ticket_type_count: 1,
            is_external: false,
            publish_date: Some(NaiveDate::from_ymd(2018, 7, 8).and_hms(9, 10, 11)),
            private_access_code: None,
            sales_start: None,
            sales_end: None,
            event_type: None,
            additional_info: None,
            top_line_info: None,
        }
    }

    pub fn with_ticket_type_count(mut self, ticket_type_count: i64) -> Self {
        self.ticket_type_count = ticket_type_count;
        self
    }

    pub fn external(mut self) -> Self {
        self.is_external = true;
        self
    }

    pub fn with_additional_info(mut self, additional_info: String) -> Self {
        self.additional_info = Some(additional_info);
        self
    }

    pub fn as_private(mut self, private_access_code: String) -> Self {
        self.private_access_code = Some(private_access_code);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_event_type(mut self, event_type: EventTypes) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_status(mut self, status: EventStatus) -> Self {
        if status != EventStatus::Published {
            self.publish_date = None;
        }
        self.status = status;
        self
    }

    pub fn with_organization(mut self, organization: &Organization) -> Self {
        self.organization_id = Some(organization.id.clone());
        self
    }

    pub fn with_venue(mut self, venue: &Venue) -> Self {
        self.venue_id = Some(venue.id.clone());
        self
    }

    pub fn with_door_time(mut self, date: NaiveDateTime) -> Self {
        self.door_time = Some(date);
        self
    }

    pub fn with_event_start(mut self, date: NaiveDateTime) -> Self {
        self.event_start = Some(date);
        self
    }

    pub fn with_event_end(mut self, date: NaiveDateTime) -> Self {
        self.event_end = Some(date);
        self
    }

    pub fn with_publish_date(mut self, date: NaiveDateTime) -> Self {
        self.publish_date = Some(date);
        self
    }

    pub fn with_tickets(mut self) -> Self {
        self.with_tickets = true;
        self
    }

    pub fn with_a_specific_number_of_tickets(mut self, num: u32) -> Self {
        self.with_tickets = true;
        self.ticket_quantity = num;
        self
    }

    pub fn with_ticket_pricing(mut self) -> Self {
        self.with_tickets = true;
        self.with_ticket_pricing = true;
        self
    }

    pub fn with_top_line_info(mut self, top_line_info: Option<String>) -> Self {
        self.top_line_info = top_line_info;
        self
    }

    pub fn with_sales_starting(mut self, time: NaiveDateTime) -> Self {
        self.sales_start = Some(time);
        self
    }

    pub fn with_sales_ending(mut self, time: NaiveDateTime) -> Self {
        self.sales_end = Some(time);
        self
    }

    pub fn with_ticket_type(self) -> TicketTypeBuilder<'a> {
        TicketTypeBuilder::new(self)
    }

    pub fn finish(&mut self) -> Event {
        let organization_id = self
            .organization_id
            .or_else(|| Some(OrganizationBuilder::new(self.connection).finish().id))
            .unwrap();
        let event_start = self.event_start.unwrap_or(dates::now().add_days(2).finish());
        let event_end = self
            .event_end
            .unwrap_or(event_start.into_builder().add_days(2).finish());

        let event = Event::create(
            &self.name,
            self.status,
            organization_id,
            self.venue_id,
            Some(event_start),
            self.door_time
                .or_else(|| Some(event_start.into_builder().add_hours(-1).finish())),
            self.publish_date,
            Some(event_end),
        )
        .commit(None, self.connection)
        .unwrap();

        let mut attributes = EventEditableAttributes {
            promo_image_url: Some(Some("http://localhost".to_string())),
            additional_info: Some(self.additional_info.clone()),
            ..Default::default()
        };
        if self.private_access_code.is_some() {
            attributes.private_access_code = Some(self.private_access_code.clone());
        }

        if self.event_type.is_some() {
            attributes.event_type = self.event_type.clone();
        }

        if self.is_external {
            attributes.is_external = Some(true);
        }

        let event = event.update(None, attributes, self.connection).unwrap();

        if self.with_tickets {
            let early_bird_start = cmp::max(
                self.sales_start.unwrap_or(times::zero()),
                dates::now().add_days(-2).finish(),
            );
            let early_bird_end = cmp::min(
                self.sales_end.unwrap_or(times::infinity()),
                dates::now().add_days(-1).finish(),
            );
            let standard_start = cmp::max(
                self.sales_start.unwrap_or(times::zero()),
                dates::now().add_days(-1).finish(),
            );
            let standard_end = cmp::min(
                self.sales_end.unwrap_or(times::infinity()),
                dates::now().add_days(2).finish(),
            );

            // TODO: The times should actually be linked to the event start date,
            // but there are many tests that need to be updated to allow this
            let sales_start = event_start.into_builder().add_days(-4).finish();
            //            // No active pricing gap
            //            let early_bird_start = event_start.into_builder().add_days(-4).finish();
            //            let early_bird_end = event_start.into_builder().add_days(-3).finish();
            //            let standard_start = event_start.into_builder().add_days(-3).finish();
            //            let standard_end = event_start.into_builder().add_hours(-3).finish();
            //            // No active pricing gap
            let sales_end = event_start.into_builder().add_hours(-2).finish();

            // No sales an hour before the event

            let wallet_id = event.issuer_wallet(self.connection).unwrap().id;

            for x in 0..self.ticket_type_count {
                let ticket_type = event
                    .add_ticket_type(
                        format!("Ticket Type {}", x),
                        None,
                        self.ticket_quantity,
                        Some(self.sales_start.unwrap_or(sales_start)),
                        Some(self.sales_end.unwrap_or(sales_end)),
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
                        self.connection,
                    )
                    .unwrap();

                if self.with_ticket_pricing {
                    ticket_type
                        .add_ticket_pricing(
                            "Early bird".into(),
                            early_bird_start,
                            early_bird_end,
                            100,
                            false,
                            None,
                            None,
                            self.connection,
                        )
                        .unwrap();

                    ticket_type
                        .add_ticket_pricing(
                            "Standard".into(),
                            standard_start,
                            standard_end,
                            150,
                            false,
                            None,
                            None,
                            self.connection,
                        )
                        .unwrap();
                }

                Asset::find_by_ticket_type(ticket_type.id, self.connection)
                    .unwrap()
                    .update_blockchain_id(
                        format!("{}.{}", event.name, ticket_type.name).to_string(),
                        self.connection,
                    )
                    .unwrap();
            }
        }

        event
    }
}
