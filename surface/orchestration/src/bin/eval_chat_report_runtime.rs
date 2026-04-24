use infring_orchestration_surface_v1::eval_chat_report;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn read_request(path: &str) -> Value {
    if path.is_empty() {
        return json!({});
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn print_json(payload: &Value) {
    let raw = serde_json::to_string(payload).unwrap_or_else(|_| "{\"ok\":false}".to_string());
    println!("{raw}");
}

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let root = parse_flag(&args, "root").unwrap_or_else(|| ".".to_string());
    let agent_id = parse_flag(&args, "agent-id")
        .or_else(|| parse_flag(&args, "agent"))
        .unwrap_or_default();
    let request_path = parse_flag(&args, "request").unwrap_or_default();
    let request = read_request(&request_path);
    let payload = eval_chat_report::stage_dashboard_chat_eval_issue_report(
        Path::new(&root),
        &agent_id,
        &request,
    );
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json(&payload);
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
