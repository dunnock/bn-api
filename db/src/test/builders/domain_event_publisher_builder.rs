use diesel::prelude::*;
use models::*;
use uuid::Uuid;

pub struct DomainEventPublisherBuilder<'a> {
    organization_id: Option<Uuid>,
    event_types: Option<Vec<DomainEventTypes>>,
    webhook_url: Option<String>,
    connection: &'a PgConnection,
}

impl<'a> DomainEventPublisherBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        DomainEventPublisherBuilder {
            organization_id: None,
            event_types: None,
            webhook_url: None,
            connection,
        }
    }

    pub fn with_organization(mut self, organization: &Organization) -> Self {
        self.organization_id = Some(organization.id);
        self
    }

    pub fn with_event_types(mut self, event_types: Vec<DomainEventTypes>) -> Self {
        self.event_types = Some(event_types);
        self
    }

    pub fn with_webhook_url(mut self, webhook_url: String) -> Self {
        self.webhook_url = Some(webhook_url);
        self
    }

    pub fn finish(self) -> DomainEventPublisher {
        DomainEventPublisher::create(
            self.organization_id,
            self.event_types.unwrap_or(vec![DomainEventTypes::OrderCreated]),
            self.webhook_url.unwrap_or("https://www.tari.com".to_string()),
        )
        .commit(self.connection)
        .unwrap()
    }
}
