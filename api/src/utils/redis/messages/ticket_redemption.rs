use uuid::Uuid;

#[derive(Deserialize, Default, Serialize)]
pub struct TicketRedemption {
    pub ticket_id: Uuid,
    pub redeemer_id: Uuid,
    pub event_id: Uuid,
}
