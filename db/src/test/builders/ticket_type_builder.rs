use chrono::NaiveDateTime;
use models::*;
use test::builders::event_builder::EventBuilder;
use utils::dates::IntoDateBuilder;

pub struct TicketTypeBuilder<'a> {
    event: EventBuilder<'a>,
    quantity: u32,
    price_in_cents: Option<i64>,
    name: Option<String>,
    sales_start: Option<NaiveDateTime>,
    sales_end: Option<NaiveDateTime>,
    visibility: TicketTypeVisibility,
    additional_fees: i64,
    end_date_type: TicketTypeEndDateType,
}

impl<'a> TicketTypeBuilder<'a> {
    pub fn new(event: EventBuilder<'a>) -> TicketTypeBuilder<'a> {
        TicketTypeBuilder {
            event,
            quantity: 100,
            price_in_cents: None,
            name: None,
            sales_start: None,
            sales_end: None,
            visibility: TicketTypeVisibility::Always,
            additional_fees: 0,
            end_date_type: TicketTypeEndDateType::Manual,
        }
    }

    pub fn starting(mut self, date: NaiveDateTime) -> Self {
        self.sales_start = Some(date);
        self
    }

    pub fn ending(mut self, date: NaiveDateTime) -> Self {
        self.sales_end = Some(date);
        self.end_date_type = TicketTypeEndDateType::Manual;
        self
    }

    pub fn visibility(mut self, visibility: TicketTypeVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_pricing(self) -> TicketPricingBuilder<'a> {
        TicketPricingBuilder::new(self)
    }

    pub fn with_price(mut self, price_in_cents: i64) -> Self {
        self.price_in_cents = Some(price_in_cents);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_additional_fees(mut self, amount: i64) -> Self {
        self.additional_fees = amount;
        self
    }

    pub fn finish(mut self) -> Event {
        let connection = self.event.connection;
        let event = self.event.finish();
        let sales_start = event.event_start.unwrap().into_builder().add_days(-4).finish();
        let sales_end = if self.end_date_type != TicketTypeEndDateType::Manual {
            None
        } else {
            Some(
                self.sales_end
                    .unwrap_or(event.event_start.unwrap().into_builder().add_hours(-2).finish()),
            )
        };
        event
            .add_ticket_type(
                self.name.unwrap_or("Ticket Builder".to_string()),
                None,
                self.quantity,
                Some(self.sales_start.unwrap_or(sales_start)),
                sales_end,
                self.end_date_type,
                Some(event.issuer_wallet(self.event.connection).unwrap().id),
                None,
                0,
                self.price_in_cents.unwrap_or(100),
                self.visibility,
                None,
                self.additional_fees,
                true,
                true,
                true,
                TicketTypeType::Token,
                vec![],
                None,
                None,
                None,
                None,
                connection,
            )
            .unwrap();

        event
    }
}

pub struct TicketPricingBuilder<'a> {
    ticket_type: TicketTypeBuilder<'a>,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
}

impl<'a> TicketPricingBuilder<'a> {
    pub fn new(ticket_type: TicketTypeBuilder<'a>) -> TicketPricingBuilder<'a> {
        TicketPricingBuilder {
            ticket_type,
            start: None,
            end: None,
        }
    }
    pub fn starting(mut self, date: NaiveDateTime) -> Self {
        self.start = Some(date);
        self
    }
    pub fn ending(mut self, date: NaiveDateTime) -> Self {
        self.end = Some(date);
        self
    }
    pub fn finish(self) -> Event {
        let connection = self.ticket_type.event.connection;
        let event = self.ticket_type.finish();

        let ticket_type = event.ticket_types(false, None, connection).unwrap().pop().unwrap();
        ticket_type
            .add_ticket_pricing(
                "Ticket Pricing Builder".to_string(),
                self.start.unwrap_or(ticket_type.start_date(connection).unwrap()),
                self.end.unwrap_or(ticket_type.end_date(connection).unwrap()),
                10,
                false,
                None,
                None,
                connection,
            )
            .unwrap();
        event
    }
}
