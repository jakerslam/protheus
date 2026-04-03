// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::app_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_conduit_enforcement, conduit_bypass_requested,
    load_json_or, parse_bool, parse_u64, read_json, scoped_state_root, sha256_hex_str, write_json,
    write_receipt,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "APP_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "app_plane";

const CHAT_STARTER_CONTRACT_PATH: &str = "planes/contracts/apps/chat_starter_contract_v1.json";
const CHAT_UI_CONTRACT_PATH: &str = "planes/contracts/apps/chat_ui_contract_v1.json";
const CODE_ENGINEER_CONTRACT_PATH: &str = "planes/contracts/apps/code_engineer_contract_v1.json";
const PRODUCT_BUILDER_CONTRACT_PATH: &str =
    "planes/contracts/apps/product_builder_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops app-plane status [--app=<chat-starter|chat-ui|code-engineer>]");
    println!(
        "  protheus-ops app-plane run --app=<chat-starter|chat-ui|code-engineer> [--session-id=<id>] [--message=<text>] [--prompt=<text>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane history --app=<chat-starter|chat-ui> [--session-id=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane replay --app=<chat-starter|chat-ui> [--session-id=<id>] [--turn=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane switch-provider --app=chat-ui --provider=<openai|frontier_provider|grok|bedrock|minimax> [--model=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane build --app=code-engineer --goal=<text> [--risk=<low|medium|high>] [--approved=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane ingress --app=code-engineer --provider=<slack|telegram> --goal=<text> [--strict=1|0]"
    );
    println!(
        "  protheus-ops app-plane template-governance --app=code-engineer --op=<install|update|list> [--template-id=builders://<id>] [--version=<semver>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_payload(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_payload(&json!({
                "ok": false,
                "type": "app_plane_error",
                "error": clean(err, 240)
            }));
            1
        }
    }
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_app_id(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase().replace('_', "-");
    match lower.as_str() {
        "chat" | "chatstarter" | "chat-starter" => "chat-starter".to_string(),
        "chat-ui" | "chatui" => "chat-ui".to_string(),
        "code-engineer" | "codeengineer" | "code-engineer-app" => "code-engineer".to_string(),
        _ => lower,
    }
}

fn parse_app_id(parsed: &crate::ParsedArgs) -> String {
    parsed
        .flags
        .get("app")
        .map(|v| normalize_app_id(v))
        .or_else(|| parsed.positional.get(1).map(|v| normalize_app_id(v)))
        .unwrap_or_else(|| "chat-starter".to_string())
}

fn claim_ids_for_action(action: &str, app_id: &str) -> Vec<&'static str> {
    match app_id {
        "chat-starter" => vec!["V6-APP-008.1"],
        "chat-ui" => vec!["V6-APP-007.1"],
        "code-engineer" => match action {
            "run" => vec!["V6-APP-006.1", "V6-APP-006.2", "V6-APP-006.3"],
            "build" => vec![
                "V6-APP-006.3",
                "V6-APP-006.4",
                "V6-APP-006.5",
                "V6-APP-006.7",
            ],
            "ingress" => vec!["V6-APP-006.3", "V6-APP-006.6"],
            "template-governance" => vec!["V6-APP-006.3", "V6-APP-006.8"],
            _ => vec!["V6-APP-006.3"],
        },
        _ => vec!["V6-APP-006.3"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
    app_id: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_rows = claim_ids_for_action(action, app_id)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "app_actions_route_through_layer0_conduit_with_fail_closed_denials",
                "evidence": {
                    "action": clean(action, 120),
                    "app_id": app_id,
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();

    build_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "app_plane_conduit_enforcement",
        "core/layer0/ops/app_plane",
        bypass_requested,
        claim_rows,
    )
}

fn status(root: &Path, app_id: Option<&str>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "app_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "latest": read_json(&latest_path(root))
    });
    if let Some(app) = app_id {
        out["app"] = Value::String(app.to_string());
    }
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn chat_starter_session_path(root: &Path, session_id: &str) -> PathBuf {
    state_root(root)
        .join("chat_starter")
        .join("sessions")
        .join(format!("{session_id}.json"))
}

fn chat_ui_session_path(root: &Path, session_id: &str) -> PathBuf {
    state_root(root)
        .join("chat_ui")
        .join("sessions")
        .join(format!("{session_id}.json"))
}

fn chat_ui_settings_path(root: &Path) -> PathBuf {
    state_root(root).join("chat_ui").join("settings.json")
}

fn code_engineer_runs_path(root: &Path) -> PathBuf {
    state_root(root).join("code_engineer").join("runs.json")
}

fn message_from_parsed(parsed: &crate::ParsedArgs, start_pos: usize, fallback: &str) -> String {
    let from_flag = parsed.flags.get("message").cloned();
    let from_input = parsed.flags.get("input").cloned();
    let from_prompt = parsed.flags.get("prompt").cloned();
    let from_positional = if parsed.positional.len() > start_pos {
        parsed.positional[start_pos..].join(" ")
    } else {
        String::new()
    };
    clean(
        from_flag.or(from_input).or(from_prompt).unwrap_or_else(|| {
            if from_positional.trim().is_empty() {
                fallback.to_string()
            } else {
                from_positional
            }
        }),
        2000,
    )
}

fn split_stream_chunks(message: &str) -> Vec<String> {
    let words = message.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec!["(empty)".to_string()];
    }
    let mut chunks = Vec::<String>::new();
    let mut cursor = 0usize;
    while cursor < words.len() && chunks.len() < 8 {
        let next = std::cmp::min(cursor + 3, words.len());
        chunks.push(words[cursor..next].join(" "));
        cursor = next;
    }
    chunks
}

fn chat_ui_history_messages(session: &Value) -> Vec<Value> {
    session
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|turn| {
            let mut rows = Vec::<Value>::new();
            let user = clean(
                turn.get("user").and_then(Value::as_str).unwrap_or(""),
                16_000,
            );
            if !user.is_empty() {
                rows.push(json!({"role": "user", "text": user}));
            }
            let assistant = clean(
                turn.get("assistant").and_then(Value::as_str).unwrap_or(""),
                16_000,
            );
            if !assistant.is_empty() {
                rows.push(json!({"role": "assistant", "text": assistant}));
            }
            rows
        })
        .collect::<Vec<_>>()
}
