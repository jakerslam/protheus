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
    pub(super) retrieval_quality: Value,
    pub(super) excellent_blockers: Vec<String>,
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
    let retrieval_quality = retrieval_provider_quality(payload);
    let source_signal = has_source_signal(&response_text, &retrieval_quality);
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
    let excellent_blockers = excellent_blockers(&retrieval_quality, source_signal);
    CaseGrade {
        score,
        pass: score >= pass_score && failures.is_empty(),
        excellent: score >= excellent_score && failures.is_empty() && excellent_blockers.is_empty(),
        gates,
        dimension_scores,
        failures,
        response_text,
        empty_response,
        raw_tool_leak,
        tool_choice_final_response,
        unsupported_claim,
        retrieval_quality,
        excellent_blockers,
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
    let synthesis_only_without_new_candidate =
        case_allows_existing_tool_state_without_new_candidate(case);
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
                gate_3_tool_matches(
                    &format!(
                        "{} {} {}",
                        str_at(request, &["tool_name"], ""),
                        str_at(request, &["tool_key"], ""),
                        str_at(request, &["selected_tool_key"], "")
                    ),
                    &expected_gate_3,
                )
            })
            .unwrap_or_else(|| gate_3_tool_matches(&serialized, &expected_gate_3))
        || (synthesis_only_without_new_candidate && gate_2);
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

fn case_allows_existing_tool_state_without_new_candidate(case: &Value) -> bool {
    let gate_1 = normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_1"], ""));
    let post_tool = normalize_for_compare(&str_at(case, &["expected_gate_path", "post_tool"], ""));
    gate_1.contains("pending_tool_result") || post_tool.starts_with("must_synthesize_from")
}

fn gate_3_tool_matches(actual_raw: &str, expected_raw: &str) -> bool {
    let actual = normalize_for_compare(actual_raw);
    let expected = normalize_for_compare(expected_raw);
    if expected.is_empty() {
        return true;
    }
    if actual.contains(&expected) {
        return true;
    }
    matches!(
        expected.as_str(),
        "web_search" | "batch_query" | "batch query"
    ) && (actual.contains("web_search")
        || actual.contains("batch_query")
        || actual.contains("batch query"))
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

fn has_source_signal(response_text: &str, retrieval_quality: &Value) -> bool {
    if retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
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

fn retrieval_provider_quality(payload: &Value) -> Value {
    let tool_executed = has_tool_execution(payload);
    let candidate_count = provider_candidate_count(payload);
    let evidence_count = provider_evidence_count(payload);
    let content_rich_candidate_count = provider_content_rich_candidate_count(payload);
    let claim_hint_count = provider_claim_hint_count(payload);
    let status_text = tool_status_marker_text(payload);
    let explicit_no_results = contains_any(
        &status_text,
        &[
            "no_results",
            "no results",
            "no usable result",
            "no usable results",
            "zero evidence",
            "zero snippets",
            "zero candidate snippets",
            "empty_feed",
        ],
    );
    let explicit_provider_degraded = contains_any(
        &status_text,
        &[
            "provider degradation",
            "provider degraded",
            "provider_error",
            "provider error",
            "transport_error",
            "timeout",
            "blocked",
            "anti_bot",
            "anti-bot",
            "proxy_error",
            "failed",
        ],
    );
    let explicit_low_signal = contains_any(
        &status_text,
        &[
            "low_signal",
            "low signal",
            "low-signal",
            "low relevance",
            "low-relevance",
            "weak evidence",
            "limited evidence",
            "limited source coverage",
            "retrieval gap",
            "retrieval miss",
            "irrelevant",
            "off target",
            "off-topic",
        ],
    );
    let evidence_artifact_conflict =
        explicit_no_results && (candidate_count > 0 || evidence_count > 0);
    let status = if !tool_executed {
        "not_attempted"
    } else if explicit_provider_degraded {
        "provider_degraded"
    } else if evidence_artifact_conflict {
        "conflicting_provider_state"
    } else if explicit_no_results {
        "no_results"
    } else if evidence_count == 0 {
        "no_evidence"
    } else if candidate_count == 0 {
        "raw_provider_absent"
    } else if explicit_low_signal {
        "low_signal"
    } else {
        "usable"
    };
    let usable_evidence = status == "usable";
    let allows_excellent =
        usable_evidence && content_rich_candidate_count > 0 && claim_hint_count > 0;
    let mut flags = Vec::new();
    if !tool_executed {
        flags.push("tool_not_executed");
    }
    if explicit_no_results {
        flags.push("explicit_no_results_marker");
    }
    if explicit_provider_degraded {
        flags.push("explicit_provider_degraded_marker");
    }
    if explicit_low_signal {
        flags.push("explicit_low_signal_marker");
    }
    if evidence_artifact_conflict {
        flags.push("evidence_artifact_conflict");
    }
    if evidence_count == 0 {
        flags.push("no_evidence_refs");
    }
    if candidate_count == 0 {
        flags.push("raw_provider_absent");
    }
    if tool_executed && evidence_count > 0 && content_rich_candidate_count == 0 {
        flags.push("content_rich_candidates_absent");
    }
    if tool_executed && evidence_count > 0 && claim_hint_count == 0 {
        flags.push("claim_hints_absent");
    }
    flags.sort_unstable();
    flags.dedup();
    json!({
        "status": status,
        "tool_executed": tool_executed,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "content_rich_candidate_count": content_rich_candidate_count,
        "claim_hint_count": claim_hint_count,
        "materialized_evidence_available": content_rich_candidate_count > 0 && claim_hint_count > 0,
        "usable_evidence": usable_evidence,
        "allows_excellent": allows_excellent,
        "quality_flags": flags,
        "classification_inputs": {
            "explicit_no_results_marker": explicit_no_results,
            "explicit_provider_degraded_marker": explicit_provider_degraded,
            "explicit_low_signal_marker": explicit_low_signal,
            "evidence_artifact_conflict": evidence_artifact_conflict,
            "content_rich_candidate_count": content_rich_candidate_count,
            "claim_hint_count": claim_hint_count,
            "status_marker_source": "structured_tool_status_fields_only"
        },
        "note": "Excellent requires usable retrieval/provider evidence; low-evidence fallbacks may pass but cannot earn excellent."
    })
}

fn excellent_blockers(retrieval_quality: &Value, source_signal: bool) -> Vec<String> {
    let mut blockers = Vec::new();
    if !source_signal {
        blockers.push("missing_source_signal".to_string());
    }
    if !retrieval_quality
        .get("allows_excellent")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let status = str_at(retrieval_quality, &["status"], "unknown");
        blockers.push(format!("retrieval_quality:{status}"));
    }
    blockers
}

fn provider_candidate_count(payload: &Value) -> u64 {
    tool_rows(payload)
        .iter()
        .map(|row| {
            let explicit = [
                "provider_raw_count",
                "provider_filtered_count",
                "candidate_count",
                "raw_count",
                "evidence_pack_candidate_count",
                "materialized_candidate_count",
            ]
            .iter()
            .filter_map(|key| row.get(*key).and_then(Value::as_u64))
            .max()
            .unwrap_or(0);
            let inferred = [
                "raw",
                "raw_result",
                "raw_results",
                "provider_result",
                "provider_results",
                "search_results",
                "organic_results",
                "web_results",
                "evidence_pack",
                "evidence_pack_candidates",
            ]
            .iter()
            .map(|key| count_content_items(row.get(*key).unwrap_or(&Value::Null)))
            .sum::<u64>();
            explicit.max(inferred)
        })
        .sum()
}

fn provider_evidence_count(payload: &Value) -> u64 {
    let top_level = [
        "/evidence",
        "/evidence_refs",
        "/evidence_pack",
        "/evidence_pack_candidates",
        "/sources",
        "/citations",
        "/response_workflow/evidence",
        "/response_workflow/evidence_refs",
        "/response_workflow/evidence_pack",
        "/response_workflow/evidence_pack_candidates",
        "/response_workflow/sources",
        "/response_workflow/citations",
        "/response_finalization/evidence",
        "/response_finalization/evidence_refs",
        "/response_finalization/evidence_pack",
        "/response_finalization/evidence_pack_candidates",
        "/response_finalization/tool_completion/evidence_refs",
        "/response_finalization/tool_completion/evidence_pack",
        "/response_finalization/tool_completion/evidence_pack_candidates",
        "/response_finalization/tool_completion/findings",
    ]
    .iter()
    .map(|pointer| count_content_items(payload.pointer(pointer).unwrap_or(&Value::Null)))
    .sum::<u64>();
    top_level
        + tool_rows(payload)
            .iter()
            .map(|row| {
                [
                    "evidence",
                    "evidence_refs",
                    "evidence_pack",
                    "evidence_pack_candidates",
                    "sources",
                    "citations",
                    "findings",
                ]
                .iter()
                .map(|key| count_content_items(row.get(*key).unwrap_or(&Value::Null)))
                .sum::<u64>()
            })
            .sum::<u64>()
}

fn provider_content_rich_candidate_count(payload: &Value) -> u64 {
    let explicit = provider_explicit_quality_metric(
        payload,
        &[
            "content_rich_candidate_count",
            "content_rich_item_count",
            "materialized_candidate_count",
        ],
    );
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_content_rich_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn provider_claim_hint_count(payload: &Value) -> u64 {
    let explicit = provider_explicit_quality_metric(
        payload,
        &[
            "claim_hint_count",
            "claim_hints_count",
            "claim_extraction_count",
            "extracted_claim_count",
        ],
    );
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_claim_hint_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn selected_tool_contexts(payload: &Value) -> Vec<&Value> {
    let mut rows = tool_rows(payload);
    for pointer in [
        "/tool_result_quality",
        "/evidence_pack_quality",
        "/evidence_pack",
        "/evidence_pack_candidates",
        "/evidence_refs",
        "/response_workflow/evidence_pack",
        "/response_workflow/evidence_pack_candidates",
        "/response_finalization/tool_completion/evidence_pack",
        "/response_finalization/tool_completion/evidence_pack_candidates",
    ] {
        if let Some(value) = payload.pointer(pointer) {
            rows.push(value);
        }
    }
    rows
}

fn provider_explicit_quality_metric(payload: &Value, metric_keys: &[&str]) -> u64 {
    selected_tool_contexts(payload)
        .iter()
        .map(|row| explicit_quality_metric(row, metric_keys, 0))
        .max()
        .unwrap_or(0)
}

fn explicit_quality_metric(value: &Value, metric_keys: &[&str], depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Object(map) => {
            let direct = metric_keys
                .iter()
                .filter_map(|key| map.get(*key).and_then(Value::as_u64))
                .max()
                .unwrap_or(0);
            direct.max(
                map.values()
                    .map(|row| explicit_quality_metric(row, metric_keys, depth + 1))
                    .max()
                    .unwrap_or(0),
            )
        }
        Value::Array(rows) => rows
            .iter()
            .map(|row| explicit_quality_metric(row, metric_keys, depth + 1))
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}

fn count_content_rich_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::String(raw) => u64::from(content_rich_text(raw)),
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_content_rich_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            let direct = [
                "snippet",
                "summary",
                "content",
                "markdown",
                "text",
                "body",
                "description",
                "abstract",
                "content_preview",
                "snippet_preview",
                "result",
            ]
            .iter()
            .any(|key| {
                map.get(*key)
                    .and_then(Value::as_str)
                    .map(content_rich_text)
                    .unwrap_or(false)
            });
            if direct {
                1
            } else {
                map.values()
                    .map(|row| count_content_rich_items(row, depth + 1))
                    .sum()
            }
        }
        _ => 0,
    }
}

fn count_claim_hint_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_claim_hint_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            let direct = [
                "claim_hints",
                "claims",
                "extracted_claims",
                "claim_candidates",
                "key_findings",
            ]
            .iter()
            .map(|key| count_content_items(map.get(*key).unwrap_or(&Value::Null)))
            .sum::<u64>();
            direct
                + map
                    .values()
                    .map(|row| count_claim_hint_items(row, depth + 1))
                    .sum::<u64>()
        }
        _ => 0,
    }
}

fn content_rich_text(raw: &str) -> bool {
    let cleaned = clean_text(raw, 1_800);
    if cleaned.split_whitespace().count() < 22 {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    ![
        "no results",
        "no usable result",
        "no usable results",
        "low signal",
        "low-signal",
        "retrieval-quality miss",
        "please narrow",
        "retry with",
        "verify you are human",
        "captcha",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn tool_rows(payload: &Value) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(items) = payload.get("tools").and_then(Value::as_array) {
        rows.extend(items.iter());
    }
    if let Some(items) = payload
        .pointer("/response_finalization/tool_completion/tool_attempts")
        .and_then(Value::as_array)
    {
        rows.extend(items.iter());
    }
    rows
}

fn count_content_items(value: &Value) -> u64 {
    match value {
        Value::Null => 0,
        Value::Bool(raw) => u64::from(*raw),
        Value::Number(_) => 1,
        Value::String(raw) => u64::from(!raw.trim().is_empty()),
        Value::Array(rows) => rows.iter().filter(|row| value_has_content(row)).count() as u64,
        Value::Object(map) => u64::from(!map.is_empty()),
    }
}

fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => rows.iter().any(value_has_content),
        Value::Object(map) => map.values().any(value_has_content),
    }
}

fn tool_status_marker_text(payload: &Value) -> String {
    tool_rows(payload)
        .iter()
        .flat_map(|row| {
            [
                str_at(row, &["name"], ""),
                str_at(row, &["status"], ""),
                str_at(row, &["completion_state"], ""),
                str_at(row, &["state"], ""),
                str_at(row, &["outcome"], ""),
                str_at(row, &["error"], ""),
                str_at(row, &["failure"], ""),
                str_at(row, &["failure_class"], ""),
                str_at(row, &["failure_reason"], ""),
                str_at(row, &["status_code"], ""),
                str_at(row, &["http_status"], ""),
                row.get("quality_lanes")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                row.get("quality_reasons")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                row.get("quality_flags")
                    .map(Value::to_string)
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(*needle))
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
        "web_gate_",
        "web_tooling_gates",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialized_evidence_candidates_count_as_retrieval_quality() {
        let payload = json!({
            "tools": [{
                "name": "browser_materialize_page",
                "status": "ok",
                "evidence_pack_candidates": [{
                    "source_kind": "browser_materialized_page",
                    "title": "Rendered research page",
                    "locator": "https://example.test/rendered",
                    "snippet": "This rendered page includes enough extracted body text to support a normal source-backed synthesis after materialization packaging succeeds, including context, terms, source scope, and a concrete claim for the user question.",
                    "claim_hints": ["Rendered source supports a concrete research claim."],
                    "score": 76.0,
                    "confidence": "usable"
                }]
            }]
        });

        let quality = retrieval_provider_quality(&payload);
        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            quality
                .get("materialized_evidence_available")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            quality.get("allows_excellent").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn web_tooling_gate_names_are_internal_leaks() {
        assert!(internal_workflow_leak(
            "web_gate_5_extraction_quality failed, so the final answer cannot use this source."
        ));
        assert!(internal_workflow_leak(
            "The web_tooling_gates summary says two gates passed."
        ));
    }
}
