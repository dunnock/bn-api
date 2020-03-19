use db::models::PastOrUpcoming;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PastOrUpcomingParameters {
    pub past_or_upcoming: Option<PastOrUpcoming>,
}
