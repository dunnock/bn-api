pub struct ChargeAuthResult {
    pub id: String,
    pub raw: String,
}

impl ChargeAuthResult {
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.raw)
    }
}
