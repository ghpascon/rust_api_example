use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagInput {
    pub epc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    pub ant: i32,
    pub rssi: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagRecord {
    pub identifier: String,
    pub epc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    pub ant: i32,
    pub rssi: i32,
    pub created_at_ms: u128,
}
