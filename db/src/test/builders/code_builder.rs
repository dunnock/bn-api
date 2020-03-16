use chrono::prelude::*;
use chrono::Duration;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use models::*;
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use test::builders::*;
use uuid::Uuid;

pub struct CodeBuilder<'a> {
    name: String,
    redemption_code: String,
    event_id: Option<Uuid>,
    connection: &'a PgConnection,
    ticket_type_ids: Vec<Uuid>,
    code_type: CodeTypes,
    discount_in_cents: Option<u32>,
    discount_as_percentage: Option<u32>,
    max_uses: u32,
    max_tickets_per_user: Option<u32>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
}

impl<'a> CodeBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        let x: u32 = random();
        let redemption_code = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .collect::<String>()
            .to_uppercase();

        CodeBuilder {
            name: format!("Code {}", x).into(),
            redemption_code,
            connection,
            ticket_type_ids: Vec::new(),
            event_id: None,
            code_type: CodeTypes::Discount,
            discount_in_cents: Some(100),
            discount_as_percentage: None,
            max_tickets_per_user: None,
            max_uses: 30,
            start_date: NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2)),
            end_date: NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2)),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_start_date(mut self, start_date: NaiveDateTime) -> Self {
        self.start_date = start_date;
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDateTime) -> Self {
        self.end_date = end_date;
        self
    }

    pub fn with_max_tickets_per_user(mut self, max_tickets_per_user: Option<u32>) -> Self {
        self.max_tickets_per_user = max_tickets_per_user;
        self
    }

    pub fn with_max_uses(mut self, max_uses: u32) -> Self {
        self.max_uses = max_uses;
        self
    }

    pub fn with_discount_in_cents(mut self, discount_in_cents: Option<u32>) -> Self {
        self.discount_in_cents = discount_in_cents;
        self
    }

    pub fn with_redemption_code(mut self, redemption_code: String) -> Self {
        self.redemption_code = redemption_code;
        self
    }

    pub fn with_code_type(mut self, code_type: CodeTypes) -> Self {
        self.code_type = code_type;
        self
    }

    pub fn for_ticket_type(mut self, ticket_type: &TicketType) -> Self {
        self.ticket_type_ids.push(ticket_type.id);
        self
    }

    pub fn with_event(mut self, event: &Event) -> Self {
        self.event_id = Some(event.id);
        self
    }

    pub fn finish(mut self) -> Code {
        if self.event_id.is_none() {
            if self.ticket_type_ids.len() > 0 {
                self.event_id = Some(
                    TicketType::find(self.ticket_type_ids[0], self.connection)
                        .unwrap()
                        .event_id,
                );
            } else {
                self.event_id = Some(
                    EventBuilder::new(self.connection)
                        .with_ticket_pricing()
                        .with_tickets()
                        .finish()
                        .id,
                );
            }
        }

        let code = Code::create(
            self.name,
            self.event_id.unwrap(),
            self.code_type,
            self.redemption_code,
            self.max_uses,
            self.discount_in_cents,
            self.discount_as_percentage,
            self.start_date,
            self.end_date,
            self.max_tickets_per_user,
        )
        .commit(None, self.connection)
        .unwrap();

        for ticket_type_id in self.ticket_type_ids {
            TicketTypeCode::create(ticket_type_id, code.id)
                .commit(self.connection)
                .unwrap();
        }

        code
    }
}
