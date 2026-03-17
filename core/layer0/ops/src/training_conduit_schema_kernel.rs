// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso, parse_args};

fn usage() {
    println!("training-conduit-schema-kernel commands:");
    println!("  protheus-ops training-conduit-schema-kernel default-policy --payload-base64=<json>");
    println!("  protheus-ops training-conduit-schema-kernel normalize-policy --payload-base64=<json>");
    println!("  protheus-ops training-conduit-schema-kernel load-policy --payload-base64=<json>");
    println!("  protheus-ops training-conduit-schema-kernel build-metadata --payload-base64=<json>");
    println!("  protheus-ops training-conduit-schema-kernel validate-metadata --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("training_conduit_schema_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("training_conduit_schema_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("training_conduit_schema_kernel_payload_utf8_decode_failed:{err}"))?;
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
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') {
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
            "id": "protheus_training_conduit_datum",
            "version": "1.0.0"
        },
        "defaults": {
            "owner_id": "local_operator",
            "owner_type": "human_operator",
            "license_id": "internal_protheus",
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
    let schema = src.get("schema").and_then(Value::as_object).cloned().unwrap_or_default();
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

fn normalize_delete_key(raw: Option<&Value>, fallback: &str) -> Value {
    let token = normalize_token(as_text(raw), 220);
    if !token.is_empty() {
        return Value::String(token);
    }
    let fallback = normalize_token(fallback, 220);
    if fallback.is_empty() {
        Value::Null
    } else {
        Value::String(fallback)
    }
}

fn validate_training_conduit_metadata(
    metadata: &Value,
    policy_input: Option<&Value>,
    root_dir: &Path,
) -> Value {
    let policy = normalize_policy(policy_input, root_dir);
    let defaults = policy
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let constraints = policy
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let m = metadata.as_object().cloned().unwrap_or_default();
    let source = m.get("source").and_then(Value::as_object).cloned().unwrap_or_default();
    let owner = m.get("owner").and_then(Value::as_object).cloned().unwrap_or_default();
    let license = m.get("license").and_then(Value::as_object).cloned().unwrap_or_default();
    let consent = m.get("consent").and_then(Value::as_object).cloned().unwrap_or_default();
    let retention = m.get("retention").and_then(Value::as_object).cloned().unwrap_or_default();
    let deletion = m.get("delete").and_then(Value::as_object).cloned().unwrap_or_default();

    let mut errors = Vec::<String>::new();
    if constraints.get("require_source").and_then(Value::as_bool).unwrap_or(true) {
        if normalize_token(as_text(source.get("system")), 120).is_empty() {
            errors.push("missing_source_system".to_string());
        }
        if normalize_token(as_text(source.get("channel")), 120).is_empty() {
            errors.push("missing_source_channel".to_string());
        }
    }
    if constraints.get("require_owner").and_then(Value::as_bool).unwrap_or(true)
        && normalize_token(as_text(owner.get("id")), 120).is_empty()
    {
        errors.push("missing_owner_id".to_string());
    }
    if constraints.get("require_license").and_then(Value::as_bool).unwrap_or(true)
        && normalize_token(as_text(license.get("id")), 160).is_empty()
    {
        errors.push("missing_license_id".to_string());
    }
    if constraints.get("require_consent").and_then(Value::as_bool).unwrap_or(true) {
        if normalize_consent_status(as_text(consent.get("status")), "").is_empty() {
            errors.push("missing_consent_status".to_string());
        }
        if normalize_consent_mode(as_text(consent.get("mode")), "").is_empty() {
            errors.push("missing_consent_mode".to_string());
        }
    }
    let min_retention = constraints
        .get("min_retention_days")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let max_retention = constraints
        .get("max_retention_days")
        .and_then(Value::as_i64)
        .unwrap_or(3650);
    let retention_days = clamp_int(retention.get("days"), min_retention, max_retention, -1);
    if retention_days < min_retention || retention_days > max_retention {
        errors.push("retention_days_out_of_range".to_string());
    }
    if constraints
        .get("require_delete_key")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && normalize_token(as_text(deletion.get("key")), 220).is_empty()
    {
        errors.push("missing_delete_key".to_string());
    }

    json!({
        "ok": errors.is_empty(),
        "errors": errors,
        "policy_version": as_text(policy.get("version")),
        "defaults_owner_id": as_text(defaults.get("owner_id"))
    })
}

fn build_training_conduit_metadata(
    input: Option<&Value>,
    policy_input: Option<&Value>,
    root_dir: &Path,
) -> Value {
    let policy = if let Some(policy) = policy_input {
        normalize_policy(Some(policy), root_dir)
    } else {
        load_policy(root_dir, &Map::new())
    };
    let defaults = policy
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let constraints = policy
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let schema = policy
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let input = input.and_then(Value::as_object).cloned().unwrap_or_default();

    let ts = {
        let raw = clean_text(as_text(input.get("ts")), 64);
        if raw.is_empty() { now_iso() } else { raw }
    };
    let source_system = {
        let value = normalize_token(
            if input.get("source_system").is_some() {
                as_text(input.get("source_system"))
            } else if input.get("system").is_some() {
                as_text(input.get("system"))
            } else {
                "unknown".to_string()
            },
            120,
        );
        if value.is_empty() { "unknown".to_string() } else { value }
    };
    let source_channel = {
        let value = normalize_token(
            if input.get("source_channel").is_some() {
                as_text(input.get("source_channel"))
            } else if input.get("channel").is_some() {
                as_text(input.get("channel"))
            } else {
                "unknown".to_string()
            },
            120,
        );
        if value.is_empty() { "unknown".to_string() } else { value }
    };
    let source_path = rel_path(
        root_dir,
        if input.get("source_path").is_some() {
            as_text(input.get("source_path"))
        } else {
            as_text(input.get("path"))
        },
    )
    .map(Value::String)
    .unwrap_or(Value::Null);
    let datum_id = {
        let value = normalize_token(
            if input.get("datum_id").is_some() {
                as_text(input.get("datum_id"))
            } else {
                as_text(input.get("record_id"))
            },
            180,
        );
        if value.is_empty() { Value::Null } else { Value::String(value) }
    };
    let provider = {
        let value = normalize_token(as_text(input.get("provider")), 120);
        if value.is_empty() { Value::Null } else { Value::String(value) }
    };
    let owner_id = {
        let value = normalize_token(
            if input.get("owner_id").is_some() {
                as_text(input.get("owner_id"))
            } else {
                as_text(defaults.get("owner_id"))
            },
            120,
        );
        if value.is_empty() { as_text(defaults.get("owner_id")) } else { value }
    };
    let owner_type = {
        let value = normalize_token(
            if input.get("owner_type").is_some() {
                as_text(input.get("owner_type"))
            } else {
                as_text(defaults.get("owner_type"))
            },
            80,
        );
        if value.is_empty() { as_text(defaults.get("owner_type")) } else { value }
    };
    let license_id = {
        let value = normalize_token(
            if input.get("license_id").is_some() {
                as_text(input.get("license_id"))
            } else {
                as_text(defaults.get("license_id"))
            },
            160,
        );
        if value.is_empty() { as_text(defaults.get("license_id")) } else { value }
    };
    let consent_status = normalize_consent_status(
        if input.get("consent_status").is_some() {
            as_text(input.get("consent_status"))
        } else {
            as_text(defaults.get("consent_status"))
        },
        &as_text(defaults.get("consent_status")),
    );
    let consent_mode = normalize_consent_mode(
        if input.get("consent_mode").is_some() {
            as_text(input.get("consent_mode"))
        } else {
            as_text(defaults.get("consent_mode"))
        },
        &as_text(defaults.get("consent_mode")),
    );
    let consent_evidence_ref = rel_path(
        root_dir,
        if input.get("consent_evidence_ref").is_some() {
            as_text(input.get("consent_evidence_ref"))
        } else {
            as_text(defaults.get("consent_evidence_ref"))
        },
    )
    .map(Value::String)
    .unwrap_or(Value::Null);
    let retention_days = clamp_int(
        input.get("retention_days"),
        constraints
            .get("min_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(1),
        constraints
            .get("max_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(3650),
        defaults
            .get("retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(365),
    );
    let delete_scope = {
        let value = normalize_token(
            if input.get("delete_scope").is_some() {
                as_text(input.get("delete_scope"))
            } else {
                as_text(defaults.get("delete_scope"))
            },
            120,
        );
        if value.is_empty() { as_text(defaults.get("delete_scope")) } else { value }
    };
    let fallback_delete_key = format!(
        "{}:{}:{}",
        source_system,
        source_channel,
        datum_id.as_str().unwrap_or(&Utc::now().timestamp_millis().to_string())
    );
    let delete_key = normalize_delete_key(input.get("delete_key"), &fallback_delete_key);
    let classification = {
        let value = normalize_token(
            if input.get("classification").is_some() {
                as_text(input.get("classification"))
            } else {
                as_text(defaults.get("classification"))
            },
            80,
        );
        if value.is_empty() { as_text(defaults.get("classification")) } else { value }
    };

    let metadata = json!({
        "schema_id": as_text(schema.get("id")),
        "schema_version": as_text(schema.get("version")),
        "policy_version": as_text(policy.get("version")),
        "ts": ts,
        "source": {
            "system": source_system,
            "channel": source_channel,
            "path": source_path,
            "datum_id": datum_id,
            "provider": provider
        },
        "owner": {
            "id": owner_id,
            "type": owner_type
        },
        "license": {
            "id": license_id
        },
        "consent": {
            "status": consent_status,
            "mode": consent_mode,
            "evidence_ref": consent_evidence_ref
        },
        "retention": {
            "days": retention_days,
            "expires_ts": retention_expiry(&ts, retention_days)
        },
        "delete": {
            "key": delete_key,
            "scope": delete_scope
        },
        "classification": classification
    });

    let mut metadata_obj = metadata.as_object().cloned().unwrap_or_default();
    metadata_obj.insert(
        "validation".to_string(),
        validate_training_conduit_metadata(&metadata, Some(&policy), root_dir),
    );
    Value::Object(metadata_obj)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());

    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        "default-policy" => match payload_json(argv) {
            Ok(payload) => {
                let root_dir = root_dir_from_payload(root, payload.as_object().unwrap_or(&Map::new()));
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_default_policy",
                    json!({ "policy": default_policy(&root_dir) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_default_policy",
                    &err,
                ));
                1
            }
        },
        "normalize-policy" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_normalize_policy",
                    json!({ "policy": normalize_policy(obj.get("policy"), &root_dir) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_normalize_policy",
                    &err,
                ));
                1
            }
        },
        "load-policy" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_load_policy",
                    json!({ "policy": load_policy(&root_dir, &obj) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_load_policy",
                    &err,
                ));
                1
            }
        },
        "build-metadata" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_build_metadata",
                    json!({
                        "metadata": build_training_conduit_metadata(obj.get("input"), obj.get("policy"), &root_dir)
                    }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_build_metadata",
                    &err,
                ));
                1
            }
        },
        "validate-metadata" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                let empty = json!({});
                let metadata = obj.get("metadata").unwrap_or(&empty);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_validate_metadata",
                    json!({
                        "validation": validate_training_conduit_metadata(
                            metadata,
                            obj.get("policy"),
                            &root_dir
                        )
                    }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_validate_metadata",
                    &err,
                ));
                1
            }
        },
        _ => {
            usage();
            print_json_line(&cli_error(
                "training_conduit_schema_kernel",
                "unknown_command",
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_policy_clamps_and_normalizes_defaults() {
        let root = PathBuf::from("/tmp/repo/client");
        let out = normalize_policy(
            Some(&json!({
                "defaults": {
                    "owner_id": " Team Lead ",
                    "retention_days": 99999,
                    "consent_status": "Granted"
                },
                "constraints": {
                    "min_retention_days": 5,
                    "max_retention_days": 90
                }
            })),
            &root,
        );
        assert_eq!(
            out.pointer("/defaults/owner_id").and_then(Value::as_str).unwrap_or(""),
            "team_lead"
        );
        assert_eq!(
            out.pointer("/defaults/retention_days").and_then(Value::as_i64).unwrap_or_default(),
            3650
        );
        assert_eq!(
            out.pointer("/defaults/consent_status").and_then(Value::as_str).unwrap_or(""),
            "granted"
        );
    }

    #[test]
    fn build_metadata_embeds_validation() {
        let root = PathBuf::from("/tmp/repo/client");
        let out = build_training_conduit_metadata(
            Some(&json!({
                "source_system": "discord",
                "source_channel": "ops",
                "datum_id": "abc-123"
            })),
            Some(&default_policy(&root)),
            &root,
        );
        assert_eq!(
            out.pointer("/source/system").and_then(Value::as_str).unwrap_or(""),
            "discord"
        );
        assert_eq!(
            out.pointer("/validation/ok").and_then(Value::as_bool).unwrap_or(false),
            true
        );
    }
}
