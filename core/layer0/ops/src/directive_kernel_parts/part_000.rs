// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::v8_kernel::{
    append_jsonl, keyed_digest_hex, parse_bool, print_json, read_json, scoped_state_root,
    sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "DIRECTIVE_KERNEL_STATE_ROOT";
const STATE_SCOPE: &str = "directive_kernel";
const SIGNING_ENV: &str = "DIRECTIVE_KERNEL_SIGNING_KEY";
const DIRECTIVES_SUBDIR: [&str; 4] = ["client", "runtime", "config", "directives"];
#[path = "../directive_kernel_run.rs"]
mod directive_kernel_run;
fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn vault_path(root: &Path) -> PathBuf {
    state_root(root).join("prime_directive_vault.json")
}

fn directives_dir(root: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    for segment in DIRECTIVES_SUBDIR {
        out.push(segment);
    }
    out
}

fn active_directives_path(root: &Path) -> PathBuf {
    directives_dir(root).join("ACTIVE.yaml")
}

fn yaml_to_json(text: &str) -> Result<Value, String> {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(text).map_err(|err| format!("directive_yaml_parse_failed:{err}"))?;
    serde_json::to_value(parsed).map_err(|err| format!("directive_yaml_encode_failed:{err}"))
}

fn yaml_timebound_signal_present(parsed: &Value, raw_text: &str) -> bool {
    let keywords = [
        "timeframe",
        "deadline",
        "target_date",
        "target-date",
        "review_by",
        "review-by",
        "horizon",
        "month",
        "months",
        "year",
        "years",
        "quarter",
    ];
    let raw = raw_text.to_ascii_lowercase();
    if keywords.iter().any(|keyword| raw.contains(keyword)) {
        return true;
    }

    fn scan(value: &Value, keywords: &[&str]) -> bool {
        match value {
            Value::Object(map) => map.iter().any(|(key, value)| {
                let key_norm = key.to_ascii_lowercase();
                keywords.iter().any(|keyword| key_norm.contains(keyword)) || scan(value, keywords)
            }),
            Value::Array(rows) => rows.iter().any(|row| scan(row, keywords)),
            Value::String(text) => {
                let norm = text.to_ascii_lowercase();
                keywords.iter().any(|keyword| norm.contains(keyword))
            }
            Value::Number(number) => number.as_i64().map(|value| value > 0).unwrap_or(false),
            _ => false,
        }
    }

    scan(parsed, &keywords)
}

fn validate_tier1_directive_quality(content: &str, directive_id: &str) -> Value {
    let parsed = yaml_to_json(content).unwrap_or_else(|_| Value::Object(Map::new()));
    let obj = parsed.as_object();
    let empty = Map::new();
    let root = obj.unwrap_or(&empty);
    let intent = root
        .get("intent")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let constraints = root
        .get("constraints")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let success = root
        .get("success_metrics")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let scope = root
        .get("scope")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let approval = root
        .get("approval_policy")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    let mut missing = Vec::<String>::new();
    let mut questions = Vec::<String>::new();

    let intent_primary = intent
        .get("primary")
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !intent_primary {
        missing.push("intent.primary".to_string());
        questions.push("What is the single specific objective (intent.primary)?".to_string());
    }

    let definitions = intent.get("definitions");
    let definitions_present = definitions
        .and_then(Value::as_object)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if !definitions_present
        || !yaml_timebound_signal_present(definitions.unwrap_or(&Value::Null), content)
    {
        missing.push("intent.definitions_timebound".to_string());
        questions.push("What explicit time-bound target or review horizon applies?".to_string());
    }

    let included = scope
        .get("included")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if !included {
        missing.push("scope.included".to_string());
        questions.push("What is explicitly in scope for this directive?".to_string());
    }

    let excluded = scope
        .get("excluded")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if !excluded {
        missing.push("scope.excluded".to_string());
        questions.push("What is explicitly out of scope for this directive?".to_string());
    }

    let risk_limits = constraints
        .get("risk_limits")
        .and_then(Value::as_object)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if !risk_limits {
        missing.push("constraints.risk_limits".to_string());
        questions.push(
            "What quantitative risk limits apply (drawdown, burn, position size)?".to_string(),
        );
    }

    let leading = success
        .get("leading")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if !leading {
        missing.push("success_metrics.leading".to_string());
        questions.push("Which leading indicators will be used to measure progress?".to_string());
    }

    let lagging = success
        .get("lagging")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if !lagging {
        missing.push("success_metrics.lagging".to_string());
        questions.push("Which lagging metrics define end-state success?".to_string());
    }

    let additional_gates = approval
        .get("additional_gates")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if !additional_gates {
        missing.push("approval_policy.additional_gates".to_string());
        questions
            .push("Which additional approval gates are required for Tier 1 actions?".to_string());
    }

    json!({
        "ok": missing.is_empty(),
        "directive_id": clean(directive_id, 128),
        "missing": missing,
        "questions": questions
    })
}

fn load_active_directives(
    root: &Path,
    allow_missing: bool,
    allow_weak_tier1: bool,
) -> Result<Vec<Value>, String> {
    let active_path = active_directives_path(root);
    if !active_path.exists() {
        return Err(format!(
            "active_directives_missing:{}",
            active_path.display()
        ));
    }

    let active_content = fs::read_to_string(&active_path)
        .map_err(|err| format!("active_directives_read_failed:{err}"))?;
    let active = yaml_to_json(&active_content)?;
    let active_rows = active
        .get("active_directives")
        .and_then(Value::as_array)
        .ok_or_else(|| "active_directives_array_missing".to_string())?;

    let directives_root = directives_dir(root);
    let mut loaded = Vec::<Value>::new();
    let mut missing = Vec::<Value>::new();
    for row in active_rows {
        let Some(entry) = row.as_object() else {
            continue;
        };
        let status = entry
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active")
            .trim()
            .to_ascii_lowercase();
        if status != "active" {
            continue;
        }
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .map(|value| clean(value, 160))
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let tier = entry.get("tier").and_then(Value::as_i64).unwrap_or(99);
        let file_name = if id.ends_with(".yaml") {
            id.clone()
        } else {
            format!("{id}.yaml")
        };
        let file_path = directives_root.join(&file_name);
        if !file_path.exists() {
            if allow_missing {
                continue;
            }
            missing.push(json!({
                "id": id,
                "file": file_name,
                "path": file_path.display().to_string()
            }));
            continue;
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|err| format!("directive_read_failed:{}:{err}", file_path.display()))?;
        if tier == 1 {
            let quality = validate_tier1_directive_quality(&content, &id);
            if !quality.get("ok").and_then(Value::as_bool).unwrap_or(false) && !allow_weak_tier1 {
                let missing_lines = quality
                    .get("missing")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(Value::as_str)
                            .map(|value| format!("  - {value}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let question_lines = quality
                    .get("questions")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(Value::as_str)
                            .map(|value| format!("  - {value}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                return Err(format!(
                    "tier1_directive_quality_failed:{id}\n{missing_lines}\n{question_lines}"
                ));
            }
        }

        let directive = yaml_to_json(&content)?;
        loaded.push(json!({
            "id": id,
            "tier": tier,
            "status": status,
            "data": directive
        }));
    }

    if !missing.is_empty() && !allow_missing {
        return Err(format!(
            "active_directives_missing_files:{}",
            serde_json::to_string(&missing).unwrap_or_else(|_| "[]".to_string())
        ));
    }

    loaded.sort_by_key(|row| row.get("tier").and_then(Value::as_i64).unwrap_or(99));
    Ok(loaded)
}

fn merge_active_constraints(directives: &[Value]) -> Value {
    let mut hard_blocks = Vec::<Value>::new();
    let mut approval_required = Vec::<Value>::new();
    let mut risk_limits = Map::<String, Value>::new();
    let mut high_stakes_seen = HashSet::<String>::new();
    let mut high_stakes_domains = Vec::<Value>::new();

    for directive in directives {
        let Some(data) = directive.get("data").and_then(Value::as_object) else {
            continue;
        };
        let directive_tier = directive.get("tier").and_then(Value::as_i64).unwrap_or(99);
        if let Some(rows) = data.get("hard_blocks").and_then(Value::as_array) {
            for row in rows {
                let Some(obj) = row.as_object() else {
                    continue;
                };
                let Some(rule) = obj.get("rule").and_then(Value::as_str) else {
                    continue;
                };
                hard_blocks.push(json!({
                    "rule": clean(rule, 160),
                    "description": clean(
                        obj.get("description").and_then(Value::as_str).unwrap_or(rule),
                        240
                    ),
                    "tier": obj.get("tier").and_then(Value::as_i64).unwrap_or(directive_tier),
                    "patterns": obj.get("patterns").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
                }));
            }
        }
        if let Some(rows) = data.get("approval_required").and_then(Value::as_array) {
            for row in rows {
                let Some(obj) = row.as_object() else {
                    continue;
                };
                let Some(rule) = obj.get("rule").and_then(Value::as_str) else {
                    continue;
                };
                approval_required.push(json!({
                    "rule": clean(rule, 160),
                    "description": clean(
                        obj.get("description").and_then(Value::as_str).unwrap_or(rule),
                        240
                    ),
                    "tier": obj.get("tier").and_then(Value::as_i64).unwrap_or(directive_tier),
                    "examples": obj.get("examples").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
                }));
            }
        }
        if let Some(rows) = data.get("high_stakes_domains").and_then(Value::as_array) {
            for row in rows {
                let Some(obj) = row.as_object() else {
                    continue;
                };
                let Some(domain) = obj.get("domain").and_then(Value::as_str) else {
                    continue;
                };
                if !obj
                    .get("escalation_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }
                let domain_norm = clean(domain, 160).to_ascii_lowercase();
                if high_stakes_seen.insert(domain_norm.clone()) {
                    high_stakes_domains.push(Value::String(domain_norm));
                }
            }
        }
        if let Some(rows) = data.get("directives").and_then(Value::as_array) {
            for row in rows {
                let Some(obj) = row.as_object() else {
                    continue;
                };
                let Some(id) = obj.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let Some(constraints) = obj.get("constraints").and_then(Value::as_object) else {
                    continue;
                };
                if constraints.is_empty() {
                    continue;
                }
                risk_limits.insert(clean(id, 160), Value::Object(constraints.clone()));
            }
        }
    }

    json!({
        "tier": 0,
        "hard_blocks": hard_blocks,
        "approval_required": approval_required,
        "risk_limits": Value::Object(risk_limits),
        "high_stakes_domains": high_stakes_domains
    })
}

fn payload_contains_secret_token(payload: &str, marker: &str, min_len: usize) -> bool {
    let bytes = payload.as_bytes();
    let marker_bytes = marker.as_bytes();
    let mut idx = 0usize;
    while idx + marker_bytes.len() <= bytes.len() {
        if &bytes[idx..idx + marker_bytes.len()] != marker_bytes {
            idx += 1;
            continue;
        }
        let mut count = 0usize;
        let mut cursor = idx + marker_bytes.len();
        while cursor < bytes.len() {
            let ch = bytes[cursor] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                count += 1;
                cursor += 1;
                continue;
            }
            break;
        }
        if count >= min_len {
            return true;
        }
        idx = cursor;
    }
    false
}
