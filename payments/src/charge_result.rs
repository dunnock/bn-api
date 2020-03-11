#[derive(Debug)]
pub struct ChargeResult {
    pub id: String,
    pub raw: String,
}

impl ChargeResult {
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.raw)
    }
}
