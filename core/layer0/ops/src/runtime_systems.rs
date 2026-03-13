// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::runtime_systems (authoritative)
use crate::{client_state_root, deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "runtime_systems";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops runtime-systems <status|verify|run|build|manifest|bootstrap|package|settle> [--system-id=<id>|--lane-id=<id>] [flags]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn receipt_hash(value: &Value) -> String {
    deterministic_receipt_hash(value)
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let with_eq = format!("--{key}=");
    let plain = format!("--{key}");
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some(v) = token.strip_prefix(&with_eq) {
            return Some(v.trim().to_string());
        }
        if token == plain {
            if let Some(next) = argv.get(i + 1) {
                if !next.trim_start().starts_with("--") {
                    return Some(next.trim().to_string());
                }
            }
            return Some("true".to_string());
        }
        i += 1;
    }
    None
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    let Some(v) = raw else {
        return fallback;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 160 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                out.push(ch);
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

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.split_whitespace().collect::<Vec<_>>().join(" ").chars() {
            if out.len() >= max_len {
                break;
            }
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn parse_json(raw: Option<&str>) -> Result<Value, String> {
    let text = raw.ok_or_else(|| "missing_json_payload".to_string())?;
    serde_json::from_str::<Value>(text).map_err(|err| format!("invalid_json_payload:{err}"))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut encoded =
        serde_json::to_string_pretty(payload).map_err(|err| format!("encode_failed:{err}"))?;
    encoded.push('\n');
    fs::write(path, encoded).map_err(|err| format!("write_failed:{}:{err}", path.display()))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    use std::io::Write;
    let line = serde_json::to_string(row).map_err(|err| format!("encode_failed:{err}"))? + "\n";
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    let mut file = opts
        .open(path)
        .map_err(|err| format!("open_failed:{}:{err}", path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("append_failed:{}:{err}", path.display()))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}

fn systems_dir(root: &Path) -> PathBuf {
    client_state_root(root).join("runtime_systems")
}

fn latest_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("latest.json")
}

fn history_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("history.jsonl")
}

fn read_only_command(command: &str) -> bool {
    matches!(command, "status" | "verify")
}

fn system_id_from_args(command: &str, args: &[String]) -> String {
    let by_flag = parse_flag(args, "system-id")
        .or_else(|| parse_flag(args, "lane-id"))
        .or_else(|| parse_flag(args, "id"));
    if by_flag.is_some() {
        return clean_token(by_flag.as_deref(), "runtime-system");
    }
    if command.starts_with('v')
        && command
            .chars()
            .any(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
    {
        return clean_token(Some(command), "runtime-system");
    }
    clean_token(None, "runtime-system")
}

fn collect_passthrough(args: &[String]) -> Vec<String> {
    args.iter()
        .filter_map(|row| {
            let t = row.trim();
            if t.is_empty() {
                return None;
            }
            if t.starts_with("--system-id")
                || t.starts_with("--lane-id")
                || t.starts_with("--id")
                || t.starts_with("--apply")
                || t.starts_with("--payload-json")
            {
                return None;
            }
            Some(t.to_string())
        })
        .collect::<Vec<_>>()
}

fn payload_sha(payload: &Value) -> String {
    let encoded = serde_json::to_vec(payload).unwrap_or_default();
    hex::encode(Sha256::digest(encoded))
}

fn status_payload(root: &Path, system_id: &str, command: &str) -> Value {
    let latest = read_json(&latest_path(root, system_id));
    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_status",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "latest_path": rel_path(root, &latest_path(root, system_id)),
        "history_path": rel_path(root, &history_path(root, system_id)),
        "has_state": latest.is_some(),
        "latest": latest
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn run_payload(
    root: &Path,
    system_id: &str,
    command: &str,
    args: &[String],
) -> Result<Value, String> {
    let apply_default = !read_only_command(command);
    let apply = parse_bool(parse_flag(args, "apply").as_deref(), apply_default);
    let payload = parse_flag(args, "payload-json")
        .map(|raw| parse_json(Some(raw.as_str())))
        .transpose()?
        .unwrap_or_else(|| json!({}));
    let passthrough = collect_passthrough(args);
    let ts = now_iso();
    let row = json!({
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "ts": ts,
        "payload": payload,
        "payload_sha256": payload_sha(&payload),
        "passthrough": passthrough,
        "apply": apply
    });

    if apply {
        write_json(&latest_path(root, system_id), &row)?;
        append_jsonl(&history_path(root, system_id), &row)?;
    }

    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "apply": apply,
        "latest_path": rel_path(root, &latest_path(root, system_id)),
        "history_path": rel_path(root, &history_path(root, system_id)),
        "payload_sha256": row.get("payload_sha256").cloned().unwrap_or(Value::Null),
        "claim_evidence": [{
            "id": "runtime_system_mutation_receipted",
            "claim": "runtime_system_operations_emit_deterministic_receipts_and_state",
            "evidence": {
                "system_id": system_id,
                "command": command,
                "apply": apply
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "runtime_systems_cli_error",
        "lane": LANE_ID,
        "argv": argv,
        "error": clean_text(Some(err), 300),
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
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
    let system_id = system_id_from_args(&command, &argv[1..]);
    if system_id.is_empty() {
        print_json_line(&cli_error(argv, "system_id_missing", 2));
        return 2;
    }

    let payload = match command.as_str() {
        "status" | "verify" => Ok(status_payload(root, &system_id, &command)),
        _ => run_payload(root, &system_id, &command, &argv[1..]),
    };

    match payload {
        Ok(out) => {
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error(argv, &err, 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_writes_latest_and_status_reads_it() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--system-id=systems-memory-causal_temporal_graph".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"k\":1}".to_string(),
            ],
        );
        assert_eq!(exit, 0);

        let latest = latest_path(root.path(), "systems-memory-causal_temporal_graph");
        assert!(latest.exists());

        let status = status_payload(
            root.path(),
            "systems-memory-causal_temporal_graph",
            "status",
        );
        assert_eq!(
            status.get("has_state").and_then(Value::as_bool),
            Some(true),
            "status should reflect latest state"
        );
    }

    #[test]
    fn verify_is_read_only_and_does_not_write_state() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "verify".to_string(),
                "--system-id=systems-autonomy-gated_self_improvement_loop".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = latest_path(root.path(), "systems-autonomy-gated_self_improvement_loop");
        assert!(!latest.exists());
    }
}
