// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// V6-ORGAN-001 — Stomach v1 kernel wrapper

use protheus_stomach_core_v1::burn::{
    purge_artifact_path, transition_retention, RetentionEvent, RetentionState,
};
use protheus_stomach_core_v1::proposal::{TransformKind, TransformRequest};
use protheus_stomach_core_v1::state::{rollback_by_receipt, DigestState};
use protheus_stomach_core_v1::{run_stomach_cycle, stable_hash, StomachConfig};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("stomach-kernel commands:");
    println!("  protheus-ops stomach-kernel run --id=<digest_id> --source-root=<path> --origin=<https://...> [--commit=<hash>] [--refs=refs/heads/main] [--spdx=<MIT>] [--transform=namespace_fix|header_injection|path_remap|adapter_scaffold] [--targets=a.rs,b.rs] [--header=...]");
    println!("  protheus-ops stomach-kernel status --id=<digest_id>");
    println!("  protheus-ops stomach-kernel rollback --id=<digest_id> --receipt=<receipt_id> [--reason=<text>]");
    println!("  protheus-ops stomach-kernel purge --id=<digest_id>");
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let prefix = format!("--{key}=");
    for token in argv {
        if let Some(rest) = token.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn csv_list(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn json_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn json_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": true,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn stomach_state_root(root: &Path) -> PathBuf {
    root.join("local").join("state").join("stomach")
}

fn ensure_state_dirs(state_root: &Path) -> Result<(), String> {
    for rel in [
        "quarantine",
        "snapshots",
        "provenance",
        "analysis",
        "proposals",
        "state",
    ] {
        fs::create_dir_all(state_root.join(rel))
            .map_err(|e| format!("stomach_state_dir_create_failed:{rel}:{e}"))?;
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_write_parent_create_failed:{e}"))?;
    }
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|e| format!("stomach_write_encode_failed:{e}"))?;
    fs::write(path, format!("{encoded}\n")).map_err(|e| format!("stomach_write_failed:{e}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("stomach_read_failed:{e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("stomach_decode_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_jsonl_parent_create_failed:{e}"))?;
    }
    let line =
        serde_json::to_string(value).map_err(|e| format!("stomach_jsonl_encode_failed:{e}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("stomach_jsonl_open_failed:{e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("stomach_jsonl_write_failed:{e}"))
}

fn parse_transform(argv: &[String]) -> TransformRequest {
    let transform = parse_flag(argv, "transform").unwrap_or_else(|| "header_injection".to_string());
    let targets = csv_list(parse_flag(argv, "targets"));
    match transform.to_ascii_lowercase().as_str() {
        "namespace_fix" => TransformRequest {
            kind: TransformKind::NamespaceFix,
            target_paths: targets,
            namespace_from: parse_flag(argv, "namespace-from"),
            namespace_to: parse_flag(argv, "namespace-to"),
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: None,
        },
        "path_remap" => TransformRequest {
            kind: TransformKind::PathRemap,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: parse_flag(argv, "path-from"),
            path_prefix_to: parse_flag(argv, "path-to"),
            adapter_name: None,
        },
        "adapter_scaffold" => TransformRequest {
            kind: TransformKind::AdapterScaffold,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: parse_flag(argv, "adapter-name"),
        },
        _ => TransformRequest::header_injection(
            targets,
            parse_flag(argv, "header").unwrap_or_else(|| "// staged by stomach".to_string()),
        ),
    }
}

fn run_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").unwrap_or_else(|| "stomach-default".to_string());
    let source_root = parse_flag(argv, "source-root")
        .map(PathBuf::from)
        .ok_or_else(|| "stomach_missing_source_root".to_string())?;
    let origin = parse_flag(argv, "origin")
        .unwrap_or_else(|| "https://github.com/protheuslabs/InfRing".to_string());
    let commit = parse_flag(argv, "commit").unwrap_or_else(|| "unknown".to_string());
    let refs = csv_list(parse_flag(argv, "refs"));
    let spdx = parse_flag(argv, "spdx");
    let transform = parse_transform(argv);

    let state_root = stomach_state_root(root);
    ensure_state_dirs(&state_root)?;
    let out = run_stomach_cycle(
        &state_root,
        &digest_id,
        &source_root,
        &origin,
        &commit,
        &refs,
        spdx.as_deref(),
        &transform,
        &StomachConfig::default(),
    )?;

    write_json(
        &state_root
            .join("snapshots")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.snapshot)
            .map_err(|e| format!("stomach_snapshot_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("provenance")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.provenance)
            .map_err(|e| format!("stomach_provenance_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("analysis")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.analysis)
            .map_err(|e| format!("stomach_analysis_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.proposal)
            .map_err(|e| format!("stomach_proposal_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root.join("state").join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.state)
            .map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;

    let receipt_payload = json!({
      "digest_id": digest_id,
      "snapshot_id": out.snapshot.snapshot_id,
      "proposal_id": out.proposal.proposal_id,
      "execution_status": out.execution.status,
      "state_status": out.state.status,
      "cycle_hash": stable_hash(&out)
    });
    let receipt = json_receipt("stomach_kernel_run", receipt_payload);
    append_jsonl(&state_root.join("receipts.jsonl"), &receipt)?;
    Ok(receipt)
}

fn status_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state = read_json(&state_root.join("state").join(format!("{digest_id}.json")))?;
    let proposal = read_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
    )
    .ok();
    Ok(json_receipt(
        "stomach_kernel_status",
        json!({
          "digest_id": digest_id,
          "state": state,
          "proposal": proposal
        }),
    ))
}

fn rollback_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let receipt =
        parse_flag(argv, "receipt").ok_or_else(|| "stomach_missing_receipt".to_string())?;
    let reason = parse_flag(argv, "reason").unwrap_or_else(|| "manual_rollback".to_string());
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    let rollback = rollback_by_receipt(&mut state, &receipt, &reason)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_rollback",
        json!({
          "digest_id": digest_id,
          "rollback": rollback,
          "state_status": state.status
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

fn purge_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    if state.retention.state == RetentionState::Retained {
        transition_retention(&mut state.retention, RetentionEvent::MarkEligibleForPurge)?;
    }
    let quarantine_dir = state_root.join("quarantine").join(&digest_id);
    purge_artifact_path(&quarantine_dir, &mut state.retention)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_purge",
        json!({
          "digest_id": digest_id,
          "retention_state": state.retention_state()
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let response = match command.as_str() {
        "run" => run_cycle(root, &argv[1..]),
        "status" => status_cycle(root, &argv[1..]),
        "rollback" => rollback_cycle(root, &argv[1..]),
        "purge" => purge_cycle(root, &argv[1..]),
        _ => Err("stomach_unknown_command".to_string()),
    };
    match response {
        Ok(value) => {
            print_json_line(&value);
            0
        }
        Err(err) => {
            print_json_line(&json_error("stomach_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn stomach_run_and_status_roundtrip() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");
        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);
        let status_exit = run(
            root.path(),
            &["status".to_string(), "--id=demo".to_string()],
        );
        assert_eq!(status_exit, 0);
    }
}
