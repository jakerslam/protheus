// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::binary_vuln_plane (authoritative)

use crate::v8_kernel::{
    attach_conduit, build_conduit_enforcement, canonical_json_string, conduit_bypass_requested,
    load_json_or, parse_bool, read_json, scoped_state_root, sha256_hex_str, write_json,
    write_receipt,
};
use crate::{clean, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const STATE_ENV: &str = "BINARY_VULN_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "binary_vuln_plane";

const ENGINE_CONTRACT_PATH: &str =
    "planes/contracts/binary_vuln/binary_analysis_engine_contract_v1.json";
const MCP_CONTRACT_PATH: &str = "planes/contracts/binary_vuln/mcp_analysis_server_contract_v1.json";
const OUTPUT_CONTRACT_PATH: &str =
    "planes/contracts/binary_vuln/structured_output_contract_v1.json";
const RULEPACK_PATH: &str = "planes/contracts/binary_vuln/rulepack_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops binary-vuln-plane status");
    println!("  protheus-ops binary-vuln-plane scan --input=<path> [--rulepack=<path>] [--format=json|jsonl] [--strict=1|0]");
    println!("  protheus-ops binary-vuln-plane mcp-analyze --input=<path> [--transport=stdio|http-sse] [--rulepack=<path>] [--strict=1|0]");
    println!("  protheus-ops binary-vuln-plane rulepack-install --rulepack=<path> [--name=<id>] [--signature=<sig:...>] [--provenance=<uri>] [--strict=1|0]");
    println!("  protheus-ops binary-vuln-plane rulepack-enable --name=<id> [--strict=1|0]");
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

fn normalize_rulepack_name(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= 80 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed
    }
}

fn rulepack_root(root: &Path) -> PathBuf {
    state_root(root).join("rulepacks")
}

fn installed_rulepack_dir(root: &Path) -> PathBuf {
    rulepack_root(root).join("installed")
}

fn active_rulepack_path(root: &Path) -> PathBuf {
    rulepack_root(root).join("active.json")
}

fn strip_rulepack_signatures(mut rulepack: Value) -> Value {
    if let Some(obj) = rulepack.as_object_mut() {
        obj.remove("signature");
        if let Some(meta) = obj.get_mut("metadata").and_then(Value::as_object_mut) {
            meta.remove("signature");
        }
    }
    rulepack
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
                "type": "binary_vuln_plane_error",
                "error": clean(err, 240)
            }));
            1
        }
    }
}

fn status(root: &Path) -> Value {
    let installed_count = fs::read_dir(installed_rulepack_dir(root))
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|v| v.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("json"))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "binary_vuln_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "latest": read_json(&latest_path(root)),
        "rulepack": {
            "active": read_json(&active_rulepack_path(root)),
            "installed_count": installed_count
        },
        "observability": {
            "surface": "protheus-top",
            "cockpit_lane": "core/layer0/ops/hermes_plane"
        }
    })
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let mut claim_rows = vec![
        json!({
            "id": "V6-BINVULN-001.2",
            "claim": "binary_analysis_mcp_surface_is_conduit_routed_with_receipts",
            "evidence": {
                "action": clean(action, 120),
                "bypass_requested": bypass_requested
            }
        }),
        json!({
            "id": "V6-BINVULN-001.4",
            "claim": "binary_scan_execution_is_sandboxed_with_budget_privacy_and_degrade_guards_at_the_conduit_boundary",
            "evidence": {
                "action": clean(action, 120),
                "bypass_requested": bypass_requested
            }
        }),
    ];
    if action.starts_with("rulepack") {
        claim_rows.push(json!({
            "id": "V6-BINVULN-001.5",
            "claim": "rulepack_intake_and_enable_paths_are_conduit_gated_with_fail_closed_receipts",
            "evidence": {
                "action": clean(action, 120),
                "bypass_requested": bypass_requested
            }
        }));
    }
    if action == "scan" || action == "mcp-analyze" || action == "mcp_analyze" {
        claim_rows.push(json!({
            "id": "V6-BINVULN-001.6",
            "claim": "developer_cli_aliases_route_to_core_binary_scan_lanes_and_surface_observability_in_protheus_top",
            "evidence": {
                "action": clean(action, 120),
                "bypass_requested": bypass_requested
            }
        }));
    }
    build_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "binary_vuln_conduit_enforcement",
        "core/layer0/ops/binary_vuln_plane",
        bypass_requested,
        claim_rows,
    )
}

fn resolve_rel_or_abs(root: &Path, rel_or_abs: &str) -> PathBuf {
    if Path::new(rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    }
}

fn validate_rulepack(rulepack: &Value) -> Vec<String> {
    let mut errors = Vec::<String>::new();
    if rulepack
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("rulepack_version_must_be_v1".to_string());
    }
    if rulepack
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "binary_vuln_rulepack"
    {
        errors.push("rulepack_kind_invalid".to_string());
    }
    let rules = rulepack
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rules.is_empty() {
        errors.push("rulepack_rules_required".to_string());
    }
    for (idx, rule) in rules.iter().enumerate() {
        let prefix = format!("rule[{idx}]");
        let id = clean(
            rule.get("id").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        let pattern = clean(
            rule.get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            240,
        );
        let severity = clean(
            rule.get("severity")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            40,
        )
        .to_ascii_lowercase();
        if id.is_empty() {
            errors.push(format!("{prefix}_id_required"));
        }
        if pattern.is_empty() {
            errors.push(format!("{prefix}_pattern_required"));
        }
        if !matches!(severity.as_str(), "low" | "medium" | "high" | "critical") {
            errors.push(format!("{prefix}_severity_invalid"));
        }
    }
    errors
}

fn resolve_rulepack_path_from_active(root: &Path) -> Option<PathBuf> {
    let active = read_json(&active_rulepack_path(root))?;
    let path = active
        .get("installed_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let resolved = resolve_rel_or_abs(root, path);
    if resolved.exists() {
        Some(resolved)
    } else {
        None
    }
}

fn read_input_file(root: &Path, parsed: &crate::ParsedArgs) -> Result<(PathBuf, Vec<u8>), String> {
    let raw = parsed
        .flags
        .get("input")
        .cloned()
        .or_else(|| parsed.positional.get(1).cloned())
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Err("input_required".to_string());
    }
    let path = resolve_rel_or_abs(root, &raw);
    let bytes = fs::read(&path).map_err(|_| format!("input_not_found:{}", path.display()))?;
    if bytes.is_empty() {
        return Err("input_empty".to_string());
    }
    Ok((path, bytes))
}

fn detect_input_kind(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "bin" => "binary".to_string(),
        "efi" | "uefi" | "rom" => "uefi".to_string(),
        "ba2" => "ba2".to_string(),
        "bndb" => "binary_ninja_db".to_string(),
        "fw" | "firmware" => "firmware".to_string(),
        _ => "binary".to_string(),
    }
}

fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut freq = [0u64; 256];
    for byte in bytes {
        freq[*byte as usize] += 1;
    }
    let total = bytes.len() as f64;
    freq.iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let p = *count as f64 / total;
            -(p * p.log2())
        })
        .sum()
}

fn load_rulepack(root: &Path, parsed: &crate::ParsedArgs) -> (Value, Vec<Value>, String) {
    let mut path = parsed
        .flags
        .get("rulepack")
        .map(|v| resolve_rel_or_abs(root, v))
        .or_else(|| resolve_rulepack_path_from_active(root))
        .unwrap_or_else(|| resolve_rel_or_abs(root, RULEPACK_PATH));
    let rulepack = read_json(&path).unwrap_or_else(|| {
        path = resolve_rel_or_abs(root, RULEPACK_PATH);
        json!({
            "version": "v1",
            "kind": "binary_vuln_rulepack",
            "rules": []
        })
    });
    let rules = rulepack
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    (rulepack, rules, path.display().to_string())
}

fn scan_with_rules(
    raw_utf8: &str,
    kind: &str,
    bytes: &[u8],
    rules: &[Value],
    input_sha256: &str,
) -> Vec<Value> {
    let corpus = raw_utf8.to_ascii_lowercase();
    let mut findings = Vec::<Value>::new();

    for rule in rules {
        let id = clean(
            rule.get("id").and_then(Value::as_str).unwrap_or("rule"),
            120,
        );
        let pattern = clean(
            rule.get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            240,
        );
        if pattern.is_empty() {
            continue;
        }
        let pattern_lc = pattern.to_ascii_lowercase();
        let mut cursor = 0usize;
        while let Some(found) = corpus[cursor..].find(&pattern_lc) {
            let offset = cursor + found;
            findings.push(json!({
                "id": id,
                "title": clean(rule.get("title").and_then(Value::as_str).unwrap_or("rule_match"), 140),
                "severity": clean(rule.get("severity").and_then(Value::as_str).unwrap_or("medium"), 40),
                "kind": kind,
                "pattern": pattern,
                "offset": offset,
                "confidence": rule.get("confidence").and_then(Value::as_f64).unwrap_or(0.7),
                "policy_labels": rule.get("policy_labels").cloned().unwrap_or_else(|| json!(["security"])),
                "provenance_hash": sha256_hex_str(&format!("{}:{}:{}:{}", input_sha256, kind, id, offset))
            }));
            cursor = offset.saturating_add(pattern_lc.len());
            if cursor >= corpus.len() {
                break;
            }
        }
    }

    let entropy = shannon_entropy(bytes);
    if entropy > 7.3 {
        findings.push(json!({
            "id": "entropy_high",
            "title": "high entropy payload",
            "severity": "medium",
            "kind": kind,
            "pattern": "entropy",
            "offset": 0,
            "confidence": 0.55,
            "policy_labels": ["packed_binary", "requires_manual_review"],
            "provenance_hash": sha256_hex_str(&format!("{}:{}:entropy", input_sha256, kind)),
            "entropy": entropy
        }));
    }

    findings
}

fn normalize_findings(findings: Vec<Value>) -> Vec<Value> {
    findings
        .into_iter()
        .enumerate()
        .map(|(idx, mut finding)| {
            if finding.get("finding_id").is_none() {
                let id = clean(
                    finding
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("finding"),
                    120,
                );
                finding["finding_id"] = Value::String(format!("{}-{:04}", id, idx + 1));
            }
            if finding.get("policy_labels").is_none() {
                finding["policy_labels"] = json!(["security"]);
            }
            if finding.get("confidence").is_none() {
                finding["confidence"] = json!(0.5);
            }
            finding
        })
        .collect()
}

