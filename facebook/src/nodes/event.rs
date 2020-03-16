use crate::fbid::FBID;
use crate::nodes::Category;
use crate::nodes::CoverPhoto;

#[derive(Serialize, Debug)]
pub struct Event {
    pub category: Category,
    pub name: String,
    pub description: String,
    pub place_id: Option<FBID>,
    pub timezone: String,
    pub cover: Option<CoverPhoto>,
    pub start_time: String,
    pub ticket_uri: Option<String>,
    pub address: Option<String>,
    pub admins: Vec<String>,
}

pub enum EventRole {}

impl Event {
    pub fn new(
        category: Category,
        name: String,
        description: String,
        timezone: String,
        cover: Option<CoverPhoto>,
        start_time: String,
    ) -> Event {
        Event {
            category,
            name,
            description,
            place_id: None,
            timezone,
            cover,
            start_time,
            ticket_uri: None,
            address: None,
            admins: Vec::new(),
        }
    }
}
