// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative)
// SRS coverage marker: V4-DUAL-PRI-001
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::contract_lane_utils as lane_utils;
use crate::{clean, deterministic_receipt_hash, now_iso};
const DEFAULT_POLICY_REL: &str = "client/runtime/config/duality_seed_policy.json";
const DEFAULT_CODEX_REL: &str = "client/runtime/config/duality_codex.txt";
const DEFAULT_LATEST_REL: &str = "local/state/autonomy/duality/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/autonomy/duality/history.jsonl";
const TRIT_PAIN: i64 = -1;
const TRIT_UNKNOWN: i64 = 0;
const TRIT_OK: i64 = 1;
fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}
fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, false)
}
fn load_payload(argv: &[String]) -> Result<Value, String> {
    if let Some(payload) = parse_flag(argv, "payload") {
        return serde_json::from_str::<Value>(&payload)
            .map_err(|err| format!("duality_seed_payload_decode_failed:{err}"));
    }
    if let Some(path) = parse_flag(argv, "payload-file") {
        let text = fs::read_to_string(path.trim())
            .map_err(|err| format!("duality_seed_payload_file_read_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("duality_seed_payload_decode_failed:{err}"));
    }
    Err("duality_seed_missing_payload".to_string())
}
fn as_str(value: Option<&Value>) -> String {
    value
        .map(|v| match v {
            Value::String(s) => s.trim().to_string(),
            Value::Null => String::new(),
            _ => v.to_string().trim_matches('"').trim().to_string(),
        })
        .unwrap_or_default()
}
fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(fallback),
        Some(Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        _ => fallback,
    }
}
fn as_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}
fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}
fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}
fn normalize_token(raw: &str, max_len: usize) -> String {
    let text = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in text.chars() {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-');
        if allowed {
            out.push(ch);
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    trimmed.chars().take(max_len).collect()
}
fn normalize_word(raw: &str, max_len: usize) -> String {
    let text = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in text.chars() {
        let allowed = ch.is_ascii_alphanumeric();
        if allowed {
            out.push(ch);
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    trimmed.chars().take(max_len).collect()
}
fn clamp_i64(value: i64, lo: i64, hi: i64) -> i64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}
fn clamp_f64(value: f64, lo: f64, hi: f64) -> f64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}
fn normalize_trit(value: Option<&Value>) -> i64 {
    let n = as_f64(value).unwrap_or(0.0);
    if n > 0.0 {
        TRIT_OK
    } else if n < 0.0 {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    }
}
fn trit_label(trit: i64) -> &'static str {
    if trit > 0 {
        "ok"
    } else if trit < 0 {
        "pain"
    } else {
        "unknown"
    }
}
fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("duality_seed_create_dir_failed:{}:{err}", parent.display()))?;
    }
    Ok(())
}
fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("duality_seed_encode_json_failed:{err}"))?;
    let mut file = fs::File::create(&temp)
        .map_err(|err| format!("duality_seed_create_tmp_failed:{}:{err}", temp.display()))?;
    file.write_all(payload.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| format!("duality_seed_write_tmp_failed:{}:{err}", temp.display()))?;
    fs::rename(&temp, path)
        .map_err(|err| format!("duality_seed_rename_tmp_failed:{}:{err}", path.display()))
}
fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
        .map_err(|err| format!("duality_seed_append_jsonl_failed:{err}"))
}
fn read_json(path: &Path) -> Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return Value::Null;
    };
    serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)
}
fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
fn resolve_path(root: &Path, raw: &str, fallback_rel: &str) -> PathBuf {
    let token = clean_text(raw, 400);
    if token.is_empty() {
        return root.join(fallback_rel);
    }
    let candidate = PathBuf::from(token);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}
fn default_policy(root: &Path) -> Value {
    json!({
        "version": "1.0",
        "enabled": true,
        "shadow_only": true,
        "advisory_only": true,
        "advisory_weight": 0.35,
        "positive_threshold": 0.3,
        "negative_threshold": -0.2,
        "minimum_seed_confidence": 0.25,
        "contradiction_decay_step": 0.04,
        "support_recovery_step": 0.01,
        "max_observation_window": 200,
        "self_validation_interval_minutes": 360,
        "toll_enabled": true,
        "toll_trigger_negative_threshold": -0.2,
        "toll_debt_step": 0.2,
        "toll_recovery_step": 0.08,
        "toll_hard_block_threshold": 1.0,
        "codex_path": root.join(DEFAULT_CODEX_REL).to_string_lossy(),
        "state": {
            "latest_path": root.join(DEFAULT_LATEST_REL).to_string_lossy(),
            "history_path": root.join(DEFAULT_HISTORY_REL).to_string_lossy()
        },
        "integration": {
            "belief_formation": true,
            "inversion_trigger": true,
            "assimilation_candidacy": true,
            "task_decomposition": true,
            "weaver_arbitration": true,
            "heroic_echo_filtering": true,
            "web_tooling_truth_signal": true
        },
        "dual_voice": {
            "enabled": true,
            "min_harmony": 0.42,
            "minimum_voice_confidence": 0.3
        },
        "memory": {
            "tagging_enabled": true,
            "high_recall_threshold": 0.65,
            "inversion_flag_threshold": 0.35
        },
        "outputs": {
            "persist_shadow_receipts": true,
            "persist_observations": true,
            "persist_web_tooling_receipts": true
        }
    })
}
fn load_policy(root: &Path, policy_path_override: Option<&str>) -> Value {
    let policy_path = policy_path_override
        .map(|v| resolve_path(root, v, DEFAULT_POLICY_REL))
        .or_else(|| {
            std::env::var("DUALITY_SEED_POLICY_PATH")
                .ok()
                .as_deref()
                .map(|v| resolve_path(root, v, DEFAULT_POLICY_REL))
        })
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));
    let base = default_policy(root);
    let src = read_json(&policy_path);
    let src_obj = src.as_object().cloned().unwrap_or_default();
    let base_obj = base.as_object().cloned().unwrap_or_default();
    let base_state = base_obj
        .get("state")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_integration = base_obj
        .get("integration")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_outputs = base_obj
        .get("outputs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_dual_voice = base_obj
        .get("dual_voice")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_memory = base_obj
        .get("memory")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let src_state = src_obj
        .get("state")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let src_integration = src_obj
        .get("integration")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let src_outputs = src_obj
        .get("outputs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let src_dual_voice = src_obj
        .get("dual_voice")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let src_memory = src_obj
        .get("memory")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let codex_path_raw = {
        let candidate = as_str(src_obj.get("codex_path"));
        if candidate.is_empty() {
            as_str(base_obj.get("codex_path"))
        } else {
            candidate
        }
    };
    let latest_path_raw = {
        let candidate = as_str(src_state.get("latest_path"));
        if candidate.is_empty() {
            as_str(base_state.get("latest_path"))
        } else {
            candidate
        }
    };
    let history_path_raw = {
        let candidate = as_str(src_state.get("history_path"));
        if candidate.is_empty() {
            as_str(base_state.get("history_path"))
        } else {
            candidate
        }
    };
    let version = {
        let candidate = as_str(src_obj.get("version"));
        if candidate.is_empty() {
            "1.0".to_string()
        } else {
            candidate
        }
    };
    json!({
        "version": version,
        "enabled": as_bool(src_obj.get("enabled"), as_bool(base_obj.get("enabled"), true)),
        "shadow_only": as_bool(src_obj.get("shadow_only"), as_bool(base_obj.get("shadow_only"), true)),
        "advisory_only": as_bool(src_obj.get("advisory_only"), as_bool(base_obj.get("advisory_only"), true)),
        "advisory_weight": clamp_f64(as_f64(src_obj.get("advisory_weight")).unwrap_or(as_f64(base_obj.get("advisory_weight")).unwrap_or(0.35)), 0.0, 1.0),
        "positive_threshold": clamp_f64(as_f64(src_obj.get("positive_threshold")).unwrap_or(as_f64(base_obj.get("positive_threshold")).unwrap_or(0.3)), -1.0, 1.0),
        "negative_threshold": clamp_f64(as_f64(src_obj.get("negative_threshold")).unwrap_or(as_f64(base_obj.get("negative_threshold")).unwrap_or(-0.2)), -1.0, 1.0),
        "minimum_seed_confidence": clamp_f64(as_f64(src_obj.get("minimum_seed_confidence")).unwrap_or(as_f64(base_obj.get("minimum_seed_confidence")).unwrap_or(0.25)), 0.0, 1.0),
        "contradiction_decay_step": clamp_f64(as_f64(src_obj.get("contradiction_decay_step")).unwrap_or(as_f64(base_obj.get("contradiction_decay_step")).unwrap_or(0.04)), 0.0001, 1.0),
        "support_recovery_step": clamp_f64(as_f64(src_obj.get("support_recovery_step")).unwrap_or(as_f64(base_obj.get("support_recovery_step")).unwrap_or(0.01)), 0.0001, 1.0),
        "max_observation_window": clamp_i64(as_i64(src_obj.get("max_observation_window")).unwrap_or(as_i64(base_obj.get("max_observation_window")).unwrap_or(200)), 10, 20_000),
        "self_validation_interval_minutes": clamp_i64(as_i64(src_obj.get("self_validation_interval_minutes")).unwrap_or(as_i64(base_obj.get("self_validation_interval_minutes")).unwrap_or(360)), 5, 24 * 60),
        "toll_enabled": as_bool(src_obj.get("toll_enabled"), as_bool(base_obj.get("toll_enabled"), true)),
        "toll_trigger_negative_threshold": clamp_f64(
            as_f64(src_obj.get("toll_trigger_negative_threshold"))
                .unwrap_or(as_f64(base_obj.get("toll_trigger_negative_threshold")).unwrap_or(-0.2)),
            -1.0,
            1.0
        ),
        "toll_debt_step": clamp_f64(
            as_f64(src_obj.get("toll_debt_step"))
                .unwrap_or(as_f64(base_obj.get("toll_debt_step")).unwrap_or(0.2)),
            0.0001,
            10.0
        ),
        "toll_recovery_step": clamp_f64(
            as_f64(src_obj.get("toll_recovery_step"))
                .unwrap_or(as_f64(base_obj.get("toll_recovery_step")).unwrap_or(0.08)),
            0.0001,
            10.0
        ),
        "toll_hard_block_threshold": clamp_f64(
            as_f64(src_obj.get("toll_hard_block_threshold"))
                .unwrap_or(as_f64(base_obj.get("toll_hard_block_threshold")).unwrap_or(1.0)),
            0.1,
            100.0
        ),
        "codex_path": resolve_path(root, &codex_path_raw, DEFAULT_CODEX_REL).to_string_lossy(),
        "state": {
            "latest_path": resolve_path(root, &latest_path_raw, DEFAULT_LATEST_REL).to_string_lossy(),
            "history_path": resolve_path(root, &history_path_raw, DEFAULT_HISTORY_REL).to_string_lossy()
        },
        "integration": {
            "belief_formation": as_bool(src_integration.get("belief_formation"), as_bool(base_integration.get("belief_formation"), true)),
            "inversion_trigger": as_bool(src_integration.get("inversion_trigger"), as_bool(base_integration.get("inversion_trigger"), true)),
            "assimilation_candidacy": as_bool(src_integration.get("assimilation_candidacy"), as_bool(base_integration.get("assimilation_candidacy"), true)),
            "task_decomposition": as_bool(src_integration.get("task_decomposition"), as_bool(base_integration.get("task_decomposition"), true)),
            "weaver_arbitration": as_bool(src_integration.get("weaver_arbitration"), as_bool(base_integration.get("weaver_arbitration"), true)),
            "heroic_echo_filtering": as_bool(src_integration.get("heroic_echo_filtering"), as_bool(base_integration.get("heroic_echo_filtering"), true)),
            "web_tooling_truth_signal": as_bool(src_integration.get("web_tooling_truth_signal"), as_bool(base_integration.get("web_tooling_truth_signal"), true))
        },
        "dual_voice": {
            "enabled": as_bool(src_dual_voice.get("enabled"), as_bool(base_dual_voice.get("enabled"), true)),
            "min_harmony": clamp_f64(
                as_f64(src_dual_voice.get("min_harmony"))
                    .unwrap_or(as_f64(base_dual_voice.get("min_harmony")).unwrap_or(0.42)),
                0.0,
                1.0
            ),
            "minimum_voice_confidence": clamp_f64(
                as_f64(src_dual_voice.get("minimum_voice_confidence"))
                    .unwrap_or(as_f64(base_dual_voice.get("minimum_voice_confidence")).unwrap_or(0.3)),
                0.0,
                1.0
            )
        },
        "memory": {
            "tagging_enabled": as_bool(src_memory.get("tagging_enabled"), as_bool(base_memory.get("tagging_enabled"), true)),
            "high_recall_threshold": clamp_f64(
                as_f64(src_memory.get("high_recall_threshold"))
                    .unwrap_or(as_f64(base_memory.get("high_recall_threshold")).unwrap_or(0.65)),
                0.0,
                1.0
            ),
            "inversion_flag_threshold": clamp_f64(
                as_f64(src_memory.get("inversion_flag_threshold"))
                    .unwrap_or(as_f64(base_memory.get("inversion_flag_threshold")).unwrap_or(0.35)),
                0.0,
                1.0
            )
        },
        "outputs": {
            "persist_shadow_receipts": as_bool(src_outputs.get("persist_shadow_receipts"), as_bool(base_outputs.get("persist_shadow_receipts"), true)),
            "persist_observations": as_bool(src_outputs.get("persist_observations"), as_bool(base_outputs.get("persist_observations"), true)),
            "persist_web_tooling_receipts": as_bool(src_outputs.get("persist_web_tooling_receipts"), as_bool(base_outputs.get("persist_web_tooling_receipts"), true))
        }
    })
}
fn parse_attrs(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = BTreeSet::<String>::new();
    for row in raw.split(',') {
        let token = normalize_word(row, 60);
        if token.is_empty() {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
    out
}
fn default_codex() -> Value {
    json!({
        "version": "1.0",
        "flux_pairs": [
            {
                "yin": "order",
                "yang": "chaos",
                "yin_attrs": ["structure", "stability", "planning", "precision", "discipline"],
                "yang_attrs": ["energy", "variation", "exploration", "adaptation", "novelty"]
            },
            {
                "yin": "logic",
                "yang": "intuition",
                "yin_attrs": ["analysis", "proof", "verification", "determinism"],
                "yang_attrs": ["insight", "creativity", "synthesis", "leap"]
            },
            {
                "yin": "preservation",
                "yang": "transformation",
                "yin_attrs": ["safety", "containment", "resilience"],
                "yang_attrs": ["mutation", "inversion", "breakthrough"]
            }
        ],
        "flow_values": ["life/death", "progression/degression", "creation/decay", "integration/fragmentation"],
        "balance_rules": {
            "positive_balance": "creates_energy",
            "negative_balance": "destroys",
            "extreme_yin": "stagnation",
            "extreme_yang": "unraveling"
        },
        "asymptote": {
            "zero_point": "opposites_flow_into_each_other",
            "harmony": "balanced_interplay_enables_impossible"
        },
        "warnings": [
            "single_pole_optimization_causes_debt",
            "long_extremes_trigger_snapback",
            "protect_constitution_and_user_sovereignty"
        ]
    })
}
