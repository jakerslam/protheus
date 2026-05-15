use crate::native_tools::file_patch::file_patch;
use crate::native_tools::file_read::{file_read, file_read_many};
use crate::native_tools::file_write::file_write;
use crate::native_tools::protocol::NativeToolCall;
use crate::native_tools::receipts::NativeToolReceipt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct NativeToolDispatcher {
    allowed_tools: BTreeSet<String>,
}

impl NativeToolDispatcher {
    pub fn new(tools: &[String]) -> Self {
        Self {
            allowed_tools: tools.iter().map(|tool| normalize_tool_name(tool)).collect(),
        }
    }

    pub fn has_native_tools(&self) -> bool {
        self.allowed_tools.iter().any(|tool| {
            matches!(
                tool.as_str(),
                "file_read" | "file_read_many" | "file_write" | "file_patch"
            )
        })
    }

    pub fn tool_protocol_prompt(&self) -> String {
        let mut tools = self.allowed_tools.iter().cloned().collect::<Vec<_>>();
        tools.sort();
        format!(
            "Native Infring file tools are available when needed. To call tools, return only JSON in this shape: {{\"tool_calls\":[{{\"id\":\"call_1\",\"name\":\"file_read|file_write|file_patch\",\"args\":{{...}}}}]}}. Supported tools for this run: {}. file_read args: {{\"path\":\"/absolute/path\",\"start_line\":1,\"end_line\":20}}. file_write args: {{\"path\":\"/absolute/path\",\"content\":\"text\",\"overwrite\":false}}. file_patch args: {{\"path\":\"/absolute/path\",\"old\":\"exact text\",\"new\":\"replacement text\",\"allow_multiple\":false}}. After tool results, either call more tools or return final text. Do not claim a file operation succeeded unless a tool receipt says status=ok.",
            tools.join(", ")
        )
    }

    pub fn dispatch(&self, call: NativeToolCall) -> NativeToolReceipt {
        let started = Instant::now();
        let tool_name = normalize_tool_name(&call.name);
        let result = if self.allowed_tools.contains(&tool_name) {
            dispatch_allowed_tool(&tool_name, &call.args)
        } else {
            Err(format!("native_tool_not_allowed:{tool_name}"))
        };
        let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        match result {
            Ok(result) => NativeToolReceipt {
                call_id: call.id,
                tool_name,
                status: "ok".to_string(),
                duration_ms,
                result,
                error: None,
            },
            Err(error) => NativeToolReceipt {
                call_id: call.id,
                tool_name,
                status: "error".to_string(),
                duration_ms,
                result: Value::Null,
                error: Some(error),
            },
        }
    }
}

fn dispatch_allowed_tool(tool_name: &str, args: &Value) -> Result<Value, String> {
    match tool_name {
        "file_read" => file_read(args),
        "file_read_many" => file_read_many(args),
        "file_write" => file_write(args),
        "file_patch" => file_patch(args),
        _ => Err(format!("native_tool_not_implemented:{tool_name}")),
    }
}

fn normalize_tool_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "read_file" | "workspace.read" | "workspace_read" => "file_read".to_string(),
        "read_many_files" | "workspace.read_many" | "workspace_read_many" => {
            "file_read_many".to_string()
        }
        "write_file" | "workspace.write" | "workspace_write" => "file_write".to_string(),
        "apply_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => {
            "file_patch".to_string()
        }
        other => other.to_string(),
    }
}
