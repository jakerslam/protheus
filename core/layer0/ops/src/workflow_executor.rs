// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use std::path::Path;
use std::{env, fs};

const LANE_ID: &str = "workflow_executor";
const REPLACEMENT: &str = "protheus-ops workflow-executor";

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops workflow-executor status [--scope=<value>]");
    println!("  protheus-ops workflow-executor run [--scope=<value>] [--max=<n>]");
    println!(
        "  protheus-ops workflow-executor web-status [--provider=<id>] [--history-window=<n>]"
    );
    println!(
        "  protheus-ops workflow-executor web-probe --query=<text> [--domain=<host>] [--provider=<id>]"
    );
}

fn status_receipt(root: &Path, cmd: &str, args: &[String]) -> Value {
    let scope =
        lane_utils::parse_flag(args, "scope", false).unwrap_or_else(|| "changed".to_string());
    let max = lane_utils::parse_flag(args, "max", false)
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 500))
        .unwrap_or(25);

    let mut out = protheus_autonomy_core_v1::workflow_receipt(cmd, Some(&scope));
    out["lane"] = Value::String(LANE_ID.to_string());
    out["ts"] = Value::String(now_iso());
    out["max"] = json!(max);
    out["argv"] = json!(args);
    out["root"] = Value::String(root.to_string_lossy().to_string());
    out["replacement"] = Value::String(REPLACEMENT.to_string());
    out["claim_evidence"] = json!([
        {
            "id": "native_workflow_executor_lane",
            "claim": "workflow_executor_executes_natively_in_rust",
            "evidence": {
                "command": cmd,
                "max": max
            }
        }
    ]);
    out["persona_lenses"] = json!({
        "operator": {
            "mode": "workflow"
        }
    });
    if let Some(map) = out.as_object_mut() {
        map.remove("receipt_hash");
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(args: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "workflow_executor_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": args,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(args, key, false)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_invisible_unicode(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x206A..=0x206F
            | 0xFEFF
            | 0xE0000..=0xE007F
    )
}

fn sanitize_web_query(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if is_invisible_unicode(ch) {
            continue;
        }
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        cleaned.push(ch);
    }
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(1200)
        .collect::<String>()
}

fn normalize_domain_hint(raw: &str) -> String {
    let lowered = sanitize_web_query(raw).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let without_scheme = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
        .unwrap_or(&lowered)
        .to_string();
    without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .trim_matches('.')
        .to_string()
}

fn canonicalize_web_query(query: &str, domain_hint: Option<&str>) -> String {
    let sanitized = sanitize_web_query(query);
    if sanitized.is_empty() {
        return sanitized;
    }
    if sanitized.to_ascii_lowercase().contains("site:") {
        return sanitized;
    }
    let domain = domain_hint
        .map(normalize_domain_hint)
        .filter(|value| !value.is_empty());
    if let Some(domain) = domain {
        return format!("site:{domain} {sanitized}");
    }
    sanitized
}

fn web_auth_sources() -> Vec<String> {
    let mut rows = Vec::<String>::new();
    for (label, env_var) in [
        ("openai", "OPENAI_API_KEY"),
        ("github", "GITHUB_TOKEN"),
        ("github_app", "GITHUB_APP_INSTALLATION_TOKEN"),
        ("brave", "BRAVE_API_KEY"),
        ("tavily", "TAVILY_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("exa", "EXA_API_KEY"),
    ] {
        let present = env::var(env_var)
            .ok()
            .map(|raw| !sanitize_web_query(&raw).is_empty())
            .unwrap_or(false);
        if present {
            rows.push(label.to_string());
        }
    }
    rows.sort();
    rows.dedup();
    rows
}

fn web_status_receipt(root: &Path, argv: &[String]) -> Value {
    let provider = parse_flag(argv, "provider").unwrap_or_else(|| "auto".to_string());
    let history_window = parse_flag(argv, "history-window")
        .and_then(|raw| raw.parse::<usize>().ok())
        .map(|value| value.clamp(20, 1000))
        .unwrap_or(120);
    let profile_path = root.join("client/runtime/local/state/ui/infring_dashboard/web_tooling_profile.json");
    let action_history_path = root.join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
    let action_history_rows = fs::read_to_string(&action_history_path)
        .ok()
        .map(|raw| raw.lines().count())
        .unwrap_or(0);
    let mut out = json!({
        "ok": true,
        "type": "workflow_executor_web_status",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "provider_hint": provider,
        "history_window": history_window,
        "profile": {
            "path": profile_path.to_string_lossy().to_string(),
            "exists": profile_path.exists()
        },
        "history": {
            "path": action_history_path.to_string_lossy().to_string(),
            "line_count": action_history_rows,
            "window_cap": history_window
        },
        "auth": {
            "sources": web_auth_sources()
        },
        "claim_evidence": [{
            "id": "workflow_executor_web_status_lane",
            "claim": "workflow_executor_surfaces_backend_web_tooling_readiness_without_client_authority",
            "evidence": {
                "provider_hint": provider
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn web_probe_receipt(root: &Path, argv: &[String]) -> Value {
    let raw_query = parse_flag(argv, "query")
        .or_else(|| parse_flag(argv, "q"))
        .unwrap_or_default();
    if raw_query.trim().is_empty() {
        let mut out = json!({
            "ok": false,
            "type": "workflow_executor_web_probe",
            "lane": LANE_ID,
            "error": "query_required",
            "argv": argv
        });
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        return out;
    }
    let provider = parse_flag(argv, "provider").unwrap_or_else(|| "auto".to_string());
    let domain = parse_flag(argv, "domain");
    let sanitized = sanitize_web_query(&raw_query);
    let canonical = canonicalize_web_query(&sanitized, domain.as_deref());
    let mut out = json!({
        "ok": true,
        "type": "workflow_executor_web_probe",
        "lane": LANE_ID,
        "ts": now_iso(),
        "root": root.to_string_lossy().to_string(),
        "provider_hint": provider,
        "query": {
            "input": raw_query,
            "sanitized": sanitized,
            "canonical": canonical
        },
        "claim_evidence": [{
            "id": "workflow_executor_web_probe_lane",
            "claim": "workflow_executor_applies_server_side_query_sanitization_and_canonicalization_for_web_tooling",
            "evidence": {
                "provider_hint": provider
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    match cmd.as_str() {
        "status" | "run" => {
            print_json_line(&status_receipt(root, &cmd, argv));
            0
        }
        "web-status" => {
            print_json_line(&web_status_receipt(root, argv));
            0
        }
        "web-probe" => {
            let payload = web_probe_receipt(root, argv);
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&payload);
            if ok { 0 } else { 2 }
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flag_supports_equals_and_split_forms() {
        assert_eq!(
            lane_utils::parse_flag(&["--scope=all".to_string()], "scope", false).as_deref(),
            Some("all")
        );
        assert_eq!(
            lane_utils::parse_flag(&["--max".to_string(), "9".to_string()], "max", false)
                .as_deref(),
            Some("9")
        );
    }

    #[test]
    fn status_receipt_is_hashed() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = status_receipt(
            root.path(),
            "run",
            &[
                "run".to_string(),
                "--scope=all".to_string(),
                "--max=5".to_string(),
            ],
        );
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("obj")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), hash);
    }

    #[test]
    fn web_probe_receipt_canonicalizes_domain() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = web_probe_receipt(
            root.path(),
            &[
                "web-probe".to_string(),
                "--query=top ai agent frameworks".to_string(),
                "--domain=langchain.com/docs".to_string(),
            ],
        );
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        let canonical = payload
            .pointer("/query/canonical")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(canonical.starts_with("site:langchain.com "));
    }
}
