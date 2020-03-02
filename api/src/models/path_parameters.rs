use uuid::Uuid;

#[derive(Deserialize, Debug, PartialEq)]
pub struct PathParameters {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct OptionalPathParameters {
    pub id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ExternalPathParameters {
    pub id: String,
}

#[derive(Deserialize)]
pub struct StringPathParameters {
    pub id: String,
}

#[derive(Deserialize)]
pub struct MainTablePathParameters {
    // Can't use enum currently, see https://github.com/actix/actix-web/issues/318
    pub main_table: String,
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct EventTicketPathParameters {
    pub event_id: Uuid,
    pub ticket_type_id: Uuid,
}

#[derive(Deserialize)]
pub struct OrganizationFanPathParameters {
    pub id: Uuid, // Organization Id
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct OrganizationUserPathParameters {
    pub id: Uuid, // Organization Id
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct OrganizationInvitePathParameters {
    pub id: Uuid, // Organization Id
    pub invite_id: Uuid,
}

#[derive(Deserialize)]
pub struct CompPathParameters {
    pub hold_id: Uuid,
    pub comp_id: Uuid,
}
