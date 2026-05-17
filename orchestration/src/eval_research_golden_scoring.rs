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
    pub(super) excellent_diagnostics: Value,
    pub(super) coverage_entities: Vec<String>,
    pub(super) citation_behavior: Value,
    pub(super) query_satisfaction: Value,
}

pub(super) fn grade_case(
    case: &Value,
    payload: &Value,
    pass_score: u64,
    excellent_score: u64,
) -> CaseGrade {
    let response_text = assistant_text(payload);
    let normalized = normalize_for_compare(&response_text);
    let prompt = str_at(case, &["prompt"], "");
    let normalized_prompt = normalize_for_compare(&prompt);
    let required_entities = string_array_at(case, &["required_entities"]);
    let coverage_entities = user_stated_required_entities(&normalized_prompt, &required_entities);
    let gates = gate_results(case, payload);
    let raw_tool_leak = raw_tool_payload_leak(&response_text);
    let internal_leak = internal_workflow_leak(&response_text);
    let tool_choice_final_response = tool_choice_as_final_response(&response_text);
    let empty_response = response_text.trim().is_empty();
    let unsupported_claim = unsupported_claim_signal(case, &response_text);
    let retrieval_quality = retrieval_provider_quality(payload);
    let source_signal = has_source_signal(&response_text, &retrieval_quality);
    let citation_behavior = citation_behavior(payload, &response_text, &retrieval_quality);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let limitation_signal = has_limitation_signal(&normalized);
    let final_answer_present = !empty_response && response_text.split_whitespace().count() >= 20;
    let entity_coverage = entity_coverage(&normalized, &coverage_entities);
    let query_satisfaction = query_satisfaction(
        &normalized_prompt,
        &normalized,
        &coverage_entities,
        entity_coverage,
        final_answer_present,
        source_signal,
        citation_signal,
        limitation_signal,
    );
    let query_satisfaction_score = query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let workflow_score = gates.values().filter(|ok| **ok).count() as u64 * 5;
    let evidence_score = (if source_signal { 6 } else { 0 })
        + (if citation_signal { 6 } else { 0 })
        + (if !raw_tool_leak { 5 } else { 0 })
        + (if limitation_signal { 4 } else { 0 })
        + (if !unsupported_claim { 4 } else { 0 });
    let synthesis_score = (if final_answer_present { 6 } else { 0 })
        + ((entity_coverage * 7.0).round() as u64)
        + (if has_tradeoff_or_structure(&normalized) {
            6
        } else {
            0
        })
        + (if has_recommendation_signal(&normalized) {
            4
        } else {
            0
        })
        + (if limitation_signal { 2 } else { 0 })
        + query_satisfaction_score.min(10);
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
    if !coverage_entities.is_empty() && entity_coverage < 0.75 {
        failures.push(format!("entity_coverage_low:{entity_coverage:.2}"));
    }
    if query_satisfaction_score < 7 {
        failures.push(format!(
            "query_satisfaction_low:{query_satisfaction_score}<7"
        ));
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
    let excellent_diagnostics = excellent_diagnostics(ExcellentDiagnosticInput {
        retrieval_quality: &retrieval_quality,
        citation_behavior: &citation_behavior,
        query_satisfaction: &query_satisfaction,
        source_signal,
        final_answer_present,
        limitation_signal,
        raw_tool_leak,
        internal_leak,
        unsupported_claim,
        score,
        excellent_score,
        failures: &failures,
    });
    let excellent_blockers = string_array_at(&excellent_diagnostics, &["blockers"]);
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
        excellent_diagnostics,
        coverage_entities,
        citation_behavior,
        query_satisfaction,
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

fn citation_behavior(payload: &Value, response_text: &str, retrieval_quality: &Value) -> Value {
    let citation_count = response_citation_count(payload);
    let evidence_count = retrieval_quality
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| provider_evidence_count(payload));
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = response_has_inline_citation_signal(response_text);
    let citation_signal = citation_count > 0 || response_source_signal;
    let synthesis_ignored_citable_evidence =
        usable_evidence && evidence_count > 0 && !citation_signal;
    json!({
        "schema_version": 1,
        "citation_count": citation_count,
        "evidence_count": evidence_count,
        "usable_evidence": usable_evidence,
        "response_source_signal": response_source_signal,
        "citation_signal": citation_signal,
        "synthesis_ignored_citable_evidence": synthesis_ignored_citable_evidence,
        "note": "Measures whether the final artifact/prose exposes compact citation or source-reference signal separately from whether retrieval found evidence."
    })
}

fn response_citation_count(payload: &Value) -> u64 {
    [
        "/citations",
        "/sources",
        "/response_workflow/citations",
        "/response_workflow/sources",
        "/response_workflow/final_llm_response/citations",
        "/response_workflow/final_llm_response/sources",
        "/response_finalization/citations",
        "/response_finalization/sources",
        "/response_finalization/final_response/citations",
        "/response_finalization/final_response/sources",
        "/response_finalization/final_llm_response/citations",
        "/response_finalization/final_llm_response/sources",
    ]
    .iter()
    .map(|pointer| count_content_items(payload.pointer(pointer).unwrap_or(&Value::Null)))
    .sum::<u64>()
}

fn response_has_inline_citation_signal(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "http://",
        "https://",
        "source:",
        "sources:",
        "citation",
        "citations",
        "according to",
        "the docs",
        "official docs",
        "release notes",
        "changelog",
        "paper",
        "study",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn query_satisfaction(
    normalized_prompt: &str,
    normalized_response: &str,
    coverage_entities: &[String],
    entity_coverage: f64,
    final_answer_present: bool,
    source_signal: bool,
    citation_signal: bool,
    limitation_signal: bool,
) -> Value {
    let scope_covered = coverage_entities.is_empty() || entity_coverage >= 0.75;
    let intent_answered = response_matches_prompt_intent(normalized_prompt, normalized_response);
    let decision_value = has_recommendation_signal(normalized_response)
        || response_matches_decision_prompt(normalized_prompt, normalized_response);
    let right_granularity = response_has_right_granularity(normalized_response);
    let evidence_aware = source_signal || citation_signal || limitation_signal;
    let score = [
        (final_answer_present, 2_u64),
        (intent_answered, 2),
        (scope_covered, 2),
        (evidence_aware, 2),
        (decision_value, 1),
        (right_granularity, 1),
    ]
    .iter()
    .filter_map(|(ok, points)| ok.then_some(*points))
    .sum::<u64>();
    json!({
        "schema_version": 1,
        "score": score,
        "max_score": 10,
        "intent_answered": intent_answered,
        "scope_covered": scope_covered,
        "user_stated_coverage_entities": coverage_entities,
        "entity_coverage": entity_coverage,
        "evidence_aware": evidence_aware,
        "decision_value": decision_value,
        "right_granularity": right_granularity,
        "note": "Query satisfaction is derived from the original prompt plus available evidence behavior, not from hidden expected answers."
    })
}

fn response_matches_prompt_intent(normalized_prompt: &str, normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let asks_comparison = contains_any(
        normalized_prompt,
        &[
            "compare",
            "versus",
            " vs ",
            "tradeoff",
            "tradeoffs",
            "which",
        ],
    );
    if asks_comparison {
        return has_tradeoff_or_structure(normalized_response);
    }
    let asks_explanation = contains_any(
        normalized_prompt,
        &[
            "what",
            "why",
            "how",
            "explain",
            "research",
            "summarize",
            "find",
        ],
    );
    if asks_explanation {
        return has_tradeoff_or_structure(normalized_response)
            || normalized_response.contains("finding")
            || normalized_response.contains("evidence")
            || normalized_response.contains("because");
    }
    true
}

fn response_matches_decision_prompt(normalized_prompt: &str, normalized_response: &str) -> bool {
    let wants_decision = contains_any(
        normalized_prompt,
        &[
            "which",
            "best",
            "recommend",
            "tradeoff",
            "tradeoffs",
            "practical",
            "useful",
            "appropriate",
            "choose",
            "should",
        ],
    );
    !wants_decision || has_recommendation_signal(normalized_response)
}

fn response_has_right_granularity(normalized_response: &str) -> bool {
    let word_count = normalized_response.split_whitespace().count();
    (45..=900).contains(&word_count)
}

fn user_stated_required_entities(
    normalized_prompt: &str,
    required_entities: &[String],
) -> Vec<String> {
    required_entities
        .iter()
        .filter(|entity| normalized_response_covers_entity(normalized_prompt, entity))
        .cloned()
        .collect()
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
            "execution_error",
            "error",
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

struct ExcellentDiagnosticInput<'a> {
    retrieval_quality: &'a Value,
    citation_behavior: &'a Value,
    query_satisfaction: &'a Value,
    source_signal: bool,
    final_answer_present: bool,
    limitation_signal: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    unsupported_claim: bool,
    score: u64,
    excellent_score: u64,
    failures: &'a [String],
}

fn excellent_diagnostics(input: ExcellentDiagnosticInput<'_>) -> Value {
    let retrieval_status = str_at(input.retrieval_quality, &["status"], "unknown");
    let citable_evidence_available = input
        .retrieval_quality
        .get("allows_excellent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let citation_signal = input
        .citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let query_satisfaction_score = input
        .query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let decision_value = input
        .query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope_covered = input
        .query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let needs_gap_statement = !scope_covered
        || matches!(
            retrieval_status.as_str(),
            "low_signal"
                | "no_results"
                | "no_evidence"
                | "provider_degraded"
                | "raw_provider_absent"
                | "conflicting_provider_state"
        );
    let evidence_gaps_named_when_needed = !needs_gap_statement || input.limitation_signal;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "excellent_1_query_satisfaction".to_string(),
        json!(query_satisfaction_score >= 9),
    );
    subgates.insert(
        "excellent_2_citable_evidence_available".to_string(),
        json!(citable_evidence_available),
    );
    subgates.insert(
        "excellent_3_citations_used_in_final".to_string(),
        json!(!citable_evidence_available || citation_signal),
    );
    subgates.insert(
        "excellent_4_claims_trace_to_citations".to_string(),
        json!(
            !citable_evidence_available
                || (citation_signal && input.source_signal && !input.unsupported_claim)
        ),
    );
    subgates.insert(
        "excellent_5_evidence_gaps_named_when_needed".to_string(),
        json!(evidence_gaps_named_when_needed),
    );
    subgates.insert(
        "excellent_6_decision_value_present".to_string(),
        json!(decision_value),
    );
    subgates.insert(
        "excellent_7_projection_clean".to_string(),
        json!(input.final_answer_present && !input.raw_tool_leak && !input.internal_leak),
    );
    subgates.insert(
        "excellent_8_score_threshold".to_string(),
        json!(input.score >= input.excellent_score),
    );
    subgates.insert(
        "excellent_9_no_pass_failures".to_string(),
        json!(input.failures.is_empty()),
    );

    let ordered = [
        (
            "excellent_1_query_satisfaction",
            "query_satisfaction_below_excellent",
        ),
        (
            "excellent_2_citable_evidence_available",
            "retrieval_quality_not_excellent_ready",
        ),
        (
            "excellent_3_citations_used_in_final",
            "missing_final_citation_or_source_signal",
        ),
        (
            "excellent_4_claims_trace_to_citations",
            "claims_not_traceable_to_citation_signal",
        ),
        (
            "excellent_5_evidence_gaps_named_when_needed",
            "missing_evidence_gap_statement",
        ),
        (
            "excellent_6_decision_value_present",
            "missing_decision_value",
        ),
        ("excellent_7_projection_clean", "projection_not_clean"),
        ("excellent_8_score_threshold", "score_below_excellent"),
        ("excellent_9_no_pass_failures", "pass_failures_present"),
    ];
    let blockers = ordered
        .iter()
        .filter_map(|(gate, blocker)| {
            (!subgates
                .get(*gate)
                .and_then(Value::as_bool)
                .unwrap_or(false))
            .then(|| (*blocker).to_string())
        })
        .collect::<Vec<_>>();
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "retrieval_status": retrieval_status,
        "score": input.score,
        "excellent_score": input.excellent_score,
        "note": "Excellent is diagnosed through generic quality properties, not hidden expected facts or a required visible format."
    })
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
                semantic_child_values(map)
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
                + semantic_child_values(map)
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
        Value::String(raw) => u64::from(substantive_text(raw)),
        Value::Array(rows) => rows
            .iter()
            .filter(|row| value_has_substantive_content(row))
            .count() as u64,
        Value::Object(map) => u64::from(object_has_substantive_content(map)),
    }
}

fn value_has_substantive_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => substantive_text(raw),
        Value::Array(rows) => rows.iter().any(value_has_substantive_content),
        Value::Object(map) => object_has_substantive_content(map),
    }
}

fn object_has_substantive_content(map: &serde_json::Map<String, Value>) -> bool {
    if map.is_empty() || object_is_status_or_error_only(map) {
        return false;
    }
    let direct_semantic_keys = [
        "title",
        "url",
        "link",
        "locator",
        "source_url",
        "source_domain",
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "body",
        "description",
        "abstract",
        "claim_hints",
        "claims",
        "extracted_claims",
        "claim_candidates",
        "key_findings",
        "findings",
        "citations",
        "sources",
    ];
    if direct_semantic_keys.iter().any(|key| {
        map.get(*key)
            .map(value_has_substantive_content)
            .unwrap_or(false)
    }) {
        return true;
    }
    semantic_child_values(map).any(value_has_substantive_content)
}

fn object_is_status_or_error_only(map: &serde_json::Map<String, Value>) -> bool {
    let has_error_marker = ["error", "failure", "failure_reason", "status"]
        .iter()
        .any(|key| {
            map.get(*key)
                .map(value_has_substantive_content)
                .unwrap_or(false)
        });
    has_error_marker
        && map.iter().all(|(key, value)| {
            operational_or_error_key(key) || !value_has_substantive_content(value)
        })
}

fn semantic_child_values<'a>(
    map: &'a serde_json::Map<String, Value>,
) -> impl Iterator<Item = &'a Value> {
    map.iter()
        .filter(|(key, _)| !operational_or_error_key(key))
        .map(|(_, value)| value)
}

fn operational_or_error_key(key: &str) -> bool {
    let normalized = normalize_for_compare(&key.replace(['_', '-'], " "));
    [
        "status",
        "state",
        "error",
        "failure",
        "failure reason",
        "failure class",
        "provider",
        "tool",
        "name",
        "query",
        "queries",
        "input",
        "aperture",
        "request",
        "request payload",
        "metadata",
        "query metadata policy",
        "quality flags",
        "quality reasons",
        "blocker taxonomy",
    ]
    .iter()
    .any(|needle| normalized == *needle || normalized.ends_with(&format!(" {needle}")))
}

fn substantive_text(raw: &str) -> bool {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return false;
    }
    let normalized = normalize_for_compare(cleaned);
    ![
        "error",
        "failed",
        "tool execution failed",
        "no results",
        "no_results",
        "none",
        "null",
        "unknown",
    ]
    .iter()
    .any(|marker| normalized == *marker)
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
        "weak",
        "insufficient",
        "gap",
        "gaps",
        "missing",
        "unknown",
        "not enough",
        "not clear",
        "does not establish",
        "doesn't establish",
        "does not support",
        "doesn't support",
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
        "finding",
        "source-backed",
        "evidence supports",
        "evidence shows",
        "what the evidence",
        "risk",
        "concern",
        "boundary",
        "evaluation plan",
        "plan",
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
        "what you can do",
        "next step",
        "plan",
        "treat",
        "avoid",
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
        .filter(|entity| normalized_response_covers_entity(normalized_response, entity))
        .count() as u64;
    ratio(covered, required_entities.len() as u64)
}

fn normalized_response_covers_entity(normalized_response: &str, entity: &str) -> bool {
    let normalized_entity = normalize_for_compare(entity);
    if normalized_entity.is_empty() {
        return false;
    }
    if normalized_response.contains(&normalized_entity) {
        return true;
    }
    if normalized_response.contains(&simple_plural_variant(&normalized_entity))
        || normalized_response.contains(&simple_singular_variant(&normalized_entity))
    {
        return true;
    }
    let tokens = normalized_entity
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| token_or_simple_variant_present(normalized_response, token))
}

fn token_or_simple_variant_present(normalized_response: &str, token: &str) -> bool {
    normalized_response.contains(token)
        || normalized_response.contains(&simple_plural_variant(token))
        || normalized_response.contains(&simple_singular_variant(token))
}

fn simple_plural_variant(value: &str) -> String {
    if value.ends_with('s') {
        value.to_string()
    } else {
        format!("{value}s")
    }
}

fn simple_singular_variant(value: &str) -> String {
    value.strip_suffix('s').unwrap_or(value).to_string()
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
    fn error_only_provider_rows_do_not_count_as_retrieval_evidence() {
        let payload = json!({
            "tools": [{
                "name": "batch_query",
                "status": "error",
                "input": {
                    "query": "Research current RAG stack options for a small team",
                    "keywords": ["RAG", "LlamaIndex", "LangChain"]
                },
                "provider_results": [{
                    "provider": "web",
                    "query": "Research current RAG stack options for a small team",
                    "status": "error",
                    "error": "tool_execution_failed"
                }],
                "evidence_refs": [{
                    "provider": "web",
                    "query": "Research current RAG stack options for a small team",
                    "status": "error",
                    "error": "tool_execution_failed"
                }]
            }]
        });

        let quality = retrieval_provider_quality(&payload);
        assert_eq!(
            quality.get("candidate_count").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            quality.get("evidence_count").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            quality
                .get("content_rich_candidate_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("provider_degraded")
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

    #[test]
    fn scoring_shape_accepts_general_research_findings_and_plans() {
        let security = normalize_for_compare(
            "Here is what the evidence supports on AI browser agent security concerns. \
             Source-backed finding: prompt injection is a published risk, with gaps around credential handling.",
        );
        assert!(has_tradeoff_or_structure(&security));
        assert!(has_limitation_signal(&security));

        let sparse_benchmark = normalize_for_compare(
            "The benchmark evidence is weak and insufficient. \
             What the evidence shows is partial, so the practical evaluation plan should compare latency, cost, and reliability directly.",
        );
        assert!(has_tradeoff_or_structure(&sparse_benchmark));
        assert!(has_limitation_signal(&sparse_benchmark));
        assert!(has_recommendation_signal(&sparse_benchmark));
    }

    #[test]
    fn entity_coverage_accepts_phrase_variants_without_case_specific_aliases() {
        let response = normalize_for_compare(
            "The evidence discusses agent evaluation frameworks and framework results, \
             but no head-to-head benchmark data was found.",
        );
        assert!(normalized_response_covers_entity(
            &response,
            "agent framework"
        ));
        assert_eq!(
            entity_coverage(
                &response,
                &["benchmark".to_string(), "agent framework".to_string()]
            ),
            1.0
        );
    }

    #[test]
    fn hidden_fixture_entities_do_not_hard_fail_broad_discovery_prompts() {
        let case = json!({
            "prompt": "Research the strongest open-source coding agents right now and explain which are useful for real repositories versus demos.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["OpenHands", "Aider"]
        });
        let payload = json!({
            "response": "The source-backed finding is that repository usefulness depends less on demo polish and more on repeatability, reviewability, and how well the agent can work against an existing codebase. For real repositories, choose tools with explicit edit loops, test feedback, and clear rollback behavior; treat demo-first agents as exploratory unless their docs show durable project workflows. Caveat: current source coverage is uneven, so verify recent releases before committing.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "open-source coding agents real repositories demos",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 3,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Coding agent project workflow docs",
                    "locator": "https://example.test/coding-agent-docs",
                    "snippet": "This source contains enough detail about edit loops, repository workflows, tests, review, and rollback behavior to support a practical synthesis for repository use.",
                    "claim_hints": ["Repository usefulness depends on repeatable edit and test loops."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.coverage_entities.is_empty());
        assert!(!grade
            .failures
            .iter()
            .any(|failure| failure.starts_with("entity_coverage_low")));
        assert_eq!(
            grade
                .query_satisfaction
                .get("scope_covered")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(grade.pass, "{:?}", grade.failures);
    }

    #[test]
    fn user_stated_entities_remain_query_scope() {
        let case = json!({
            "prompt": "Compare OpenHands and Aider for existing repository maintenance.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["OpenHands", "Aider"]
        });
        let payload = json!({
            "response": "According to source evidence, OpenHands has useful repository-maintenance affordances, but the comparison is incomplete. I would verify release docs before choosing because source coverage is limited and the available evidence only supports a bounded recommendation.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "OpenHands Aider repository maintenance",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Repository maintenance source",
                    "locator": "https://example.test/repo-maintenance",
                    "snippet": "This source contains enough detail about repository maintenance workflows, review, test loops, and coding agent operational concerns to support synthesis.",
                    "claim_hints": ["Existing repository work requires reviewable edit loops."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(grade.coverage_entities, vec!["OpenHands", "Aider"]);
        assert!(grade
            .failures
            .iter()
            .any(|failure| failure.starts_with("entity_coverage_low")));
    }

    #[test]
    fn citation_behavior_separates_available_evidence_from_final_citation_signal() {
        let payload = json!({
            "response": "The answer gives a recommendation without naming supporting material.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Usable source",
                    "locator": "https://example.test/source",
                    "snippet": "This source has enough content to be usable evidence for a research answer and includes concrete findings that should be cited.",
                    "claim_hints": ["A concrete source-backed claim."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload);
        let behavior = citation_behavior(
            &payload,
            "The answer gives a recommendation without naming supporting material.",
            &retrieval_quality,
        );
        assert_eq!(
            behavior.get("usable_evidence").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            behavior.get("citation_signal").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            behavior
                .get("synthesis_ignored_citable_evidence")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn excellent_diagnostics_call_out_missing_final_citation_signal() {
        let case = json!({
            "prompt": "Compare Alpha and Beta for production use.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Alpha", "Beta"]
        });
        let payload = json!({
            "response": "Alpha is the better default for production when reliability matters, while Beta is more useful for exploratory workflows. Alpha has stronger deployment and maintenance tradeoffs; Beta remains useful when speed of experimentation matters. The practical recommendation is to use Alpha for steady production and Beta for prototypes.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "Alpha Beta production comparison",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Alpha and Beta production comparison",
                    "locator": "https://example.test/alpha-beta-production",
                    "snippet": "A substantive source comparing Alpha and Beta for reliability, deployment, maintenance, and experimentation tradeoffs.",
                    "claim_hints": ["Alpha is better suited to production reliability."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.pass, "{:?}", grade.failures);
        assert!(!grade.excellent);
        assert_eq!(
            grade
                .excellent_diagnostics
                .pointer("/subgates/excellent_3_citations_used_in_final")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            grade
                .excellent_diagnostics
                .get("top_blocker")
                .and_then(Value::as_str),
            Some("missing_final_citation_or_source_signal")
        );
    }

    #[test]
    fn excellent_diagnostics_accept_public_source_signal_without_format_lock() {
        let case = json!({
            "prompt": "Compare Alpha and Beta for production use.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Alpha", "Beta"]
        });
        let payload = json!({
            "response": "According to the project docs and release notes, Alpha is the better production default when reliability and maintenance matter, while Beta is stronger for exploratory workflows. Alpha's deployment story is steadier; Beta is useful for fast prototypes. The practical recommendation is Alpha for production and Beta for experimentation.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "Alpha Beta production comparison",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Alpha and Beta production comparison",
                    "locator": "https://example.test/alpha-beta-production",
                    "snippet": "A substantive source comparing Alpha and Beta for reliability, deployment, maintenance, and experimentation tradeoffs.",
                    "claim_hints": ["Alpha is better suited to production reliability."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.pass, "{:?}", grade.failures);
        assert_eq!(
            grade
                .excellent_diagnostics
                .pointer("/subgates/excellent_3_citations_used_in_final")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(!grade
            .excellent_blockers
            .contains(&"missing_final_citation_or_source_signal".to_string()));
    }
}
