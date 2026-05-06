use super::eval_research_golden_utils::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) struct CaseGrade {
    pub(super) score: u64,
    pub(super) pass: bool,
    pub(super) excellent: bool,
    pub(super) gates: BTreeMap<String, bool>,
    pub(super) dimension_scores: BTreeMap<String, u64>,
    pub(super) failures: Vec<String>,
    pub(super) response_text: String,
    pub(super) empty_response: bool,
    pub(super) raw_tool_leak: bool,
    pub(super) tool_choice_final_response: bool,
    pub(super) unsupported_claim: bool,
}

pub(super) fn grade_case(
    case: &Value,
    payload: &Value,
    pass_score: u64,
    excellent_score: u64,
) -> CaseGrade {
    let response_text = assistant_text(payload);
    let normalized = normalize_for_compare(&response_text);
    let required_entities = string_array_at(case, &["required_entities"]);
    let gates = gate_results(case, payload);
    let raw_tool_leak = raw_tool_payload_leak(&response_text);
    let internal_leak = internal_workflow_leak(&response_text);
    let tool_choice_final_response = tool_choice_as_final_response(&response_text);
    let empty_response = response_text.trim().is_empty();
    let unsupported_claim = unsupported_claim_signal(case, &response_text);
    let source_signal = has_source_signal(payload, &response_text);
    let limitation_signal = has_limitation_signal(&normalized);
    let final_answer_present = !empty_response && response_text.split_whitespace().count() >= 20;
    let entity_coverage = entity_coverage(&normalized, &required_entities);

    let workflow_score = gates.values().filter(|ok| **ok).count() as u64 * 5;
    let evidence_score = (if source_signal { 7 } else { 0 })
        + (if !raw_tool_leak { 6 } else { 0 })
        + (if limitation_signal { 6 } else { 0 })
        + (if !unsupported_claim { 6 } else { 0 });
    let synthesis_score = (if final_answer_present { 8 } else { 0 })
        + ((entity_coverage * 9.0).round() as u64)
        + (if has_tradeoff_or_structure(&normalized) {
            8
        } else {
            0
        })
        + (if has_recommendation_signal(&normalized) {
            5
        } else {
            0
        })
        + (if limitation_signal { 5 } else { 0 });
    let projection_score = (if !raw_tool_leak { 5 } else { 0 })
        + (if !internal_leak { 5 } else { 0 })
        + (if !empty_response { 5 } else { 0 })
        + (if normal_prose_signal(&response_text) {
            5
        } else {
            0
        });
    let mut dimension_scores = BTreeMap::new();
    dimension_scores.insert("workflow_path".to_string(), workflow_score.min(20));
    dimension_scores.insert("evidence_behavior".to_string(), evidence_score.min(25));
    dimension_scores.insert("synthesis_quality".to_string(), synthesis_score.min(35));
    dimension_scores.insert("projection_safety".to_string(), projection_score.min(20));
    let score = dimension_scores.values().sum::<u64>().min(100);
    let mut failures = Vec::new();
    if !gates.values().all(|ok| *ok) {
        failures.push("workflow_gate_path_incomplete".to_string());
    }
    if empty_response {
        failures.push("empty_research_response".to_string());
    }
    if !source_signal {
        failures.push("missing_evidence_or_source_signal".to_string());
    }
    if entity_coverage < 0.75 {
        failures.push(format!("entity_coverage_low:{entity_coverage:.2}"));
    }
    if raw_tool_leak {
        failures.push("raw_tool_payload_leaked".to_string());
    }
    if internal_leak {
        failures.push("internal_workflow_state_leaked".to_string());
    }
    if tool_choice_final_response {
        failures.push("tool_choice_visible_as_final_response".to_string());
    }
    if unsupported_claim {
        failures.push("unsupported_overconfident_claim_signal".to_string());
    }
    if score < pass_score {
        failures.push(format!("research_score_below_pass:{score}<{pass_score}"));
    }
    failures.sort();
    failures.dedup();
    CaseGrade {
        score,
        pass: score >= pass_score && failures.is_empty(),
        excellent: score >= excellent_score && failures.is_empty(),
        gates,
        dimension_scores,
        failures,
        response_text,
        empty_response,
        raw_tool_leak,
        tool_choice_final_response,
        unsupported_claim,
    }
}

pub(super) fn response_diagnostics(payload: &Value, response_text: &str) -> Value {
    json!({
        "top_keys": payload
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "pending_tool_request": pending_tool_request(payload).cloned().unwrap_or(Value::Null),
        "tools_present": has_tool_execution(payload),
        "provider": payload.get("provider").and_then(Value::as_str),
        "model": payload.get("model").and_then(Value::as_str),
        "runtime_model": payload.get("runtime_model").and_then(Value::as_str),
        "initial_invoke_error": payload.get("initial_invoke_error").and_then(Value::as_bool),
        "error": payload
            .get("error")
            .and_then(Value::as_str)
            .map(sanitize_backend_error),
        "transport_error": payload.get("transport_error").and_then(Value::as_str),
        "stderr": payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 500)),
        "response_empty": response_text.trim().is_empty(),
        "final_llm_status": payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
    })
}

fn sanitize_backend_error(raw: &str) -> String {
    let mut cleaned = clean_text(raw, 800);
    let lower = cleaned.to_ascii_lowercase();
    let marker = "incorrect api key provided:";
    if let Some(idx) = lower.find(marker) {
        let secret_start = idx + marker.len();
        let secret_end = cleaned[secret_start..]
            .find('.')
            .map(|offset| secret_start + offset)
            .unwrap_or_else(|| cleaned.len());
        cleaned.replace_range(secret_start..secret_end, " [redacted]");
    }
    cleaned
}

pub(super) fn gate_rate_rows(
    total_counts: &BTreeMap<String, u64>,
    pass_counts: &BTreeMap<String, u64>,
    min_rate: f64,
) -> Vec<Value> {
    total_counts
        .iter()
        .map(|(gate, total)| {
            let passed = *pass_counts.get(gate).unwrap_or(&0);
            let rate = ratio(passed, *total);
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": rate,
                "min_rate": min_rate,
                "ok": rate >= min_rate
            })
        })
        .collect()
}

pub(super) fn dimension_average_rows(
    totals: &BTreeMap<String, u64>,
    total_cases: u64,
) -> Vec<Value> {
    totals
        .iter()
        .map(|(dimension, total)| {
            json!({
                "dimension": dimension,
                "average": ratio(*total, total_cases)
            })
        })
        .collect()
}

fn gate_results(case: &Value, payload: &Value) -> BTreeMap<String, bool> {
    let mut gates = BTreeMap::new();
    let serialized = payload.to_string().to_ascii_lowercase();
    let tool_request = pending_tool_request(payload);
    let expected_gate_2 =
        normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_2"], ""));
    let expected_gate_3 =
        normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_3"], ""));
    let required_gate_4_fields =
        string_array_at(case, &["expected_gate_path", "gate_4_required_fields"]);
    let gate_2 = expected_gate_2.is_empty()
        || tool_request
            .map(|request| {
                let family = normalize_for_compare(&format!(
                    "{} {}",
                    str_at(request, &["selected_tool_family"], ""),
                    str_at(request, &["selected_tool_label"], "")
                ));
                (family.contains("web") || family.contains("research"))
                    && (family.contains("search") || family.contains("fetch"))
            })
            .unwrap_or_else(|| {
                (serialized.contains("web") || serialized.contains("research"))
                    && (serialized.contains("search") || serialized.contains("fetch"))
            });
    let gate_3 = expected_gate_3.is_empty()
        || tool_request
            .map(|request| {
                let tool_name = normalize_for_compare(&format!(
                    "{} {} {}",
                    str_at(request, &["tool_name"], ""),
                    str_at(request, &["tool_key"], ""),
                    str_at(request, &["selected_tool_key"], "")
                ));
                tool_name.contains(&expected_gate_3)
            })
            .unwrap_or_else(|| serialized.contains(&expected_gate_3));
    let gate_4 = required_gate_4_fields.iter().all(|field| {
        let field = normalize_for_compare(field);
        tool_request
            .and_then(|request| {
                request
                    .get("input")
                    .or_else(|| request.get("request_payload"))
                    .or_else(|| request.get("payload"))
            })
            .and_then(Value::as_object)
            .map(|input| input.keys().any(|key| normalize_for_compare(key) == field))
            .unwrap_or_else(|| serialized.contains(&format!("\"{field}\"")))
    });
    let gate_1 = has_pending_tool(payload)
        || has_tool_execution(payload)
        || gate_2
        || gate_3
        || gate_4
        || serialized.contains("tool_required")
        || serialized.contains("answered_yes")
        || serialized.contains("should_call_tools\":true");
    gates.insert("gate_1_tool_need".to_string(), gate_1);
    gates.insert("gate_2_tool_family".to_string(), gate_2);
    gates.insert("gate_3_tool_key".to_string(), gate_3);
    gates.insert("gate_4_request_template".to_string(), gate_4);
    gates
}

fn has_pending_tool(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| payload.pointer(pointer).and_then(Value::as_str) == Some("pending_confirmation"))
}

fn pending_tool_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn has_tool_execution(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn has_source_signal(payload: &Value, response_text: &str) -> bool {
    if has_tool_execution(payload) {
        return true;
    }
    let normalized = normalize_for_compare(response_text);
    [
        "source",
        "evidence",
        "according",
        "docs",
        "release",
        "changelog",
        "citation",
        "http://",
        "https://",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn has_limitation_signal(normalized: &str) -> bool {
    [
        "limited",
        "limitation",
        "uncertain",
        "caveat",
        "sparse",
        "unknown",
        "not enough",
        "not clear",
        "as of",
        "current",
        "verify",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn has_tradeoff_or_structure(normalized: &str) -> bool {
    [
        "tradeoff",
        "trade-off",
        "compare",
        "comparison",
        "criteria",
        "dimension",
        "versus",
        "vs",
        "strength",
        "weakness",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn has_recommendation_signal(normalized: &str) -> bool {
    [
        "recommend",
        "best for",
        "use ",
        "choose",
        "should",
        "default",
        "pragmatic",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn normal_prose_signal(response_text: &str) -> bool {
    let trimmed = response_text.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('[')
        && trimmed.split_whitespace().count() >= 8
}

fn entity_coverage(normalized_response: &str, required_entities: &[String]) -> f64 {
    if required_entities.is_empty() {
        return 1.0;
    }
    let covered = required_entities
        .iter()
        .filter(|entity| normalized_response.contains(&normalize_for_compare(entity)))
        .count() as u64;
    ratio(covered, required_entities.len() as u64)
}

fn raw_tool_payload_leak(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "pending_tool_request",
        "response_workflow",
        "request_payload",
        "tool_attempts",
        "tool_receipt",
        "receipt_binding",
        "selected_tool_family",
        "\"tool_name\"",
        "\"tool_key\"",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn internal_workflow_leak(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "gate_1",
        "gate_2",
        "gate_3",
        "gate_4",
        "workflow_trace",
        "workflow_state",
        "finalization_outcome",
        "visible_response_source",
        "llm_gate_instruction",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn tool_choice_as_final_response(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    normalized.starts_with("yes. tool")
        || normalized.starts_with("tool family")
        || normalized.starts_with("tool:")
        || normalized.contains("request payload:")
        || normalized.contains("selected tool:")
}

fn unsupported_claim_signal(case: &Value, response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    if normalized.is_empty() {
        return false;
    }
    let asks_best = normalize_for_compare(&str_at(case, &["prompt"], "")).contains("best");
    let has_universal_best = normalized.contains("the best")
        || normalized.contains("clear winner")
        || normalized.contains("always use");
    asks_best && has_universal_best && !has_limitation_signal(&normalized)
}
