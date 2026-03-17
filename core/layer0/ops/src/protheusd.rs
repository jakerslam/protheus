// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::{
    client_state_root, configure_low_memory_allocator_env, daemon_control,
    deterministic_receipt_hash, now_iso, parse_os_args, status_runtime_efficiency_floor,
};
use serde_json::{json, Value};
use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "embedded-minimal-core")]
type PlaneRunner = fn(&Path, &[String]) -> i32;

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  infringd status");
    println!("  infringd start [--strict=1|0]");
    println!("  infringd stop [--strict=1|0]");
    println!("  infringd restart [--strict=1|0]");
    println!("  infringd attach [--strict=1|0]");
    println!("  infringd subscribe [--strict=1|0]");
    println!("  infringd tick [--strict=1|0]");
    println!("  infringd diagnostics [--strict=1|0]");
    println!("  infringd think --prompt=<text> [--session-id=<id>] [--memory-limit=<n>]");
    println!("  infringd research <status|fetch|diagnostics> [flags]");
    println!("  infringd memory <status|write|query> [flags]");
    println!("  infringd efficiency-status");
    #[cfg(feature = "embedded-minimal-core")]
    println!("  infringd embedded-core-status");
    #[cfg(feature = "tiny")]
    println!("  infringd tiny-status");
    #[cfg(feature = "embedded-max")]
    println!("  infringd tiny-max-status");
}

fn cli_error(error: &str, command: &str) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "protheusd_error",
        "command": command,
        "error": error,
        "ts": now_iso()
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let key_token = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&pref) {
            return Some(value.trim().to_string());
        }
        if token == key_token {
            if let Some(next) = argv.get(idx + 1) {
                return Some(next.trim().to_string());
            }
        }
        idx += 1;
    }
    None
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    let source = raw.unwrap_or("").trim();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            out.push(ch);
        } else if ch.is_ascii_whitespace() && !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 64 {
            break;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed
    }
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.unwrap_or("").trim().chars() {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        out.push(ch);
        if out.len() >= max_len {
            break;
        }
    }
    out
}

fn parse_usize(raw: Option<&str>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn memory_store_path(root: &Path) -> PathBuf {
    client_state_root(root)
        .join("memory")
        .join("pure_workspace_memory_v1.jsonl")
}

fn read_memory_entries(path: &Path) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    let file = match std::fs::File::open(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            out.push(value);
        }
    }
    out
}

fn append_memory_entry(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create_memory_parent_failed:{err}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_memory_store_failed:{err}"))?;
    let line =
        serde_json::to_string(row).map_err(|err| format!("encode_memory_row_failed:{err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("write_memory_row_failed:{err}"))?;
    file.write_all(b"\n")
        .map_err(|err| format!("write_memory_newline_failed:{err}"))?;
    Ok(())
}

fn memory_status_payload(root: &Path) -> Value {
    let path = memory_store_path(root);
    let entries = read_memory_entries(&path);
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let last_ts = entries
        .last()
        .and_then(|v| v.get("ts"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_status",
        "ts": now_iso(),
        "path": path.to_string_lossy(),
        "entry_count": entries.len(),
        "bytes": bytes,
        "last_ts": last_ts
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn memory_write_payload(root: &Path, argv: &[String]) -> Result<Value, String> {
    let text = clean_text(parse_flag(argv, "text").as_deref(), 4000);
    if text.is_empty() {
        return Err("missing_text".to_string());
    }
    let session_id = clean_token(parse_flag(argv, "session-id").as_deref(), "default");
    let tags = parse_flag(argv, "tags")
        .map(|raw| {
            raw.split(',')
                .map(|v| clean_token(Some(v), ""))
                .filter(|v| !v.is_empty())
                .take(16)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ts = now_iso();
    let id_seed = json!({
        "text": text,
        "session_id": session_id,
        "ts": ts
    });
    let derived_id = deterministic_receipt_hash(&id_seed)
        .chars()
        .take(16)
        .collect::<String>();
    let item_id = clean_token(parse_flag(argv, "id").as_deref(), derived_id.as_str());
    let row = json!({
        "id": item_id,
        "ts": ts,
        "session_id": session_id,
        "text": text,
        "tags": tags
    });
    let path = memory_store_path(root);
    append_memory_entry(&path, &row)?;
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_write",
        "ts": now_iso(),
        "path": path.to_string_lossy(),
        "item": row
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    Ok(out)
}

fn memory_query_payload(root: &Path, argv: &[String]) -> Value {
    let q = clean_text(
        parse_flag(argv, "q")
            .or_else(|| parse_flag(argv, "text"))
            .as_deref(),
        240,
    )
    .to_ascii_lowercase();
    let session = clean_token(parse_flag(argv, "session-id").as_deref(), "");
    let tag = clean_token(parse_flag(argv, "tag").as_deref(), "").to_ascii_lowercase();
    let limit = parse_usize(parse_flag(argv, "limit").as_deref(), 20, 1, 200);
    let mut entries = read_memory_entries(&memory_store_path(root))
        .into_iter()
        .filter(|row| {
            let text = row
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let row_session = row
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let tag_match = if tag.is_empty() {
                true
            } else {
                row.get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| {
                        tags.iter().any(|v| {
                            v.as_str()
                                .map(|s| s.to_ascii_lowercase() == tag)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            };
            let session_match = session.is_empty() || row_session == session;
            let text_match = q.is_empty() || text.contains(&q);
            session_match && tag_match && text_match
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        b.get("ts")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("ts").and_then(Value::as_str).unwrap_or(""))
    });
    entries.truncate(limit);
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_query",
        "ts": now_iso(),
        "q": q,
        "session_id": if session.is_empty() { Value::Null } else { Value::String(session.clone()) },
        "tag": if tag.is_empty() { Value::Null } else { Value::String(tag.clone()) },
        "limit": limit,
        "matches": entries
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn contains_any_token(haystack: &str, tokens: &[String]) -> usize {
    let hay = haystack.to_ascii_lowercase();
    tokens
        .iter()
        .filter(|token| hay.contains(token.as_str()))
        .count()
}

fn think_payload(root: &Path, argv: &[String]) -> Result<Value, String> {
    let prompt = clean_text(parse_flag(argv, "prompt").as_deref(), 1200);
    if prompt.is_empty() {
        return Err("missing_prompt".to_string());
    }
    let session_id = clean_token(parse_flag(argv, "session-id").as_deref(), "default");
    let memory_limit = parse_usize(parse_flag(argv, "memory-limit").as_deref(), 5, 1, 20);
    let lower_prompt = prompt.to_ascii_lowercase();
    let tokens = lower_prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .take(12)
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    let mut scored = read_memory_entries(&memory_store_path(root))
        .into_iter()
        .filter_map(|entry| {
            let row_session = entry
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if row_session != session_id {
                return None;
            }
            let text = entry
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let score = contains_any_token(&text, &tokens);
            if score == 0 {
                return None;
            }
            Some((score, entry))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let memory_hits = scored
        .into_iter()
        .take(memory_limit)
        .map(|(_, row)| row)
        .collect::<Vec<_>>();

    let hint = if lower_prompt.contains("http://") || lower_prompt.contains("https://") {
        "Detected URL intent: run `infring research fetch --url=<url>` for source capture."
    } else if lower_prompt.contains("research") {
        "Research intent detected: run `infring research status` then `infring research fetch --url=<url>`."
    } else {
        "Action intent detected: break the task into one immediate execution step and one verification step."
    };
    let response = format!(
        "Prompt focus: {}. {}",
        prompt.chars().take(180).collect::<String>(),
        hint
    );
    let mut out = json!({
        "ok": true,
        "type": "pure_think",
        "ts": now_iso(),
        "session_id": session_id,
        "prompt": prompt,
        "memory_hits": memory_hits,
        "response": response,
        "next_actions": [
            "define_success_criteria",
            "execute_smallest_safe_step",
            "record_outcome_in_memory"
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    Ok(out)
}

fn run_research(root: &Path, argv: &[String]) -> i32 {
    let mut rest = argv.to_vec();
    if rest.is_empty() {
        rest.push("status".to_string());
    }
    let command = rest
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if !matches!(command.as_str(), "status" | "fetch" | "diagnostics") {
        print_json(&cli_error(
            "research_command_not_allowed_in_pure_v1",
            "research",
        ));
        return 1;
    }
    protheus_ops_core::research_plane::run(root, &rest)
}

fn run_memory(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match command.as_str() {
        "status" => {
            print_json(&memory_status_payload(root));
            0
        }
        "write" => match memory_write_payload(root, &argv[1..]) {
            Ok(payload) => {
                print_json(&payload);
                0
            }
            Err(err) => {
                print_json(&cli_error(err.as_str(), "memory"));
                1
            }
        },
        "query" => {
            print_json(&memory_query_payload(root, &argv[1..]));
            0
        }
        _ => {
            print_json(&cli_error("unknown_memory_command", "memory"));
            1
        }
    }
}

fn run_think(root: &Path, argv: &[String]) -> i32 {
    match think_payload(root, argv) {
        Ok(payload) => {
            print_json(&payload);
            0
        }
        Err(err) => {
            print_json(&cli_error(err.as_str(), "think"));
            1
        }
    }
}

#[cfg(feature = "embedded-minimal-core")]
fn embedded_minimal_core_planes() -> [(&'static str, &'static str, PlaneRunner); 5] {
    [
        (
            "layer0-directives",
            "directive_kernel",
            protheus_ops_core::directive_kernel::run,
        ),
        (
            "layer0-attention",
            "attention_queue",
            protheus_ops_core::attention_queue::run,
        ),
        (
            "layer0-receipts",
            "metakernel",
            protheus_ops_core::metakernel::run,
        ),
        (
            "layer0-min-memory",
            "memory_plane",
            protheus_ops_core::memory_plane::run,
        ),
        (
            "layer-1-substrate-detector",
            "substrate_plane",
            protheus_ops_core::substrate_plane::run,
        ),
    ]
}

#[cfg(feature = "embedded-minimal-core")]
fn embedded_minimal_core_status() -> Value {
    let planes = embedded_minimal_core_planes();
    let lane_entries: Vec<Value> = planes
        .iter()
        .map(|(feature, lane, runner)| {
            json!({
                "feature": feature,
                "lane": lane,
                "runner_ptr": format!("{:p}", *runner as *const ())
            })
        })
        .collect();
    let runner_ptr_fingerprint = deterministic_receipt_hash(&json!(lane_entries));
    let mut out = json!({
        "ok": true,
        "type": "protheusd_embedded_minimal_core_status",
        "ts": now_iso(),
        "embedded_feature": "embedded-minimal-core",
        "planes_embedded": lane_entries,
        "runner_ptr_fingerprint": runner_ptr_fingerprint,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(feature = "tiny")]
fn tiny_status() -> Value {
    let profile = protheus_tiny_runtime::tiny_profile();
    let capacity = protheus_tiny_runtime::normalized_capacity_score(
        profile.max_heap_kib,
        profile.max_concurrent_hands,
    );
    let mut out = json!({
        "ok": true,
        "type": "protheusd_tiny_status",
        "ts": now_iso(),
        "profile": profile.profile,
        "no_std": profile.no_std,
        "max_heap_kib": profile.max_heap_kib,
        "max_concurrent_hands": profile.max_concurrent_hands,
        "supports_hibernation": profile.supports_hibernation,
        "supports_receipt_batching": profile.supports_receipt_batching,
        "capacity_score": capacity
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(feature = "embedded-max")]
fn tiny_max_status() -> Value {
    let profile = protheus_tiny_runtime::tiny_profile();
    let mut out = json!({
        "ok": true,
        "type": "protheusd_tiny_max_status",
        "ts": now_iso(),
        "mode": "embedded-max",
        "no_std_runtime": profile.no_std,
        "allocator_profile": "minimal-alloc",
        "pgo_profile_enabled": cfg!(feature = "pgo-profile"),
        "max_heap_kib": profile.max_heap_kib,
        "max_concurrent_hands": profile.max_concurrent_hands
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn main() {
    configure_low_memory_allocator_env();
    #[cfg(feature = "embedded-max")]
    std::env::set_var("PROTHEUS_EMBEDDED_MAX", "1");
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = parse_os_args(env::args_os().skip(1));
    let command = args
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return;
    }

    match command.as_str() {
        "status" | "start" | "stop" | "restart" | "attach" | "subscribe" | "tick"
        | "diagnostics" => {
            let exit = daemon_control::run(&cwd, &args);
            std::process::exit(exit);
        }
        "think" => {
            let exit = run_think(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "research" => {
            let exit = run_research(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "memory" => {
            let exit = run_memory(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "efficiency-status" => {
            let parsed = protheus_ops_core::parse_args(&[]);
            let out = status_runtime_efficiency_floor(&cwd, &parsed).json;
            print_json(&out);
            std::process::exit(0);
        }
        #[cfg(feature = "embedded-minimal-core")]
        "embedded-core-status" => {
            print_json(&embedded_minimal_core_status());
            std::process::exit(0);
        }
        #[cfg(feature = "tiny")]
        "tiny-status" => {
            print_json(&tiny_status());
            std::process::exit(0);
        }
        #[cfg(feature = "embedded-max")]
        "tiny-max-status" => {
            print_json(&tiny_max_status());
            std::process::exit(0);
        }
        _ => {
            usage();
            print_json(&cli_error("unknown_command", command.as_str()));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use serde_json::Value;

    #[test]
    fn memory_write_and_query_roundtrip() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let payload = memory_write_payload(
            root,
            &[
                "--text=remember pure intelligence".to_string(),
                "--session-id=test".to_string(),
                "--tags=intel,pure".to_string(),
            ],
        )
        .expect("write memory");
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("pure_memory_write")
        );

        let query = memory_query_payload(
            root,
            &[
                "--q=intelligence".to_string(),
                "--session-id=test".to_string(),
                "--limit=5".to_string(),
            ],
        );
        assert_eq!(
            query.get("type").and_then(Value::as_str),
            Some("pure_memory_query")
        );
        assert!(query
            .get("matches")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn think_uses_session_memory_hits() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        memory_write_payload(
            root,
            &[
                "--text=research rust safety constraints".to_string(),
                "--session-id=alpha".to_string(),
            ],
        )
        .expect("seed memory");
        let thought = think_payload(
            root,
            &[
                "--prompt=Can you research safety constraints?".to_string(),
                "--session-id=alpha".to_string(),
            ],
        )
        .expect("think");
        assert_eq!(
            thought.get("type").and_then(Value::as_str),
            Some("pure_think")
        );
        assert!(thought
            .get("memory_hits")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[cfg(feature = "tiny")]
    #[test]
    fn tiny_status_emits_receipt_and_profile() {
        let payload = tiny_status();
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("protheusd_tiny_status")
        );
        assert_eq!(payload.get("no_std").and_then(Value::as_bool), Some(true));
        assert!(payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }
}
