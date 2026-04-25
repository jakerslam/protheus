// Layer ownership: core/layer0/ops (authoritative memory query tool surface).
use serde_json::{json, Value};
use std::path::Path;

fn usage() {
    println!("memory-semantic-query commands:");
    println!("  infring-ops memory-semantic-query query --agent=<id> --query=\"...\" [--limit=<n>]");
    println!("  infring-ops memory-semantic-query status");
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub fn memory_semantic_query_payload(root: &Path, argv: &[String]) -> Value {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "query".to_string());
    match command.as_str() {
        "help" => json!({
            "ok": true,
            "type": "memory_semantic_query_help",
            "commands": ["query", "status"]
        }),
        "status" => json!({
            "ok": true,
            "type": "memory_semantic_query_status",
            "tool": "memory_semantic_query",
            "agent_scoped": true,
            "engine": "dashboard_agent_memory_kv_semantic_query",
            "max_limit": 25
        }),
        "query" => {
            let agent_id = parsed
                .flags
                .get("agent")
                .or_else(|| parsed.flags.get("agent-id"))
                .or_else(|| parsed.positional.get(1))
                .map(String::as_str)
                .unwrap_or("");
            let query = parsed
                .flags
                .get("query")
                .or_else(|| parsed.flags.get("q"))
                .or_else(|| parsed.positional.get(2))
                .map(String::as_str)
                .unwrap_or("");
            let limit = parsed
                .flags
                .get("limit")
                .and_then(|raw| raw.trim().parse::<usize>().ok())
                .unwrap_or(8)
                .clamp(1, 25);
            let mut out =
                crate::dashboard_agent_state::memory_kv_semantic_query(root, agent_id, query, limit);
            out["tool"] = json!("memory_semantic_query");
            out["surface"] = json!("cli");
            out["limit"] = json!(limit);
            out["receipt_hash"] = json!(crate::deterministic_receipt_hash(&out));
            out
        }
        _ => json!({
            "ok": false,
            "type": "memory_semantic_query_error",
            "error": "memory_semantic_query_unknown_command",
            "command": command
        }),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let payload = memory_semantic_query_payload(root, argv);
    if payload.get("type").and_then(Value::as_str) == Some("memory_semantic_query_help") {
        usage();
    }
    print_payload(&payload);
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_semantic_query_cli_returns_ranked_matches_and_receipt() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::memory_kv_set(
            root.path(),
            "agent-cli",
            "fact.auth.flow",
            &json!("OAuth callback uses PKCE and nonce binding"),
        );
        let _ = crate::dashboard_agent_state::memory_kv_set(
            root.path(),
            "agent-cli",
            "fact.release.notes",
            &json!("Dashboard resize changed panel density"),
        );
        let out = memory_semantic_query_payload(
            root.path(),
            &[
                "query".to_string(),
                "--agent=agent-cli".to_string(),
                "--query=auth pkce".to_string(),
                "--limit=4".to_string(),
            ],
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/matches/0/key").and_then(Value::as_str),
            Some("fact.auth.flow")
        );
        assert_eq!(
            out.get("tool").and_then(Value::as_str),
            Some("memory_semantic_query")
        );
        assert!(out
            .get("receipt_hash")
            .and_then(Value::as_str)
            .is_some_and(|row| !row.is_empty()));
    }

    #[test]
    fn memory_semantic_query_missing_agent_or_query_fails_closed() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = memory_semantic_query_payload(root.path(), &["query".to_string()]);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("agent_id_and_query_required")
        );
        assert_eq!(
            out.get("tool").and_then(Value::as_str),
            Some("memory_semantic_query")
        );
    }
}
