pub struct RepeatChargeToken {
    pub token: String,
    pub raw: String,
}

use serde_json::Error as SerdeError;
impl RepeatChargeToken {
    pub fn to_json(&self) -> Result<serde_json::Value, SerdeError> {
        serde_json::from_str(&self.raw)
    }
}
