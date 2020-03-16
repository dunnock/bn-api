use crate::error::FacebookError;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    ArtEvent,
    BookEvent,
    MovieEvent,
    Fundraiser,
    Volunteering,
    FamilyEvent,
    FestivalEvent,
    Neighborhood,
    ReligiousEvent,
    Shopping,
    ComedyEvent,
    MusicEvent,
    DanceEvent,
    Nightlife,
    TheaterEvent,
    DiningEvent,
    FoodTasting,
    ConferenceEvent,
    Meetup,
    ClassEvent,
    Lecture,
    Workshop,
    Fitness,
    SportsEvent,
    Other,
}

impl FromStr for Category {
    type Err = FacebookError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::Category::*;

        let c = match s {
            "MUSIC_EVENT" => MusicEvent,
            _ => {
                return Err(FacebookError::ParseError(format!("Invalid value encountered:{}", s)));
            }
        };
        Ok(c)
    }
}
