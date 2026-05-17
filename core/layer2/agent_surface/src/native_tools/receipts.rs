use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeToolReceipt {
    pub call_id: String,
    pub tool_name: String,
    pub status: String,
    pub duration_ms: u64,
    pub result: Value,
    pub error: Option<String>,
}
