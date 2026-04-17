// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative)

use crate::deterministic_receipt_hash;
use crate::now_iso;
use serde_json::{json, Value};
use std::path::Path;
use std::{env, fs};

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn parse_scope(argv: &[String]) -> Option<String> {
    for token in argv {
        if let Some(value) = token.strip_prefix("--scope=") {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    for token in argv {
        if let Some(value) = token.strip_prefix(&format!("--{key}=")) {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
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

fn usage() {
    println!("Usage:");
    println!("  protheus-ops workflow-controller <run|status|list|promote> [--scope=<value>] [--max=<n>]");
    println!(
        "  protheus-ops workflow-controller workflow-generator [--action=<run|status>] [flags]"
    );
    println!(
        "  protheus-ops workflow-controller data-rights-engine [--action=<ingest|revoke|process|status>] [flags]"
    );
    println!("  protheus-ops workflow-controller web-tooling-status [--provider=<id>]");
    println!("  protheus-ops workflow-controller web-tooling-errors [--limit=<n>]");
    println!(
        "  protheus-ops workflow-controller web-tooling-probe --query=<text> [--domain=<host>] [--provider=<id>]"
    );
    println!(
        "  protheus-ops workflow-controller web-tooling-preferences [--provider-order=<csv>] [--max-queries=<n>] [--prefer-official-docs=<0|1>]"
    );
}

fn success_receipt(command: &str, scope: Option<&str>, argv: &[String], root: &Path) -> Value {
    let mut out = protheus_autonomy_core_v1::workflow_receipt(command, scope);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("argv".to_string(), json!(argv));
        obj.insert(
            "root".to_string(),
            Value::String(root.to_string_lossy().to_string()),
        );
        obj.insert(
            "claim_evidence".to_string(),
            json!([
                {
                    "id": "workflow_controller_core_lane",
                    "claim": "workflow_controller_commands_are_core_authoritative",
                    "evidence": {
                        "command": command,
                        "scope": scope
                    }
                }
            ]),
        );
    }
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn error_receipt(error: &str, argv: &[String]) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "workflow_controller_error",
        "error": error,
        "argv": argv
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn web_tooling_status_receipt(argv: &[String], root: &Path) -> Value {
    let provider = parse_flag(argv, "provider").unwrap_or_else(|| "auto".to_string());
    let profile_path = root.join("client/runtime/local/state/ui/infring_dashboard/web_tooling_profile.json");
    let action_history_path = root.join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
    let action_rows = fs::read_to_string(&action_history_path)
        .ok()
        .map(|raw| raw.lines().count())
        .unwrap_or(0);
    let mut out = json!({
        "ok": true,
        "type": "workflow_controller_web_tooling_status",
        "command": "web-tooling-status",
        "provider_hint": provider,
        "ts": now_iso(),
        "argv": argv,
        "profile": {
            "path": profile_path.to_string_lossy().to_string(),
            "exists": profile_path.exists()
        },
        "history": {
            "path": action_history_path.to_string_lossy().to_string(),
            "line_count": action_rows
        },
        "auth": {
            "sources": web_auth_sources()
        }
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn web_tooling_errors_receipt(argv: &[String], root: &Path) -> Value {
    let limit = parse_flag(argv, "limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .map(|value| value.clamp(1, 200))
        .unwrap_or(40);
    let action_history_path = root.join("client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl");
    let rows = fs::read_to_string(&action_history_path)
        .ok()
        .map(|raw| {
            raw.lines()
                .rev()
                .filter(|line| line.contains("error") || line.contains("web_tool_"))
                .take(limit)
                .map(|line| line.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut out = json!({
        "ok": true,
        "type": "workflow_controller_web_tooling_errors",
        "command": "web-tooling-errors",
        "ts": now_iso(),
        "argv": argv,
        "history_path": action_history_path.to_string_lossy().to_string(),
        "error_rows": rows,
        "count": rows.len()
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn web_tooling_probe_receipt(argv: &[String], root: &Path) -> Value {
    let input = parse_flag(argv, "query")
        .or_else(|| parse_flag(argv, "q"))
        .unwrap_or_default();
    if input.trim().is_empty() {
        return error_receipt("query_required", argv);
    }
    let domain = parse_flag(argv, "domain");
    let provider = parse_flag(argv, "provider").unwrap_or_else(|| "auto".to_string());
    let sanitized = sanitize_web_query(&input);
    let canonical = canonicalize_web_query(&sanitized, domain.as_deref());
    let mut out = json!({
        "ok": true,
        "type": "workflow_controller_web_tooling_probe",
        "command": "web-tooling-probe",
        "ts": now_iso(),
        "argv": argv,
        "root": root.to_string_lossy().to_string(),
        "provider_hint": provider,
        "query": {
            "input": input,
            "sanitized": sanitized,
            "canonical": canonical
        }
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn web_tooling_preferences_receipt(argv: &[String], _root: &Path) -> Value {
    let provider_order = parse_flag(argv, "provider-order")
        .unwrap_or_else(|| "auto".to_string())
        .split(',')
        .map(|row| row.trim().to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    let max_queries = parse_flag(argv, "max-queries")
        .and_then(|raw| raw.parse::<i64>().ok())
        .map(|value| value.clamp(1, 8))
        .unwrap_or(4);
    let prefer_official_docs = parse_flag(argv, "prefer-official-docs")
        .map(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(true);
    let mut out = json!({
        "ok": true,
        "type": "workflow_controller_web_tooling_preferences",
        "command": "web-tooling-preferences",
        "ts": now_iso(),
        "argv": argv,
        "profile_patch": {
            "provider_order": if provider_order.is_empty() { vec!["auto".to_string()] } else { provider_order },
            "query_policy": {
                "max_queries": max_queries,
                "prefer_official_docs": prefer_official_docs
            }
        }
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let scope = parse_scope(argv).or_else(|| Some("changed".to_string()));
    if matches!(
        command.as_str(),
        "run" | "status" | "list" | "promote" | "workflow-generator" | "data-rights-engine"
    ) {
        print_json_line(&success_receipt(
            command.as_str(),
            scope.as_deref(),
            argv,
            root,
        ));
        return 0;
    }
    if command == "web-tooling-status" {
        print_json_line(&web_tooling_status_receipt(argv, root));
        return 0;
    }
    if command == "web-tooling-errors" {
        print_json_line(&web_tooling_errors_receipt(argv, root));
        return 0;
    }
    if command == "web-tooling-probe" {
        print_json_line(&web_tooling_probe_receipt(argv, root));
        return 0;
    }
    if command == "web-tooling-preferences" {
        print_json_line(&web_tooling_preferences_receipt(argv, root));
        return 0;
    }
    usage();
    print_json_line(&error_receipt("unknown_command", argv));
    2
}
