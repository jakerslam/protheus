// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const PLACEHOLDER_TYPES: &[&str] = &[
    "", "unknown", "new", "queued", "pending", "proposal", "item", "generic",
];

fn usage() {
    println!("proposal-type-classifier-kernel commands:");
    println!("  protheus-ops proposal-type-classifier-kernel normalize-type-key [--payload-base64=<json>]");
    println!("  protheus-ops proposal-type-classifier-kernel extract-source-eye-id [--payload-base64=<json>]");
    println!("  protheus-ops proposal-type-classifier-kernel classify [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out =
        json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({"ok": false, "type": kind, "ts": ts, "date": ts[..10].to_string(), "error": error, "fail_closed": true});
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
            .map_err(|err| format!("proposal_type_classifier_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("proposal_type_classifier_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("proposal_type_classifier_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("proposal_type_classifier_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').trim().to_string(),
    }
}

fn normalize_type_key(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-') {
            ch
        } else {
            '_'
        };
        if mapped == '_' {
            if prev_us || out.is_empty() {
                continue;
            }
            prev_us = true;
            out.push(mapped);
        } else {
            prev_us = false;
            out.push(mapped);
        }
        if out.len() >= 64 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn is_usable_type(raw: &str) -> bool {
    let key = normalize_type_key(raw);
    !key.is_empty() && !PLACEHOLDER_TYPES.contains(&key.as_str())
}

fn extract_source_eye_id(proposal: &Map<String, Value>) -> String {
    let meta = proposal
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let direct = normalize_text(meta.get("source_eye"));
    if !direct.is_empty() {
        return direct;
    }
    for row in proposal
        .get("evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let reference = normalize_text(row.get("evidence_ref"));
        if let Some(captures) = regex::Regex::new(r"(?i)\beye:([^\s|]+)")
            .unwrap()
            .captures(&reference)
        {
            if let Some(m) = captures.get(1) {
                let value = m.as_str().trim().to_string();
                if !value.is_empty() {
                    return value;
                }
            }
        }
    }
    String::new()
}

fn infer_type_from_signals(source_eye: &str, text_blob: &str) -> &'static str {
    let eye = normalize_type_key(source_eye);
    let text = text_blob.to_ascii_lowercase();
    if eye.contains("memory_embedding")
        || eye.contains("capability_provider")
        || eye.contains("provider_runtime")
    {
        return "memory_embedding_provider_contract";
    }
    if matches!(eye.as_str(), "directive_pulse" | "directive_compiler") {
        return "directive_clarification";
    }
    if matches!(
        eye.as_str(),
        "local_state_fallback" | "local_state_digest" | "tier1_exception"
    ) {
        return "local_state_fallback";
    }
    if eye.contains("moltbook")
        || eye.contains("upwork")
        || eye.contains("bird_x")
        || eye.contains("x_")
    {
        return "external_intel";
    }
    let has_collector =
        regex::Regex::new(r"\b(collector|eye|sensor|feed|ingest|crawler|scrap|parser)\b")
            .unwrap()
            .is_match(&text);
    let has_repair = regex::Regex::new(r"\b(fail|failure|error|timeout|retry|recover|restor|remediation|broken|degraded|down|fix)\b").unwrap().is_match(&text);
    if has_collector && has_repair {
        return "collector_remediation";
    }
    if regex::Regex::new(r"\b(directive|objective|tier|scope|clarif|decompose|lineage)\b")
        .unwrap()
        .is_match(&text)
    {
        return "directive_clarification";
    }
    if regex::Regex::new(r"\b(campaign|strategy|portfolio|sequenc|roadmap|big[-\s]?bet)\b")
        .unwrap()
        .is_match(&text)
    {
        return "strategy";
    }
    if regex::Regex::new(r"\b(memory embedding|memory_embedding|capability provider|provider runtime|contracts\.memoryembeddingproviders)\b")
        .unwrap()
        .is_match(&text)
    {
        return "memory_embedding_provider_contract";
    }
    if regex::Regex::new(r"\b(opportunity|outreach|lead|sales|bizdev|revenue|freelance|contract|gig|client|rfp|reply|interview|proposal draft)\b").unwrap().is_match(&text) { return "external_intel"; }
    if regex::Regex::new(r"\b(governance|routing|autonomy|spine|memory|reflex|spawn|security|integrity|queue|budget|attestation)\b").unwrap().is_match(&text) { return "local_state_fallback"; }
    if !eye.is_empty() && eye != "unknown_eye" {
        return "external_intel";
    }
    "local_state_fallback"
}

fn classify(proposal: &Map<String, Value>, opts: &Map<String, Value>) -> Value {
    let meta = proposal
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (source, candidate) in [
        ("proposal.type", proposal.get("type")),
        ("meta.type", meta.get("type")),
        ("fallback_type", opts.get("fallback_type")),
    ] {
        let raw = normalize_text(candidate);
        if is_usable_type(&raw) {
            return json!({ "type": normalize_type_key(&raw), "inferred": false, "source": source });
        }
    }
    let source_eye = {
        let extracted = extract_source_eye_id(proposal);
        if extracted.is_empty() {
            normalize_text(opts.get("source_eye"))
        } else {
            extracted
        }
    };
    let text_blob = [
        proposal.get("title"),
        proposal.get("summary"),
        proposal.get("notes"),
        proposal.get("suggested_next_command"),
        proposal.get("expected_impact"),
        meta.get("trigger"),
        meta.get("normalized_objective"),
        meta.get("normalized_expected_outcome"),
        meta.get("normalized_validation_metric"),
    ]
    .into_iter()
    .map(normalize_text)
    .filter(|row| !row.is_empty())
    .collect::<Vec<_>>()
    .join(" ");
    let inferred = infer_type_from_signals(&source_eye, &text_blob);
    let type_key = {
        let key = normalize_type_key(inferred);
        if key.is_empty() {
            "local_state_fallback".to_string()
        } else {
            key
        }
    };
    json!({
        "type": type_key,
        "inferred": true,
        "source": if source_eye.is_empty() { "infer:proposal_text".to_string() } else { format!("infer:{source_eye}") },
    })
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("proposal_type_classifier_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "normalize-type-key" => cli_receipt(
            "proposal_type_classifier_kernel_normalize_type_key",
            json!({ "ok": true, "type_key": normalize_type_key(&normalize_text(input.get("value").or_else(|| input.get("type")))) }),
        ),
        "extract-source-eye-id" => cli_receipt(
            "proposal_type_classifier_kernel_extract_source_eye_id",
            json!({ "ok": true, "source_eye": extract_source_eye_id(input.get("proposal").and_then(Value::as_object).unwrap_or(input)) }),
        ),
        "classify" => cli_receipt(
            "proposal_type_classifier_kernel_classify",
            json!({ "ok": true, "classification": classify(input.get("proposal").and_then(Value::as_object).unwrap_or(input), input.get("opts").and_then(Value::as_object).unwrap_or(&Map::new())) }),
        ),
        _ => cli_error(
            "proposal_type_classifier_kernel_error",
            &format!("unknown_command:{command}"),
        ),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&result);
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_strategy_text() {
        let result = classify(
            &serde_json::from_value(json!({"summary":"campaign roadmap sequencing"})).unwrap(),
            &Map::new(),
        );
        assert_eq!(result["type"], json!("strategy"));
        assert_eq!(result["inferred"], json!(true));
    }

    #[test]
    fn classifies_memory_embedding_provider_contract_text() {
        let result = classify(
            &serde_json::from_value(json!({
                "summary":"enforce contracts.memoryEmbeddingProviders parity for provider runtime"
            }))
            .unwrap(),
            &Map::new(),
        );
        assert_eq!(result["type"], json!("memory_embedding_provider_contract"));
        assert_eq!(result["inferred"], json!(true));
    }
}
