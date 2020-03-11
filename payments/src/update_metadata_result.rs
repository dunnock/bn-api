pub struct UpdateMetadataResult {
    pub id: String,
    pub raw: String,
}

impl UpdateMetadataResult {
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.raw)
    }
}
