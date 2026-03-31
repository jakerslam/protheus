// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

use crate::clean;
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn state_dir(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("security_plane")
}

fn capability_event_path(root: &Path) -> PathBuf {
    state_dir(root).join("capability_events.jsonl")
}

fn security_latest_path(root: &Path) -> PathBuf {
    state_dir(root).join("latest.json")
}

fn security_history_path(root: &Path) -> PathBuf {
    state_dir(root).join("history.jsonl")
}

fn scanner_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("scanner")
}

fn scanner_latest_path(root: &Path) -> PathBuf {
    scanner_state_dir(root).join("latest.json")
}

fn remediation_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("remediation")
}

fn remediation_gate_path(root: &Path) -> PathBuf {
    remediation_state_dir(root).join("promotion_gate.json")
}

fn blast_radius_events_path(root: &Path) -> PathBuf {
    state_dir(root).join("blast_radius_events.jsonl")
}

fn secrets_state_path(root: &Path) -> PathBuf {
    state_dir(root).join("secrets_federation.json")
}

fn secrets_events_path(root: &Path) -> PathBuf {
    state_dir(root).join("secrets_events.jsonl")
}

fn proofs_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("proofs")
}

fn proofs_latest_path(root: &Path) -> PathBuf {
    proofs_state_dir(root).join("latest.json")
}

fn proofs_history_path(root: &Path) -> PathBuf {
    proofs_state_dir(root).join("history.jsonl")
}

fn audit_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("audit")
}

fn audit_latest_path(root: &Path) -> PathBuf {
    audit_state_dir(root).join("latest.json")
}

fn audit_history_path(root: &Path) -> PathBuf {
    audit_state_dir(root).join("history.jsonl")
}

fn threat_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("threat_model")
}

fn threat_latest_path(root: &Path) -> PathBuf {
    threat_state_dir(root).join("latest.json")
}

fn threat_history_path(root: &Path) -> PathBuf {
    threat_state_dir(root).join("history.jsonl")
}

fn contracts_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("contracts")
}

fn contract_state_path(root: &Path, id: &str) -> PathBuf {
    contracts_state_dir(root).join(format!("{id}.json"))
}

fn contract_history_path(root: &Path) -> PathBuf {
    contracts_state_dir(root).join("history.jsonl")
}

fn skill_quarantine_state_path(root: &Path) -> PathBuf {
    state_dir(root).join("skill_quarantine.json")
}

fn skill_quarantine_events_path(root: &Path) -> PathBuf {
    state_dir(root).join("skill_quarantine_events.jsonl")
}

fn skills_plane_state_root(root: &Path) -> PathBuf {
    if let Ok(value) = std::env::var("SKILLS_PLANE_STATE_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("skills_plane")
}

fn skills_registry_path(root: &Path) -> PathBuf {
    skills_plane_state_root(root).join("registry.json")
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, false)
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.as_deref(), fallback)
}

fn parse_u64(raw: Option<String>, fallback: u64) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn parse_subcommand(argv: &[String], fallback: &str) -> String {
    argv.iter()
        .find(|token| !token.starts_with("--"))
        .map(|token| clean(token, 64).to_ascii_lowercase())
        .unwrap_or_else(|| fallback.to_string())
}

fn append_jsonl(path: &Path, row: &Value) {
    let _ = lane_utils::append_jsonl(path, row);
}

fn write_json(path: &Path, payload: &Value) {
    let _ = lane_utils::write_json(path, payload);
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn persist_security_receipt(root: &Path, payload: &Value) {
    let _ = lane_utils::write_json(&security_latest_path(root), payload);
    let _ = lane_utils::append_jsonl(&security_history_path(root), payload);
}

fn run_security_contract_command(
    root: &Path,
    argv: &[String],
    strict: bool,
    command: &str,
    contract_id: &str,
    checks: &[(&str, Option<&str>)],
) -> (Value, i32) {
    let mut missing = Vec::<String>::new();
    let mut mismatch = Vec::<String>::new();
    let mut provided = serde_json::Map::<String, Value>::new();
    for (key, expected) in checks {
        let got = lane_utils::parse_flag(argv, key, false);
        if let Some(value) = got.as_deref() {
            provided.insert((*key).to_string(), Value::String(clean(value, 200)));
        } else {
            missing.push((*key).to_string());
            continue;
        }
        if let Some(expected_value) = expected {
            if got
                .as_deref()
                .map(|v| v.trim().eq_ignore_ascii_case(expected_value))
                .unwrap_or(false)
            {
                continue;
            }
            mismatch.push(format!("{key}:{expected_value}"));
        }
    }

    let ok = missing.is_empty() && mismatch.is_empty();
    let contract_state = json!({
        "id": contract_id,
        "command": command,
        "strict": strict,
        "updated_at": now_iso(),
        "missing_flags": missing,
        "mismatch_flags": mismatch,
        "provided_flags": provided
    });
    let path = contract_state_path(root, contract_id);
    let _ = lane_utils::write_json(&path, &contract_state);
    let _ = lane_utils::append_jsonl(
        &contract_history_path(root),
        &json!({
            "ts": now_iso(),
            "id": contract_id,
            "command": command,
            "ok": ok,
            "strict": strict
        }),
    );

    let out = json!({
        "ok": ok,
        "type": "security_plane_contract_lane",
        "lane": "core/layer1/security",
        "mode": command,
        "strict": strict,
        "contract_id": contract_id,
        "state_path": path.display().to_string(),
        "missing_flags": contract_state.get("missing_flags").cloned().unwrap_or(Value::Null),
        "mismatch_flags": contract_state.get("mismatch_flags").cloned().unwrap_or(Value::Null),
        "claim_evidence": [{
            "id": contract_id,
            "claim": "security_contract_lane_executes_with_fail_closed_validation_and_receipted_state_artifacts",
            "evidence": {
                "command": command,
                "state_path": path.display().to_string(),
                "missing_flags": contract_state.get("missing_flags").cloned().unwrap_or(Value::Null),
                "mismatch_flags": contract_state.get("mismatch_flags").cloned().unwrap_or(Value::Null)
            }
        }]
    });
    let exit = if strict && !ok { 2 } else { 0 };
    (out, exit)
}

fn split_csv(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|row| clean(row, 160).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn canonicalize_for_prefix_check(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    if let Some(parent) = path.parent() {
        if let Ok(canonical_parent) = fs::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return canonical_parent.join(name);
            }
            return canonical_parent;
        }
    }
    path.to_path_buf()
}

fn run_skill_install_path_enforcer(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let Some(raw_path) = parse_flag(argv, "skill-path") else {
        let out = json!({
            "ok": false,
            "type": "security_plane_skill_install_path_enforcer",
            "strict": strict,
            "error": "skill_path_required",
            "claim_evidence": [{
                "id": "V6-SEC-SKILL-PATH-001",
                "claim": "skill_install_paths_are_enforced_to_approved_roots_with_fail_closed_guardrails",
                "evidence": {"skill_path_present": false}
            }]
        });
        return (out, if strict { 2 } else { 0 });
    };

    let raw_candidate = PathBuf::from(raw_path.trim());
    let candidate = if raw_candidate.is_absolute() {
        raw_candidate
    } else {
        root.join(raw_candidate)
    };
    let candidate_norm = canonicalize_for_prefix_check(&candidate);

    let mut allowed_roots = vec![
        root.join("client")
            .join("runtime")
            .join("systems")
            .join("skills")
            .join("packages"),
        root.join("local")
            .join("workspace")
            .join("assistant")
            .join("skills"),
    ];
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        let trimmed = codex_home.trim();
        if !trimmed.is_empty() {
            allowed_roots.push(PathBuf::from(trimmed).join("skills"));
        }
    }
    for extra in split_csv(parse_flag(argv, "extra-allowed-root")) {
        let path = PathBuf::from(extra);
        allowed_roots.push(if path.is_absolute() {
            path
        } else {
            root.join(path)
        });
    }
    let normalized_roots = allowed_roots
        .iter()
        .map(|path| canonicalize_for_prefix_check(path))
        .collect::<Vec<_>>();

    let allowed = normalized_roots
        .iter()
        .any(|prefix| candidate_norm.starts_with(prefix));

    let out = json!({
        "ok": allowed,
        "type": "security_plane_skill_install_path_enforcer",
        "strict": strict,
        "skill_path": candidate_norm.display().to_string(),
        "allowed": allowed,
        "allowed_roots": normalized_roots
            .iter()
            .map(|path| Value::String(path.display().to_string()))
            .collect::<Vec<_>>(),
        "claim_evidence": [{
            "id": "V6-SEC-SKILL-PATH-001",
            "claim": "skill_install_paths_are_enforced_to_approved_roots_with_fail_closed_guardrails",
            "evidence": {
                "skill_path": candidate_norm.display().to_string(),
                "allowed": allowed
            }
        }]
    });
    (out, if strict && !allowed { 2 } else { 0 })
}
