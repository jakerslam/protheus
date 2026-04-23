// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{now_iso, parse_args};

fn usage() {
    println!("training-conduit-schema-kernel commands:");
    println!(
        "  infring-ops training-conduit-schema-kernel default-policy --payload-base64=<json>"
    );
    println!(
        "  infring-ops training-conduit-schema-kernel normalize-policy --payload-base64=<json>"
    );
    println!("  infring-ops training-conduit-schema-kernel load-policy --payload-base64=<json>");
    println!(
        "  infring-ops training-conduit-schema-kernel build-metadata --payload-base64=<json>"
    );
    println!(
        "  infring-ops training-conduit-schema-kernel validate-metadata --payload-base64=<json>"
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("training_conduit_schema_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("training_conduit_schema_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("training_conduit_schema_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("training_conduit_schema_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn as_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn clean_text(raw: impl ToString, max_len: usize) -> String {
    let mut out = raw.to_string().trim().to_string();
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_token(raw: impl ToString, max_len: usize) -> String {
    let text = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut previous_underscore = false;
    for ch in text.chars() {
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-')
        {
            previous_underscore = false;
            ch
        } else {
            if previous_underscore {
                continue;
            }
            previous_underscore = true;
            '_'
        };
        out.push(normalized);
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let parsed = match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(v)) => v.trim().parse::<i64>().ok(),
        Some(Value::Bool(v)) => Some(if *v { 1 } else { 0 }),
        _ => None,
    }
    .unwrap_or(fallback);
    parsed.clamp(lo, hi)
}

fn root_dir_from_payload(repo_root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = clean_text(as_text(payload.get("root_dir")), 400);
    if raw.is_empty() {
        return repo_root.to_path_buf();
    }
    if Path::new(&raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        repo_root.join(raw)
    }
}

fn default_policy_path(root_dir: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("TRAINING_CONDUIT_POLICY_PATH") {
        let raw = clean_text(raw, 400);
        if !raw.is_empty() {
            if Path::new(&raw).is_absolute() {
                return PathBuf::from(raw);
            }
            return root_dir.join(raw);
        }
    }
    root_dir.join("config").join("training_conduit_policy.json")
}

fn rel_path(root_dir: &Path, raw: impl ToString) -> Option<String> {
    let raw = clean_text(raw, 400);
    if raw.is_empty() {
        return None;
    }
    let resolved = if Path::new(&raw).is_absolute() {
        PathBuf::from(&raw)
    } else {
        root_dir.join(&raw)
    };
    let rel = resolved
        .strip_prefix(root_dir)
        .map(PathBuf::from)
        .unwrap_or(resolved);
    Some(rel.to_string_lossy().replace('\\', "/"))
}

fn normalize_consent_status(raw: impl ToString, fallback: &str) -> String {
    let token = normalize_token(raw, 40);
    if matches!(token.as_str(), "granted" | "denied" | "revoked" | "unknown") {
        token
    } else {
        let normalized_fallback = normalize_token(fallback, 40);
        if normalized_fallback.is_empty() {
            "unknown".to_string()
        } else {
            normalized_fallback
        }
    }
}

fn normalize_consent_mode(raw: impl ToString, fallback: &str) -> String {
    let token = normalize_token(raw, 60);
    if matches!(
        token.as_str(),
        "explicit_opt_in"
            | "operator_policy"
            | "contractual"
            | "public_domain"
            | "internal_system"
            | "unknown"
    ) {
        token
    } else {
        let normalized_fallback = normalize_token(fallback, 60);
        if normalized_fallback.is_empty() {
            "unknown".to_string()
        } else {
            normalized_fallback
        }
    }
}

fn default_policy(root_dir: &Path) -> Value {
    json!({
        "version": "1.0",
        "schema": {
            "id": "infring_training_conduit_datum",
            "version": "1.0.0"
        },
        "defaults": {
            "owner_id": "local_operator",
            "owner_type": "human_operator",
            "license_id": "internal_infring",
            "consent_status": "granted",
            "consent_mode": "operator_policy",
            "consent_evidence_ref": rel_path(root_dir, "config/training_conduit_policy.json").unwrap_or_else(|| "config/training_conduit_policy.json".to_string()),
            "retention_days": 365,
            "delete_scope": "training_conduit",
            "classification": "internal"
        },
        "constraints": {
            "min_retention_days": 1,
            "max_retention_days": 3650,
            "require_source": true,
            "require_owner": true,
            "require_license": true,
            "require_consent": true,
            "require_delete_key": true
        }
    })
}

fn normalize_policy(raw: Option<&Value>, root_dir: &Path) -> Value {
    let base = default_policy(root_dir);
    let src = raw.and_then(Value::as_object).cloned().unwrap_or_default();
    let schema = src
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let defaults = src
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let constraints = src
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_defaults = base
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_constraints = base
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_schema = base
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let version = {
        let value = clean_text(as_text(src.get("version")), 40);
        if value.is_empty() {
            as_text(base.get("version"))
        } else {
            value
        }
    };
    let schema_id = {
        let value = clean_text(as_text(schema.get("id")), 120);
        if value.is_empty() {
            as_text(base_schema.get("id"))
        } else {
            value
        }
    };
    let schema_version = {
        let value = clean_text(as_text(schema.get("version")), 40);
        if value.is_empty() {
            as_text(base_schema.get("version"))
        } else {
            value
        }
    };
    let owner_id = {
        let value = normalize_token(as_text(defaults.get("owner_id")), 120);
        if value.is_empty() {
            as_text(base_defaults.get("owner_id"))
        } else {
            value
        }
    };
    let owner_type = {
        let value = normalize_token(as_text(defaults.get("owner_type")), 80);
        if value.is_empty() {
            as_text(base_defaults.get("owner_type"))
        } else {
            value
        }
    };
    let license_id = {
        let value = normalize_token(as_text(defaults.get("license_id")), 160);
        if value.is_empty() {
            as_text(base_defaults.get("license_id"))
        } else {
            value
        }
    };
    let consent_status = normalize_consent_status(
        as_text(defaults.get("consent_status")),
        &as_text(base_defaults.get("consent_status")),
    );
    let consent_mode = normalize_consent_mode(
        as_text(defaults.get("consent_mode")),
        &as_text(base_defaults.get("consent_mode")),
    );
    let consent_evidence_ref = rel_path(
        root_dir,
        if defaults.get("consent_evidence_ref").is_some() {
            as_text(defaults.get("consent_evidence_ref"))
        } else {
            as_text(base_defaults.get("consent_evidence_ref"))
        },
    )
    .unwrap_or_else(|| as_text(base_defaults.get("consent_evidence_ref")));
    let retention_days = clamp_int(
        defaults.get("retention_days"),
        1,
        3650,
        base_defaults
            .get("retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(365),
    );
    let delete_scope = {
        let value = normalize_token(as_text(defaults.get("delete_scope")), 120);
        if value.is_empty() {
            as_text(base_defaults.get("delete_scope"))
        } else {
            value
        }
    };
    let classification = {
        let value = normalize_token(as_text(defaults.get("classification")), 80);
        if value.is_empty() {
            as_text(base_defaults.get("classification"))
        } else {
            value
        }
    };
    let min_retention_days = clamp_int(
        constraints.get("min_retention_days"),
        1,
        3650,
        base_constraints
            .get("min_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(1),
    );
    let max_retention_days = clamp_int(
        constraints.get("max_retention_days"),
        1,
        3650 * 3,
        base_constraints
            .get("max_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(3650),
    );
    let require_source = constraints
        .get("require_source")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_owner = constraints
        .get("require_owner")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_license = constraints
        .get("require_license")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_consent = constraints
        .get("require_consent")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_delete_key = constraints
        .get("require_delete_key")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    json!({
        "version": version,
        "schema": {
            "id": schema_id,
            "version": schema_version
        },
        "defaults": {
            "owner_id": owner_id,
            "owner_type": owner_type,
            "license_id": license_id,
            "consent_status": consent_status,
            "consent_mode": consent_mode,
            "consent_evidence_ref": consent_evidence_ref,
            "retention_days": retention_days,
            "delete_scope": delete_scope,
            "classification": classification
        },
        "constraints": {
            "min_retention_days": min_retention_days,
            "max_retention_days": max_retention_days,
            "require_source": require_source,
            "require_owner": require_owner,
            "require_license": require_license,
            "require_consent": require_consent,
            "require_delete_key": require_delete_key
        }
    })
}

fn load_policy(root_dir: &Path, payload: &Map<String, Value>) -> Value {
    let policy_path = {
        let raw = clean_text(as_text(payload.get("policy_path")), 400);
        if raw.is_empty() {
            default_policy_path(root_dir)
        } else if Path::new(&raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            root_dir.join(raw)
        }
    };
    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| default_policy(root_dir));
    normalize_policy(Some(&raw), root_dir)
}

fn retention_expiry(ts: &str, days: i64) -> Value {
    let Ok(parsed) = DateTime::parse_from_rfc3339(ts) else {
        return Value::Null;
    };
    let expiry = parsed.with_timezone(&Utc) + Duration::days(days);
    Value::String(expiry.to_rfc3339())
}
