// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("success-criteria-compiler-kernel commands:");
    println!(
        "  protheus-ops success-criteria-compiler-kernel compile-rows --payload-base64=<json>"
    );
    println!(
        "  protheus-ops success-criteria-compiler-kernel compile-proposal --payload-base64=<json>"
    );
    println!("  protheus-ops success-criteria-compiler-kernel to-action-spec-rows --payload-base64=<json>");
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
        return serde_json::from_str::<Value>(&raw).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_decode_failed:{err}")
        });
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_decode_failed:{err}")
        });
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: OnceLock<Vec<Value>> = OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn normalize_text(value: Option<&Value>) -> String {
    as_str(value)
}

fn normalize_spaces_str(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = normalize_spaces_str(&normalize_text(value));
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_spaces(value: Option<&Value>) -> String {
    normalize_spaces_str(&normalize_text(value))
}

fn normalize_capability_key(value: Option<&Value>) -> String {
    normalize_spaces(value).to_ascii_lowercase()
}

fn outreach_capability_hint_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b(opportunity|outreach|lead|sales|bizdev|revenue|freelance|contract|gig|external_intel|client|prospect)\b").unwrap()
    })
}

fn capability_allows_outreach(capability_key: &str) -> bool {
    if capability_key.is_empty() {
        return true;
    }
    if capability_key.starts_with("proposal:") {
        return outreach_capability_hint_re().is_match(capability_key);
    }
    true
}

fn remap_metric_for_capability(metric: &str, capability_key: &str) -> String {
    let norm_metric = normalize_spaces_str(metric).to_ascii_lowercase();
    if !capability_allows_outreach(capability_key)
        && (norm_metric == "reply_or_interview_count" || norm_metric == "outreach_artifact")
    {
        return "artifact_count".to_string();
    }
    if norm_metric.is_empty() {
        "execution_success".to_string()
    } else {
        norm_metric
    }
}

fn first_int_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+)\b").unwrap())
}

fn parse_first_int(text: &str, fallback: i64) -> i64 {
    first_int_re()
        .captures(text)
        .and_then(|m| m.get(1))
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn parse_comparator(text: &str, fallback: &str) -> &'static str {
    let lower = text.to_ascii_lowercase();
    if Regex::new(
        r"(?:<=|≤|\bat most\b|\bwithin\b|\bunder\b|\bbelow\b|\bmax(?:imum)?\b|\bless than\b)",
    )
    .unwrap()
    .is_match(&lower)
    {
        return "lte";
    }
    if Regex::new(r"(?:>=|≥|\bat least\b|\bover\b|\babove\b|\bminimum\b|\bmin\b|\bmore than\b)")
        .unwrap()
        .is_match(&lower)
    {
        return "gte";
    }
    if fallback == "lte" {
        "lte"
    } else {
        "gte"
    }
}

fn duration_limit_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(\d+(?:\.\d+)?)\s*(ms|msec|millisecond(?:s)?|s|sec|secs|second(?:s)?|m|min|mins|minute(?:s)?)",
        )
        .unwrap()
    })
}

fn parse_duration_limit_ms(text: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let captures = duration_limit_re().captures(&lower)?;
    let mut value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    let unit = captures.get(2)?.as_str();
    if matches!(unit, "m" | "min" | "mins") || unit.starts_with("minute") {
        value *= 60.0 * 1000.0;
    } else if matches!(unit, "s" | "sec" | "secs") || unit.starts_with("second") {
        value *= 1000.0;
    }
    Some(value.round() as i64)
}

fn token_limit_re_a() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+(?:\.\d+)?)\s*(k|m)?\s*tokens?").unwrap())
}

fn token_limit_re_b() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"tokens?\s*(?:<=|≥|>=|≤|<|>|=|at most|at least|under|over|below|above|within|max(?:imum)?|min(?:imum)?)?\s*(\d+(?:\.\d+)?)(?:\s*(k|m))?")
            .unwrap()
    })
}

fn parse_token_limit(text: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let captures = token_limit_re_a()
        .captures(&lower)
        .or_else(|| token_limit_re_b().captures(&lower))?;
    let mut value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    let suffix = captures.get(2).map(|m| m.as_str()).unwrap_or("");
    if suffix == "k" {
        value *= 1000.0;
    } else if suffix == "m" {
        value *= 1_000_000.0;
    }
    Some(value.round() as i64)
}

fn horizon_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\b(\d+\s*(?:h|hr|hour|hours|d|day|days|w|week|weeks|min|mins|minute|minutes|run|runs))\b",
        )
        .unwrap()
    })
}

fn parse_horizon(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if let Some(captures) = horizon_re().captures(&lower) {
        if let Some(m) = captures.get(1) {
            return normalize_spaces_str(m.as_str());
        }
    }
    if lower.contains("next run") {
        return "next run".to_string();
    }
    if lower.contains("next 2 runs") || lower.contains("next 2 run") {
        return "2 runs".to_string();
    }
    if lower.contains("24h") {
        return "24h".to_string();
    }
    if lower.contains("48h") {
        return "48h".to_string();
    }
    if lower.contains("7d") {
        return "7d".to_string();
    }
    String::new()
}

fn normalize_target(metric: &str, target_text: &str, horizon_text: &str) -> String {
    let text = normalize_spaces_str(&format!("{target_text} {horizon_text}").to_ascii_lowercase());
    match metric {
        "execution_success" => "execution success".to_string(),
        "postconditions_ok" => "postconditions pass".to_string(),
        "queue_outcome_logged" => "outcome receipt logged".to_string(),
        "artifact_count" => {
            let comparator = parse_comparator(&text, "gte");
            let threshold = parse_first_int(&text, 1);
            format!(
                "{}{} artifact",
                if comparator == "lte" { "<=" } else { ">=" },
                threshold
            )
        }
        "outreach_artifact" => {
            let comparator = parse_comparator(&text, "gte");
            let threshold = parse_first_int(&text, 1);
            format!(
                "{}{} outreach artifact",
                if comparator == "lte" { "<=" } else { ">=" },
                threshold
            )
        }
        "reply_or_interview_count" => {
            let comparator = parse_comparator(&text, "gte");
            let threshold = parse_first_int(&text, 1);
            format!(
                "{}{} reply/interview signal",
                if comparator == "lte" { "<=" } else { ">=" },
                threshold
            )
        }
        "entries_count" => {
            let comparator = parse_comparator(&text, "gte");
            let threshold = parse_first_int(&text, 1);
            format!(
                "{}{} entries",
                if comparator == "lte" { "<=" } else { ">=" },
                threshold
            )
        }
        "revenue_actions_count" => {
            let comparator = parse_comparator(&text, "gte");
            let threshold = parse_first_int(&text, 1);
            format!(
                "{}{} revenue actions",
                if comparator == "lte" { "<=" } else { ">=" },
                threshold
            )
        }
        "token_usage" => {
            let comparator = parse_comparator(&text, "lte");
            let limit = parse_token_limit(&text).unwrap_or(1200);
            format!(
                "tokens {}{}",
                if comparator == "gte" { ">=" } else { "<=" },
                limit
            )
        }
        "duration_ms" => {
            let comparator = parse_comparator(&text, "lte");
            let limit = parse_duration_limit_ms(&text).unwrap_or(15000);
            format!(
                "duration {}{}ms",
                if comparator == "gte" { ">=" } else { "<=" },
                limit
            )
        }
        _ => {
            let normalized = normalize_spaces_str(target_text);
            if normalized.is_empty() {
                "execution success".to_string()
            } else {
                normalized
            }
        }
    }
}

fn classify_metric(metric_text: &str, target_text: &str, source_text: &str) -> String {
    let metric = normalize_spaces_str(metric_text).to_ascii_lowercase();
    let text = normalize_spaces_str(&format!("{metric_text} {target_text} {source_text}"))
        .to_ascii_lowercase();

    if metric.is_empty() && (text.contains("reply") || text.contains("interview")) {
        return "reply_or_interview_count".to_string();
    }
    if metric.is_empty()
        && text.contains("outreach")
        && ["artifact", "draft", "offer", "proposal"]
            .iter()
            .any(|token| text.contains(token))
    {
        return "outreach_artifact".to_string();
    }

    match metric.as_str() {
        "validation_metric" | "validation_check" | "verification_metric" | "verification_check" => {
            "postconditions_ok".to_string()
        }
        "outreach_artifact" => "outreach_artifact".to_string(),
        "reply_or_interview_count"
        | "reply_count"
        | "interview_count"
        | "outreach_reply_count"
        | "outreach_interview_count" => "reply_or_interview_count".to_string(),
        "artifact_count"
        | "experiment_artifact"
        | "collector_success_runs"
        | "hypothesis_signal_lift"
        | "outreach_artifact_count"
        | "offer_draft_count"
        | "proposal_draft_count" => "artifact_count".to_string(),
        "verification_checks_passed" | "postconditions_ok" => "postconditions_ok".to_string(),
        "collector_failure_streak" | "queue_outcome_logged" => "queue_outcome_logged".to_string(),
        "entries_count" => "entries_count".to_string(),
        "revenue_actions_count" => "revenue_actions_count".to_string(),
        "latency" | "duration" | "time" | "elapsed_ms" | "elapsed" => "duration_ms".to_string(),
        "token_usage" => "token_usage".to_string(),
        "duration_ms" => "duration_ms".to_string(),
        "execution_success" => "execution_success".to_string(),
        _ => {
            if text.contains("reply") || text.contains("interview") {
                "reply_or_interview_count".to_string()
            } else if text.contains("outreach")
                && ["artifact", "draft", "offer", "proposal"]
                    .iter()
                    .any(|token| text.contains(token))
            {
                "outreach_artifact".to_string()
            } else if [
                "artifact",
                "draft",
                "experiment",
                "patch",
                "plan",
                "deliverable",
            ]
            .iter()
            .any(|token| text.contains(token))
            {
                "artifact_count".to_string()
            } else if [
                "postcondition",
                "contract",
                "verify",
                "verification",
                "check pass",
            ]
            .iter()
            .any(|token| text.contains(token))
            {
                "postconditions_ok".to_string()
            } else if ["receipt", "evidence", "queue outcome", "logged"]
                .iter()
                .any(|token| text.contains(token))
            {
                "queue_outcome_logged".to_string()
            } else if text.contains("revenue") {
                "revenue_actions_count".to_string()
            } else if ["entries", "entry", "notes"]
                .iter()
                .any(|token| text.contains(token))
            {
                "entries_count".to_string()
            } else if text.contains("token") {
                "token_usage".to_string()
            } else if [
                "latency",
                "duration",
                "time",
                "ms",
                "msec",
                "millisecond",
                "second",
                "sec",
                "min",
                "minute",
            ]
            .iter()
            .any(|token| text.contains(token))
            {
                "duration_ms".to_string()
            } else {
                "execution_success".to_string()
            }
        }
    }
}

fn normalize_input_rows(
    rows: Option<&Value>,
    source: &str,
) -> Vec<(String, String, String, String)> {
    let src = if source.trim().is_empty() {
        "success_criteria".to_string()
    } else {
        source.trim().to_string()
    };
    let mut out = Vec::new();
    for row in as_array(rows) {
        if let Some(text) = row.as_str() {
            let target = normalize_spaces_str(text);
            if !target.is_empty() {
                out.push((src.clone(), String::new(), target, String::new()));
            }
            continue;
        }
        let Some(obj) = row.as_object() else {
            continue;
        };
        let metric = normalize_spaces(obj.get("metric").or_else(|| obj.get("name")));
        let target = normalize_spaces(
            obj.get("target")
                .or_else(|| obj.get("threshold"))
                .or_else(|| obj.get("description"))
                .or_else(|| obj.get("goal")),
        );
        let horizon = normalize_spaces(
            obj.get("horizon")
                .or_else(|| obj.get("window"))
                .or_else(|| obj.get("by")),
        );
        if metric.is_empty() && target.is_empty() && horizon.is_empty() {
            continue;
        }
        out.push((src.clone(), metric, target, horizon));
    }
    out
}

pub(crate) fn compile_success_criteria_rows(rows: Option<&Value>, source: &str) -> Vec<Value> {
    let raw_rows = normalize_input_rows(rows, source);
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (row_source, row_metric, row_target, row_horizon) in raw_rows {
        let metric = classify_metric(&row_metric, &row_target, &row_source);
        let horizon = if row_horizon.is_empty() {
            parse_horizon(&row_target)
        } else {
            row_horizon
        };
        let target = normalize_target(&metric, &row_target, &horizon);
        let key = format!("{metric}|{target}|{horizon}|{row_source}").to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(json!({
            "source": row_source,
            "metric": metric,
            "target": target,
            "horizon": horizon,
            "measurable": true
        }));
    }
    out
}

pub(crate) fn compile_proposal_success_criteria(payload: &Map<String, Value>) -> Vec<Value> {
    let proposal = as_object(payload.get("proposal"))
        .cloned()
        .unwrap_or_default();
    let action_spec = as_object(proposal.get("action_spec"))
        .cloned()
        .unwrap_or_default();
    let opts = as_object(payload.get("opts")).cloned().unwrap_or_default();
    let include_verify = opts
        .get("include_verify")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let include_validation = opts
        .get("include_validation")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let allow_fallback = opts
        .get("allow_fallback")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let capability_key = normalize_capability_key(opts.get("capability_key"));

    let mut compiled = Vec::new();
    compiled.extend(compile_success_criteria_rows(
        proposal.get("success_criteria"),
        "success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.get("success_criteria"),
        "action_spec.success_criteria",
    ));
    if include_verify {
        compiled.extend(compile_success_criteria_rows(
            action_spec.get("verify"),
            "action_spec.verify",
        ));
    }
    if include_validation {
        compiled.extend(compile_success_criteria_rows(
            proposal.get("validation"),
            "validation",
        ));
    }

    if compiled.is_empty() && allow_fallback {
        compiled.push(json!({
            "source": "compiler_fallback",
            "metric": "execution_success",
            "target": "execution success",
            "horizon": "",
            "measurable": true
        }));
    }

    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in compiled {
        let metric = remap_metric_for_capability(&as_str(row.get("metric")), &capability_key);
        let horizon = normalize_spaces(row.get("horizon"));
        let target = normalize_target(&metric, &as_str(row.get("target")), &horizon);
        let source = {
            let normalized = normalize_spaces(row.get("source"));
            if normalized.is_empty() {
                "success_criteria".to_string()
            } else {
                normalized
            }
        };
        let key = format!("{source}|{metric}|{target}|{horizon}").to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(json!({
            "source": source,
            "metric": metric,
            "target": target,
            "horizon": horizon,
            "measurable": true
        }));
    }
    out
}

fn to_action_spec_rows(rows: Option<&Value>) -> Vec<Value> {
    as_array(rows)
        .iter()
        .map(|row| {
            let metric = {
                let metric = as_str(row.get("metric"));
                if metric.is_empty() {
                    "execution_success".to_string()
                } else {
                    metric
                }
            };
            let target = {
                let target = as_str(row.get("target"));
                if target.is_empty() {
                    "execution success".to_string()
                } else {
                    target
                }
            };
            json!({
                "metric": metric,
                "target": target,
                "horizon": normalize_spaces(row.get("horizon"))
            })
        })
        .collect()
}

fn run_command(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "compile-rows" => {
            let source = clean_text(payload.get("source"), 120);
            Ok(json!({
                "ok": true,
                "rows": compile_success_criteria_rows(payload.get("rows"), if source.is_empty() { "success_criteria" } else { &source })
            }))
        }
        "compile-proposal" => Ok(json!({
            "ok": true,
            "rows": compile_proposal_success_criteria(payload)
        })),
        "to-action-spec-rows" => Ok(json!({
            "ok": true,
            "rows": to_action_spec_rows(payload.get("rows"))
        })),
        _ => Err("success_criteria_compiler_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("success_criteria_compiler_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("success_criteria_compiler_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("success_criteria_compiler_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_rows_detects_token_and_duration_metrics() {
        let payload = json!([
            { "metric": "latency", "target": "under 5 s", "horizon": "next run" },
            { "metric": "token usage", "target": "at most 1.2k tokens" }
        ]);
        let rows = compile_success_criteria_rows(Some(&payload), "success_criteria");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("metric").and_then(Value::as_str),
            Some("duration_ms")
        );
        assert_eq!(
            rows[1].get("target").and_then(Value::as_str),
            Some("tokens <=1200")
        );
    }

    #[test]
    fn compile_proposal_remaps_outreach_metrics_for_non_outreach_capability() {
        let payload = json!({
            "proposal": {
                "success_criteria": [
                    { "metric": "reply_or_interview_count", "target": ">=1 interview signal" },
                    { "metric": "reply_or_interview_count", "target": ">=1 interview signal" }
                ]
            },
            "opts": {
                "capability_key": "proposal:maintenance_patch",
                "allow_fallback": false
            }
        });
        let rows = compile_proposal_success_criteria(payload_obj(&payload));
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("metric").and_then(Value::as_str),
            Some("artifact_count")
        );
    }
}
