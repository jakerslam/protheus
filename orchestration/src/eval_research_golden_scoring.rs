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
    pub(super) response_grading_layers: Value,
    pub(super) soft_quality_smoke: Value,
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
    let retrieval_quality = retrieval_provider_quality(payload, &normalized_prompt);
    let source_signal = has_source_signal(&response_text, &retrieval_quality);
    let citation_behavior = citation_behavior(payload, &response_text, &retrieval_quality);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = citation_behavior
        .get("response_source_signal")
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
        response_source_signal,
        citation_signal,
        limitation_signal,
    );
    let query_satisfaction_score = query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let source_summary_without_answer = source_summary_without_answer_signal(&normalized);
    let generic_response_contract = generic_response_contract(
        &response_text,
        final_answer_present,
        &query_satisfaction,
        source_summary_without_answer,
        raw_tool_leak,
        internal_leak,
        tool_choice_final_response,
    );
    let evidence_use_contract = tool_backed_evidence_contract(
        &normalized,
        &retrieval_quality,
        &citation_behavior,
        limitation_signal,
        &query_satisfaction,
        unsupported_claim,
    );
    let workflow_specific_rubric = research_workflow_specific_rubric(
        &query_satisfaction,
        source_signal,
        limitation_signal,
        &normalized,
    );
    let response_grading_layers = json!({
        "schema_version": 1,
        "generic_response_contract": generic_response_contract,
        "tool_backed_evidence_contract": evidence_use_contract,
        "workflow_specific_rubric": workflow_specific_rubric,
        "note": "Separates general answer quality, evidence-use discipline, and research-specific rubric checks so format flexibility and workflow-specific semantics can evolve independently."
    });
    let soft_quality_smoke = soft_quality_smoke_check(
        &response_text,
        &normalized,
        final_answer_present,
        &query_satisfaction,
        source_summary_without_answer,
        raw_tool_leak,
        internal_leak,
        tool_choice_final_response,
    );

    let workflow_score = gates.values().filter(|ok| **ok).count() as u64 * 5;
    let evidence_score = (if source_signal { 6 } else { 0 })
        + (if citation_signal { 6 } else { 0 })
        + (if !raw_tool_leak { 5 } else { 0 })
        + (if limitation_signal { 4 } else { 0 })
        + (if !unsupported_claim { 4 } else { 0 });
    let synthesis_score_raw = (if final_answer_present { 6 } else { 0 })
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
    let synthesis_score =
        synthesis_score_raw.saturating_sub(if source_summary_without_answer { 8 } else { 0 });
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
    if source_summary_without_answer {
        failures.push("source_summary_without_user_answer".to_string());
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
        normalized_response: &normalized,
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
        response_grading_layers,
        soft_quality_smoke,
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
        "evidence_outcome_posture": payload
            .pointer("/response_workflow/final_llm_response/evidence_outcome_posture")
            .or_else(|| payload.pointer("/response_finalization/final_llm_response/evidence_outcome_posture"))
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
        "/source_refs",
        "/response_workflow/citations",
        "/response_workflow/sources",
        "/response_workflow/source_refs",
        "/response_workflow/final_llm_response/citations",
        "/response_workflow/final_llm_response/sources",
        "/response_workflow/final_llm_response/source_refs",
        "/response_finalization/citations",
        "/response_finalization/sources",
        "/response_finalization/source_refs",
        "/response_finalization/final_response/citations",
        "/response_finalization/final_response/sources",
        "/response_finalization/final_response/source_refs",
        "/response_finalization/final_llm_response/citations",
        "/response_finalization/final_llm_response/sources",
        "/response_finalization/final_llm_response/source_refs",
        "/response_finalization/tool_completion/citations",
        "/response_finalization/tool_completion/source_refs",
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
        || text_contains_domain_like_source_marker(&normalized)
}

fn text_contains_domain_like_source_marker(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let cleaned = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '/' && ch != ':' && ch != '-'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = cleaned
            .split('/')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
            .collect::<String>();
        let labels = host
            .split('.')
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        if labels.len() < 2 {
            return false;
        }
        let tld = labels.last().copied().unwrap_or("");
        if !(2..=24).contains(&tld.len()) || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return false;
        }
        labels
            .iter()
            .any(|label| label.chars().any(|ch| ch.is_ascii_alphabetic()))
    })
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
    let coverage_gap_prevents_answer =
        response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response);
    let intent_answered = response_matches_prompt_intent(normalized_prompt, normalized_response)
        && !coverage_gap_prevents_answer;
    let decision_value = (has_recommendation_signal(normalized_response)
        || response_matches_decision_prompt(normalized_prompt, normalized_response))
        && !coverage_gap_prevents_answer;
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
        "coverage_gap_prevents_answer": coverage_gap_prevents_answer,
        "coverage_entity_aliases": coverage_entity_aliases(coverage_entities),
        "note": "Query satisfaction is derived from the original prompt plus available evidence behavior, not from hidden expected answers."
    })
}

fn generic_response_contract(
    response_text: &str,
    final_answer_present: bool,
    query_satisfaction: &Value,
    source_summary_without_answer: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    tool_choice_final_response: bool,
) -> Value {
    let intent_answered = query_satisfaction
        .get("intent_answered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let clean_projection = !raw_tool_leak && !internal_leak && !tool_choice_final_response;
    let human_readable = normal_prose_signal(response_text);
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "generic_1_final_answer_present".to_string(),
        json!(final_answer_present),
    );
    subgates.insert(
        "generic_2_answers_user_goal".to_string(),
        json!(intent_answered),
    );
    subgates.insert(
        "generic_3_no_source_summary_without_answer".to_string(),
        json!(!source_summary_without_answer),
    );
    subgates.insert(
        "generic_4_projection_clean".to_string(),
        json!(clean_projection),
    );
    subgates.insert(
        "generic_5_human_readable_shape".to_string(),
        json!(human_readable),
    );
    let ordered = [
        ("generic_1_final_answer_present", "missing_final_answer"),
        ("generic_2_answers_user_goal", "user_goal_not_answered"),
        (
            "generic_3_no_source_summary_without_answer",
            "source_summary_without_user_answer",
        ),
        (
            "generic_4_projection_clean",
            "projection_contains_internal_or_tool_state",
        ),
        (
            "generic_5_human_readable_shape",
            "response_shape_not_human_readable",
        ),
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
    let score = [
        final_answer_present,
        intent_answered,
        !source_summary_without_answer,
        clean_projection,
        human_readable,
    ]
    .iter()
    .filter(|ok| **ok)
    .count() as u64
        * 4;
    json!({
        "schema_version": 1,
        "layer_id": "generic_response_contract_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 20,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Generic response grading checks that the answer is actually user-facing, goal-directed, and readable without depending on a fixed visible format."
    })
}

fn tool_backed_evidence_contract(
    normalized_response: &str,
    retrieval_quality: &Value,
    citation_behavior: &Value,
    limitation_signal: bool,
    query_satisfaction: &Value,
    unsupported_claim: bool,
) -> Value {
    let tool_executed = retrieval_quality
        .get("tool_executed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let retrieval_status = str_at(retrieval_quality, &["status"], "unknown");
    let evidence_count = citation_behavior
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = citation_behavior
        .get("response_source_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let synthesis_ignored_citable_evidence = citation_behavior
        .get("synthesis_ignored_citable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope_covered = query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let needs_gap_statement = !scope_covered
        || matches!(
            retrieval_status.as_str(),
            "low_signal"
                | "no_results"
                | "no_evidence"
                | "provider_degraded"
                | "raw_provider_absent"
                | "conflicting_provider_state"
                | "low_relevance"
        );
    let denies_recorded_evidence =
        response_denies_recorded_evidence(normalized_response, evidence_count);
    let uses_recorded_evidence_when_present =
        !tool_executed || evidence_count == 0 || response_source_signal || citation_signal;
    let preserves_source_signal_when_citable =
        !usable_evidence || evidence_count == 0 || citation_signal;
    let names_limits_when_needed = !needs_gap_statement || limitation_signal;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "evidence_1_uses_recorded_evidence_when_present".to_string(),
        json!(uses_recorded_evidence_when_present),
    );
    subgates.insert(
        "evidence_2_preserves_compact_source_signal_when_citable".to_string(),
        json!(preserves_source_signal_when_citable),
    );
    subgates.insert(
        "evidence_3_does_not_ignore_citable_evidence".to_string(),
        json!(!synthesis_ignored_citable_evidence),
    );
    subgates.insert(
        "evidence_4_does_not_overclaim_or_deny_recorded_state".to_string(),
        json!(!unsupported_claim && !denies_recorded_evidence),
    );
    subgates.insert(
        "evidence_5_names_limits_when_needed".to_string(),
        json!(names_limits_when_needed),
    );
    let ordered = [
        (
            "evidence_1_uses_recorded_evidence_when_present",
            "recorded_evidence_not_used",
        ),
        (
            "evidence_2_preserves_compact_source_signal_when_citable",
            "missing_compact_source_signal",
        ),
        (
            "evidence_3_does_not_ignore_citable_evidence",
            "citable_evidence_ignored",
        ),
        (
            "evidence_4_does_not_overclaim_or_deny_recorded_state",
            "recorded_state_overclaimed_or_denied",
        ),
        (
            "evidence_5_names_limits_when_needed",
            "missing_evidence_gap_statement",
        ),
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
    let score = [
        uses_recorded_evidence_when_present,
        preserves_source_signal_when_citable,
        !synthesis_ignored_citable_evidence,
        !unsupported_claim && !denies_recorded_evidence,
        names_limits_when_needed,
    ]
    .iter()
    .filter(|ok| **ok)
    .count() as u64
        * 5;
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "layer_id": "tool_backed_evidence_contract_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 25,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "retrieval_status": retrieval_status,
        "note": "Evidence-use grading is format-flexible but requires the final answer to use recorded evidence honestly when evidence exists."
    })
}

fn research_workflow_specific_rubric(
    query_satisfaction: &Value,
    source_signal: bool,
    limitation_signal: bool,
    normalized_response: &str,
) -> Value {
    let query_satisfaction_score = query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let scope_covered = query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let decision_value = query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let right_granularity = query_satisfaction
        .get("right_granularity")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let research_structure =
        has_tradeoff_or_structure(normalized_response) || source_signal || limitation_signal;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "rubric_1_query_satisfaction".to_string(),
        json!(query_satisfaction_score >= 7),
    );
    subgates.insert("rubric_2_scope_covered".to_string(), json!(scope_covered));
    subgates.insert(
        "rubric_3_decision_or_explanatory_value".to_string(),
        json!(decision_value || has_tradeoff_or_structure(normalized_response)),
    );
    subgates.insert(
        "rubric_4_right_granularity".to_string(),
        json!(right_granularity),
    );
    subgates.insert(
        "rubric_5_research_structure_or_grounding".to_string(),
        json!(research_structure),
    );
    let ordered = [
        (
            "rubric_1_query_satisfaction",
            "query_satisfaction_below_rubric",
        ),
        ("rubric_2_scope_covered", "requested_scope_not_covered"),
        (
            "rubric_3_decision_or_explanatory_value",
            "missing_decision_or_explanatory_value",
        ),
        ("rubric_4_right_granularity", "response_granularity_off"),
        (
            "rubric_5_research_structure_or_grounding",
            "missing_research_structure_or_grounding",
        ),
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
    let score = (query_satisfaction_score.min(10) * 2)
        + (if scope_covered { 5 } else { 0 })
        + (if decision_value || has_tradeoff_or_structure(normalized_response) {
            4
        } else {
            0
        })
        + (if right_granularity { 3 } else { 0 })
        + (if research_structure { 3 } else { 0 });
    let normalized_score = score.min(35);
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "layer_id": "research_workflow_specific_rubric_v1",
        "pass": blockers.is_empty(),
        "score": normalized_score,
        "max_score": 35,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "note": "This layer is intentionally workflow-specific. It captures research-answer usefulness without requiring any fixed visible format."
    })
}

fn soft_quality_smoke_check(
    response_text: &str,
    normalized_response: &str,
    final_answer_present: bool,
    query_satisfaction: &Value,
    source_summary_without_answer: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    tool_choice_final_response: bool,
) -> Value {
    let intent_answered = query_satisfaction
        .get("intent_answered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_value = query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_user_help = final_answer_present && intent_answered;
    let meta_process_talk = response_has_meta_process_talk(normalized_response)
        && (!direct_user_help || source_summary_without_answer);
    let delegates_research_back_to_user =
        response_delegates_research_back_to_user(normalized_response) && !intent_answered;
    let obviously_bad_shape = raw_tool_leak
        || internal_leak
        || tool_choice_final_response
        || source_summary_without_answer
        || !normal_prose_signal(response_text);

    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "smoke_1_no_meta_process_talk".to_string(),
        json!(!meta_process_talk),
    );
    subgates.insert(
        "smoke_2_not_source_dump_without_answer".to_string(),
        json!(!source_summary_without_answer),
    );
    subgates.insert(
        "smoke_3_not_delegating_research_back_to_user".to_string(),
        json!(!delegates_research_back_to_user),
    );
    subgates.insert(
        "smoke_4_direct_user_help_present".to_string(),
        json!(direct_user_help),
    );
    subgates.insert(
        "smoke_5_projection_not_obviously_bad".to_string(),
        json!(!obviously_bad_shape),
    );
    subgates.insert(
        "smoke_6_decision_or_explanatory_value_present".to_string(),
        json!(decision_value || has_tradeoff_or_structure(normalized_response)),
    );
    let ordered = [
        ("smoke_1_no_meta_process_talk", "meta_process_talk_visible"),
        (
            "smoke_2_not_source_dump_without_answer",
            "source_dump_without_answer",
        ),
        (
            "smoke_3_not_delegating_research_back_to_user",
            "delegates_research_back_to_user",
        ),
        ("smoke_4_direct_user_help_present", "direct_answer_missing"),
        (
            "smoke_5_projection_not_obviously_bad",
            "projection_shape_obviously_bad",
        ),
        (
            "smoke_6_decision_or_explanatory_value_present",
            "decision_or_explanatory_value_missing",
        ),
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
    let score = subgates
        .values()
        .filter(|value| value.as_bool().unwrap_or(false))
        .count() as u64;
    json!({
        "schema_version": 1,
        "lane_id": "soft_quality_smoke_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 6,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "This is a soft UX smoke lane, not an authoritative grading contract. It flags answers that would likely feel obviously bad to a real user even if structural gates passed."
    })
}

fn response_has_meta_process_talk(normalized_response: &str) -> bool {
    contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "the current turn does not yet support",
            "the current turn",
            "recorded state",
            "research workflow",
            "structured workflow",
            "prompt chain",
            "i m operating within",
            "i am operating within",
            "tools actually executed",
            "no tools actually executed",
            "tool trace complete",
        ],
    )
}

fn response_delegates_research_back_to_user(normalized_response: &str) -> bool {
    contains_any(
        normalized_response,
        &[
            "try a narrower query",
            "retry with a narrower query",
            "check directly",
            "provide sources directly",
            "you can attempt the search again",
            "you could try again",
            "narrow the query",
        ],
    )
}

fn response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let explicit_goal_gap = contains_any(
        normalized_response,
        &[
            "i dont have usable source backed evidence",
            "i do not have usable source backed evidence",
            "i dont have usable evidence about",
            "i do not have usable evidence about",
            "i cant provide a source backed",
            "i cannot provide a source backed",
            "no source backed basis to compare",
            "no source backed basis to choose",
            "none of the required facets",
            "everything specific to your research goal",
            "search missed the entity entirely",
        ],
    );
    let off_topic_or_missing_coverage = contains_any(
        normalized_response,
        &[
            "largely off topic snippets",
            "largely off-topic snippets",
            "do not cover the actual",
            "does not cover the actual",
            "doesnt cover the actual",
            "what the evidence covers none",
            "what the evidence misses everything specific",
        ],
    );
    explicit_goal_gap
        || (off_topic_or_missing_coverage
            && response_delegates_research_back_to_user(normalized_response))
}

fn response_denies_recorded_evidence(normalized_response: &str, evidence_count: u64) -> bool {
    if evidence_count == 0 {
        return false;
    }
    let qualified_relevance_denial = contains_any(
        normalized_response,
        &[
            "no relevant evidence",
            "no relevant source",
            "does not cover",
            "doesn't cover",
            "not cover",
            "false positive",
            "off topic",
            "off-topic",
            "not relevant",
            "not about",
            "does not establish",
            "doesn't establish",
            "no source-backed basis to",
            "no source backed basis to",
        ],
    );
    if qualified_relevance_denial {
        return false;
    }
    let denies_source_backed = contains_any(
        normalized_response,
        &[
            "no source backed findings are available",
            "no source-backed findings are available",
            "no source backed synthesis is available",
            "no source-backed synthesis is available",
            "no source backed evidence is available",
            "no source-backed evidence is available",
        ],
    );
    denies_source_backed
        || contains_any(
            normalized_response,
            &[
                "no evidence was found",
                "no evidence is available",
                "no tool result is available",
            ],
        )
}

fn source_summary_without_answer_signal(normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let generic_bounded_template = normalized_response.contains("the safest bounded answer")
        && normalized_response.contains("recorded evidence so far");
    let raw_retrieval_summary = normalized_response.contains("recorded evidence so far")
        && normalized_response.contains("from web retrieval")
        && (normalized_response.contains("here s what i found")
            || normalized_response.contains("heres what i found"));
    let unanswered_retry_template = normalized_response
        .contains("current turn does not yet support a complete answer")
        && (normalized_response.contains("current tradeoff is breadth versus confidence")
            || normalized_response.contains("treat this as a partial answer"));
    let retrieval_status_dump = contains_any(
        normalized_response,
        &[
            "this retrieval attempt did not produce enough",
            "retrieval attempt did not produce enough",
            "web retrieval ran, but only",
            "only low signal snippets were available",
            "only low-signal snippets were available",
        ],
    ) && contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "here s what i found",
            "heres what i found",
            "retry with a narrower query",
            "narrower query",
        ],
    );
    let broken_prompt_echo = normalized_response.contains("complete answer to ?");
    generic_bounded_template
        || raw_retrieval_summary
        || unanswered_retry_template
        || retrieval_status_dump
        || broken_prompt_echo
}

fn excellent_insufficiency_marker_count(normalized_response: &str) -> usize {
    [
        "very limited evidence",
        "limited evidence",
        "insufficient evidence",
        "evidence is insufficient",
        "low confidence snippets",
        "low-confidence snippets",
        "off topic snippets",
        "off-topic snippets",
        "missing entity",
        "missing entities",
        "no source backed",
        "no source-backed",
        "no returned tool result",
        "comparison evidence is insufficient",
        "cannot answer from current evidence",
        "cannot provide a source backed",
        "cannot provide a source-backed",
        "do not have usable source backed evidence",
        "do not have usable source-backed evidence",
        "search missed the entity entirely",
    ]
    .iter()
    .filter(|needle| normalized_response.contains(**needle))
    .count()
}

fn opening_limitation_preface_for_excellent(normalized_response: &str) -> bool {
    let opening = normalized_response
        .split_whitespace()
        .take(60)
        .collect::<Vec<_>>()
        .join(" ");
    if opening.is_empty() {
        return false;
    }
    excellent_insufficiency_marker_count(&opening) >= 1
        || response_has_meta_process_talk(&opening)
        || contains_any(
            &opening,
            &[
                "what the recorded evidence actually shows",
                "what we know",
                "what we do not know",
                "recorded evidence so far",
                "the current turn does not yet support",
            ],
        )
}

fn limitation_heavy_for_excellent(normalized_response: &str) -> bool {
    let insufficiency_marker_count = excellent_insufficiency_marker_count(normalized_response);
    let limitation_preface = opening_limitation_preface_for_excellent(normalized_response);
    let recommendation_signal = has_recommendation_signal(normalized_response);
    let structure_signal = has_tradeoff_or_structure(normalized_response);
    let explicit_goal_gap =
        response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response);
    let source_summary_without_answer = source_summary_without_answer_signal(normalized_response);
    explicit_goal_gap
        || source_summary_without_answer
        || (limitation_preface && insufficiency_marker_count >= 2)
        || (limitation_preface && !recommendation_signal && !structure_signal)
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
    (20..=900).contains(&word_count)
}

fn user_stated_required_entities(
    normalized_prompt: &str,
    required_entities: &[String],
) -> Vec<String> {
    required_entities
        .iter()
        .filter(|entity| required_entity_needs_entity_coverage(entity))
        .filter(|entity| normalized_response_covers_entity(normalized_prompt, entity))
        .cloned()
        .collect()
}

fn required_entity_needs_entity_coverage(entity: &str) -> bool {
    let trimmed = entity.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed
        .chars()
        .any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return true;
    }

    if trimmed.contains(['-', '_', '/', '.']) {
        return true;
    }

    let normalized = normalize_for_compare(trimmed);
    let tokens = normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return false;
    }

    if tokens.len() == 1 {
        return !matches!(
            tokens[0],
            "agent"
                | "agents"
                | "agentic"
                | "benchmark"
                | "benchmarks"
                | "browser"
                | "company"
                | "comparison"
                | "credential"
                | "credentials"
                | "database"
                | "deployment"
                | "doc"
                | "docs"
                | "documentation"
                | "evidence"
                | "framework"
                | "frameworks"
                | "inference"
                | "integration"
                | "landscape"
                | "model"
                | "news"
                | "observability"
                | "pricing"
                | "product"
                | "prompt"
                | "provider"
                | "providers"
                | "rag"
                | "release"
                | "releases"
                | "retrieval"
                | "search"
                | "security"
                | "sentiment"
                | "snippet"
                | "snippets"
                | "stack"
                | "tool"
                | "tools"
                | "tradeoff"
                | "tradeoffs"
                | "update"
                | "vector"
                | "workflow"
                | "workflows"
        );
    }

    !tokens.iter().all(|token| {
        matches!(
            *token,
            "agent"
                | "agents"
                | "agentic"
                | "best"
                | "benchmark"
                | "benchmarks"
                | "browser"
                | "company"
                | "comparison"
                | "credential"
                | "credentials"
                | "current"
                | "database"
                | "deployment"
                | "enterprise"
                | "framework"
                | "frameworks"
                | "injection"
                | "integration"
                | "landscape"
                | "latest"
                | "model"
                | "news"
                | "observability"
                | "prompt"
                | "provider"
                | "providers"
                | "public"
                | "rag"
                | "recent"
                | "release"
                | "releases"
                | "research"
                | "retrieval"
                | "security"
                | "sentiment"
                | "snippet"
                | "snippets"
                | "stack"
                | "tool"
                | "tools"
                | "tradeoff"
                | "tradeoffs"
                | "update"
                | "vector"
                | "workflow"
                | "workflows"
        )
    })
}

fn retrieval_provider_quality(payload: &Value, normalized_prompt: &str) -> Value {
    let tool_executed = has_tool_execution(payload);
    let candidate_count = provider_candidate_count(payload);
    let evidence_count = provider_evidence_count(payload);
    let materialized_candidate_count = provider_materialized_candidate_count(payload);
    let content_rich_candidate_count = provider_content_rich_candidate_count(payload);
    let claim_hint_count = provider_claim_hint_count(payload);
    let materialization_failure_report =
        provider_explicit_quality_value(payload, &["materialization_failure_report"]);
    let prompt_relevance = evidence_prompt_relevance(payload, normalized_prompt);
    let topic_relevant_evidence = prompt_relevance
        .get("topic_relevant_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let relevant_evidence_count = prompt_relevance
        .get("relevant_evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
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
    let materialized_evidence_available =
        materialized_candidate_count > 0 && claim_hint_count > 0;
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
    } else if materialized_candidate_count == 0 || claim_hint_count == 0 {
        "low_signal"
    } else if explicit_low_signal {
        "low_signal"
    } else if evidence_count > 0 && !topic_relevant_evidence {
        "low_relevance"
    } else {
        "usable"
    };
    let usable_evidence = status == "usable";
    let allows_excellent = usable_evidence
        && content_rich_candidate_count > 0
        && claim_hint_count > 0
        && relevant_evidence_count >= 2;
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
    if tool_executed && evidence_count > 0 && materialized_candidate_count == 0 {
        flags.push("materialized_evidence_absent");
    }
    if tool_executed && evidence_count > 0 && content_rich_candidate_count == 0 {
        flags.push("content_rich_candidates_absent");
    }
    if tool_executed && evidence_count > 0 && claim_hint_count == 0 {
        flags.push("claim_hints_absent");
    }
    if tool_executed && evidence_count > 0 && !topic_relevant_evidence {
        flags.push("topic_relevance_absent");
    }
    flags.sort_unstable();
    flags.dedup();
    json!({
        "status": status,
        "tool_executed": tool_executed,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "materialized_candidate_count": materialized_candidate_count,
        "content_rich_candidate_count": content_rich_candidate_count,
        "claim_hint_count": claim_hint_count,
        "materialization_failure_report": materialization_failure_report,
        "materialized_evidence_available": materialized_evidence_available,
        "usable_evidence": usable_evidence,
        "allows_excellent": allows_excellent,
        "quality_flags": flags,
        "prompt_relevance": prompt_relevance,
        "classification_inputs": {
            "explicit_no_results_marker": explicit_no_results,
            "explicit_provider_degraded_marker": explicit_provider_degraded,
            "explicit_low_signal_marker": explicit_low_signal,
            "evidence_artifact_conflict": evidence_artifact_conflict,
            "materialized_candidate_count": materialized_candidate_count,
            "content_rich_candidate_count": content_rich_candidate_count,
            "claim_hint_count": claim_hint_count,
            "relevant_evidence_count": relevant_evidence_count,
            "topic_relevant_evidence": topic_relevant_evidence,
            "status_marker_source": "structured_tool_status_fields_only"
        },
        "note": "Excellent requires usable retrieval/provider evidence; low-evidence fallbacks may pass but cannot earn excellent."
    })
}

fn evidence_prompt_relevance(payload: &Value, normalized_prompt: &str) -> Value {
    let prompt_terms = research_prompt_topic_terms(normalized_prompt, 12);
    let evidence_texts = evidence_relevance_texts(payload);
    if prompt_terms.len() < 2 || evidence_texts.is_empty() {
        return json!({
            "schema_version": 1,
            "topic_relevant_evidence": true,
            "prompt_terms": prompt_terms,
            "evidence_text_count": evidence_texts.len(),
            "relevant_evidence_count": 0,
            "min_overlap_terms": 0,
            "note": "Prompt relevance was not enforced because the prompt had too few durable topic terms or no evidence text was available."
        });
    }
    let min_overlap = if prompt_terms.len() <= 3 { 1 } else { 2 };
    let relevant_evidence_count = evidence_texts
        .iter()
        .filter(|text| prompt_term_overlap_count(&prompt_terms, text) >= min_overlap)
        .count() as u64;
    json!({
        "schema_version": 1,
        "topic_relevant_evidence": relevant_evidence_count > 0,
        "prompt_terms": prompt_terms,
        "evidence_text_count": evidence_texts.len(),
        "relevant_evidence_count": relevant_evidence_count,
        "min_overlap_terms": min_overlap,
        "note": "Checks whether at least one evidence item overlaps the user's durable topic terms, so unrelated source rows do not count as usable research evidence."
    })
}

fn evidence_relevance_texts(payload: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for row in selected_tool_contexts(payload) {
        collect_evidence_relevance_texts(row, 0, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_evidence_relevance_texts(value: &Value, depth: usize, out: &mut Vec<String>) {
    if depth > 7 || out.len() >= 80 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_evidence_relevance_texts(row, depth + 1, out);
            }
        }
        Value::Object(map) => {
            let mut doc_parts = Vec::<String>::new();
            for key in [
                "title",
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
            ] {
                if let Some(child) = map.get(key) {
                    collect_relevance_doc_parts(child, depth + 1, &mut doc_parts);
                }
            }
            if !doc_parts.is_empty() {
                let combined = normalize_for_compare(&doc_parts.join(" "));
                if combined.split_whitespace().count() >= 3 {
                    out.push(combined);
                }
            }
            for key in [
                "evidence",
                "evidence_refs",
                "evidence_pack",
                "evidence_pack_candidates",
                "sources",
                "citations",
                "search_results",
                "provider_results",
            ] {
                if let Some(child) = map.get(key) {
                    collect_evidence_relevance_texts(child, depth + 1, out);
                }
            }
        }
        Value::String(raw) => {
            let cleaned = clean_text(raw, 1_000);
            if cleaned.split_whitespace().count() >= 3 {
                out.push(normalize_for_compare(&cleaned));
            }
        }
        _ => {}
    }
}

fn collect_relevance_doc_parts(value: &Value, depth: usize, out: &mut Vec<String>) {
    if depth > 7 || out.len() >= 32 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_relevance_doc_parts(row, depth + 1, out);
            }
        }
        Value::Object(map) => {
            for key in [
                "text",
                "snippet",
                "summary",
                "title",
                "content",
                "markdown",
                "body",
                "description",
                "abstract",
            ] {
                if let Some(child) = map.get(key) {
                    collect_relevance_doc_parts(child, depth + 1, out);
                }
            }
        }
        Value::String(raw) => {
            let cleaned = clean_text(raw, 500);
            if cleaned.split_whitespace().count() >= 2 {
                out.push(cleaned);
            }
        }
        _ => {}
    }
}

fn research_prompt_topic_terms(normalized_prompt: &str, limit: usize) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    for token in normalized_prompt.split_whitespace() {
        let token = normalize_research_token(token);
        if token.len() < 3 && token != "ai" {
            continue;
        }
        if research_prompt_stop_term(&token) {
            continue;
        }
        let stem = research_term_stem(&token);
        if stem.len() < 3 && stem != "ai" {
            continue;
        }
        if !terms.iter().any(|existing| existing == &stem) {
            terms.push(stem);
        }
        if terms.len() >= limit {
            break;
        }
    }
    terms
}

fn normalize_research_token(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn research_prompt_stop_term(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "after"
            | "against"
            | "also"
            | "answer"
            | "anything"
            | "around"
            | "before"
            | "best"
            | "but"
            | "between"
            | "blindly"
            | "browse"
            | "citation"
            | "citations"
            | "current"
            | "currently"
            | "defensible"
            | "doc"
            | "docs"
            | "documentation"
            | "does"
            | "explain"
            | "find"
            | "first"
            | "give"
            | "into"
            | "landscape"
            | "latest"
            | "look"
            | "looking"
            | "make"
            | "marketing"
            | "more"
            | "most"
            | "need"
            | "news"
            | "not"
            | "official"
            | "overview"
            | "page"
            | "pages"
            | "primary"
            | "research"
            | "release"
            | "releases"
            | "result"
            | "results"
            | "right"
            | "search"
            | "some"
            | "source"
            | "sources"
            | "summarize"
            | "tell"
            | "that"
            | "the"
            | "their"
            | "there"
            | "these"
            | "this"
            | "trust"
            | "update"
            | "using"
            | "web"
            | "what"
            | "when"
            | "where"
            | "which"
            | "while"
            | "with"
            | "would"
            | "january"
            | "february"
            | "march"
            | "april"
            | "may"
            | "june"
            | "july"
            | "august"
            | "september"
            | "october"
            | "november"
            | "december"
    )
}

fn research_term_stem(token: &str) -> String {
    let mut value = normalize_research_token(token);
    for suffix in ["ing", "ed", "es", "s"] {
        if value.len() > suffix.len() + 3 && value.ends_with(suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value
}

fn prompt_term_overlap_count(prompt_terms: &[String], normalized_text: &str) -> usize {
    let text_terms = normalized_text
        .split_whitespace()
        .map(research_term_stem)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    prompt_terms
        .iter()
        .filter(|term| text_terms.iter().any(|text_term| text_term == *term))
        .count()
}

struct ExcellentDiagnosticInput<'a> {
    retrieval_quality: &'a Value,
    citation_behavior: &'a Value,
    query_satisfaction: &'a Value,
    normalized_response: &'a str,
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
                | "low_relevance"
        );
    let evidence_gaps_named_when_needed = !needs_gap_statement || input.limitation_signal;
    let limitation_heavy_answer = limitation_heavy_for_excellent(input.normalized_response);
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
    subgates.insert(
        "excellent_10_answer_not_limitation_heavy".to_string(),
        json!(!limitation_heavy_answer),
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
        (
            "excellent_10_answer_not_limitation_heavy",
            "limitation_heavy_answer_shape",
        ),
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
        "limitation_heavy_answer": limitation_heavy_answer,
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
    let explicit_materialized =
        provider_explicit_quality_metric(payload, &["materialized_candidate_count"]);
    let explicit = if explicit_materialized > 0 {
        explicit_materialized
    } else {
        provider_explicit_quality_metric(
            payload,
            &["content_rich_candidate_count", "content_rich_item_count"],
        )
    };
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_content_rich_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn provider_materialized_candidate_count(payload: &Value) -> u64 {
    let explicit = provider_explicit_quality_metric(payload, &["materialized_candidate_count"]);
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_materialized_items(row, 0))
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

fn provider_explicit_quality_value(payload: &Value, value_keys: &[&str]) -> Value {
    selected_tool_contexts(payload)
        .iter()
        .find_map(|row| explicit_quality_value(row, value_keys, 0))
        .unwrap_or(Value::Null)
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

fn explicit_quality_value(value: &Value, value_keys: &[&str], depth: usize) -> Option<Value> {
    if depth > 7 {
        return None;
    }
    match value {
        Value::Object(map) => {
            for key in value_keys {
                if let Some(found) = map.get(*key) {
                    return Some(found.clone());
                }
            }
            map.values()
                .find_map(|row| explicit_quality_value(row, value_keys, depth + 1))
        }
        Value::Array(rows) => rows
            .iter()
            .find_map(|row| explicit_quality_value(row, value_keys, depth + 1)),
        _ => None,
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
            if let Some(false) = value_counts_as_usable_evidence(value) {
                return 0;
            }
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
            if let Some(false) = value_counts_as_usable_evidence(value) {
                return 0;
            }
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

fn count_materialized_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_materialized_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            let direct = value_counts_as_usable_evidence(value)
                .filter(|eligible| *eligible)
                .map(|_| 1)
                .unwrap_or(0);
            if direct > 0 {
                direct
            } else {
                semantic_child_values(map)
                    .map(|row| count_materialized_items(row, depth + 1))
                    .sum()
            }
        }
        _ => 0,
    }
}

fn value_counts_as_usable_evidence(value: &Value) -> Option<bool> {
    let map = value.as_object()?;
    if let Some(explicit) = map.get("counts_as_usable_evidence").and_then(Value::as_bool) {
        return Some(explicit);
    }
    let quality = map
        .get("materialization_quality")
        .and_then(Value::as_str)
        .map(normalize_for_compare)
        .or_else(|| {
            let source_kind = map.get("source_kind").and_then(Value::as_str).unwrap_or("");
            let permissions = map.get("permissions").and_then(Value::as_str).unwrap_or("");
            let snippet = map
                .get("snippet")
                .or_else(|| map.get("summary"))
                .or_else(|| map.get("content"))
                .or_else(|| map.get("markdown"))
                .or_else(|| map.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            infer_materialization_quality(source_kind, permissions, snippet)
        })?;
    Some(matches!(
        quality.as_str(),
        "full_materialized" | "partial_materialized" | "trusted_structured_feed"
    ))
}

fn infer_materialization_quality(
    source_kind: &str,
    permissions: &str,
    snippet: &str,
) -> Option<String> {
    let source_kind = normalize_for_compare(source_kind);
    let permissions = normalize_for_compare(permissions);
    if source_kind.is_empty() && permissions.is_empty() {
        return None;
    }
    let snippet_rich = content_rich_text(snippet);
    let materialized = source_kind.contains("materialized")
        || source_kind.contains("page_enriched")
        || source_kind.contains("document_page_artifact")
        || source_kind.contains("reader_output")
        || source_kind.contains("rendered_page")
        || source_kind.contains("page_artifact")
        || permissions.contains("browser_materialized");
    if materialized {
        return Some(if snippet_rich {
            "full_materialized".to_string()
        } else {
            "partial_materialized".to_string()
        });
    }
    let fetch_like = source_kind.contains("direct_fetch")
        || source_kind.contains("fetch_candidate")
        || permissions.contains("fetch_materialized");
    if fetch_like {
        return Some(if snippet_rich {
            "partial_materialized".to_string()
        } else {
            "failed_materialization".to_string()
        });
    }
    let trusted_structured_feed = source_kind.contains("rss")
        || source_kind.contains("feed")
        || source_kind.contains("api")
        || permissions.contains("structured_feed")
        || permissions.contains("headline_feed");
    if trusted_structured_feed {
        return Some("trusted_structured_feed".to_string());
    }
    Some("candidate_only".to_string())
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
        "low signal",
        "low-signal",
        "off topic",
        "off-topic",
        "no substantive",
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
    let aliases = entity_coverage_aliases(entity);
    aliases
        .iter()
        .any(|alias| normalized_response_covers_entity_alias(normalized_response, alias))
}

fn normalized_response_covers_entity_alias(normalized_response: &str, alias: &str) -> bool {
    let normalized_alias = normalize_for_compare(alias);
    if normalized_alias.is_empty() {
        return false;
    }
    if normalized_term_present(normalized_response, &normalized_alias) {
        return true;
    }
    if normalized_term_present(
        normalized_response,
        &simple_plural_variant(&normalized_alias),
    ) || normalized_term_present(
        normalized_response,
        &simple_singular_variant(&normalized_alias),
    ) {
        return true;
    }
    let tokens = normalized_alias
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| token_or_simple_variant_present(normalized_response, token))
}

fn entity_coverage_aliases(entity: &str) -> Vec<String> {
    let mut aliases = Vec::<String>::new();
    push_unique_alias(&mut aliases, entity);
    for alias in explicit_parenthetical_aliases(entity) {
        push_unique_alias(&mut aliases, &alias);
    }
    if let Some(acronym) = derived_initialism_alias(entity) {
        push_unique_alias(&mut aliases, &acronym);
    }
    aliases
}

fn coverage_entity_aliases(coverage_entities: &[String]) -> Value {
    Value::Object(
        coverage_entities
            .iter()
            .map(|entity| {
                (
                    entity.clone(),
                    json!(entity_coverage_aliases(entity)
                        .into_iter()
                        .filter(
                            |alias| normalize_for_compare(alias) != normalize_for_compare(entity)
                        )
                        .collect::<Vec<_>>()),
                )
            })
            .collect(),
    )
}

fn push_unique_alias(aliases: &mut Vec<String>, raw: &str) {
    let cleaned = clean_text(raw, 120);
    if cleaned.is_empty() {
        return;
    }
    let normalized = normalize_for_compare(&cleaned);
    if aliases
        .iter()
        .any(|existing| normalize_for_compare(existing) == normalized)
    {
        return;
    }
    aliases.push(cleaned);
}

fn explicit_parenthetical_aliases(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut rest = raw;
    while let Some(open_idx) = rest.find('(') {
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = after_open.find(')') else {
            break;
        };
        let alias = clean_text(&after_open[..close_idx], 40);
        if alias
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch.is_whitespace())
            && alias
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .count()
                >= 2
        {
            out.push(alias);
        }
        rest = &after_open[close_idx + 1..];
    }
    out
}

fn derived_initialism_alias(raw: &str) -> Option<String> {
    let tokens = raw
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| !entity_initialism_stopword(token))
        .collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let acronym = tokens
        .iter()
        .filter_map(|token| token.chars().next())
        .collect::<String>()
        .to_ascii_uppercase();
    let len = acronym.chars().count();
    if (3..=8).contains(&len) {
        Some(acronym)
    } else {
        None
    }
}

fn entity_initialism_stopword(raw: &str) -> bool {
    matches!(
        normalize_for_compare(raw).as_str(),
        "a" | "an"
            | "and"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "in"
            | "of"
            | "on"
            | "or"
            | "the"
            | "to"
            | "vs"
            | "with"
    )
}

fn normalized_term_present(normalized_response: &str, normalized_term: &str) -> bool {
    if normalized_term.is_empty() {
        return false;
    }
    if normalized_term.split_whitespace().count() > 1 {
        return normalized_response.contains(normalized_term);
    }
    if normalized_term.len() <= 4 {
        return normalized_response
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|token| token == normalized_term);
    }
    normalized_response.contains(normalized_term)
}

fn token_or_simple_variant_present(normalized_response: &str, token: &str) -> bool {
    normalized_term_present(normalized_response, token)
        || normalized_term_present(normalized_response, &simple_plural_variant(token))
        || normalized_term_present(normalized_response, &simple_singular_variant(token))
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

        let quality =
            retrieval_provider_quality(&payload, "rendered research page source backed synthesis");
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
            Some(false)
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

        let quality = retrieval_provider_quality(&payload, "rag stack options");
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
    fn entity_coverage_accepts_derived_initialism_aliases() {
        let response = normalize_for_compare(
            "The MCP ecosystem has strong momentum, but product teams should avoid \
             overcommitting to unstable server behavior without source-backed checks.",
        );
        assert!(normalized_response_covers_entity(
            &response,
            "Model Context Protocol"
        ));
        assert_eq!(
            entity_coverage(&response, &["Model Context Protocol".to_string()]),
            1.0
        );
    }

    #[test]
    fn query_satisfaction_reports_entity_aliases_without_requiring_format() {
        let response = normalize_for_compare(
            "According to source evidence, MCP is useful as an integration pattern, \
             but the ecosystem still has maturity and security gaps.",
        );
        let entities = vec!["Model Context Protocol".to_string()];
        let coverage = entity_coverage(&response, &entities);
        let satisfaction = query_satisfaction(
            &normalize_for_compare("Research the current Model Context Protocol ecosystem."),
            &response,
            &entities,
            coverage,
            true,
            true,
            true,
            true,
        );
        assert_eq!(
            satisfaction.get("scope_covered").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            satisfaction
                .pointer("/coverage_entity_aliases/Model Context Protocol/0")
                .and_then(Value::as_str),
            Some("MCP")
        );
    }

    #[test]
    fn query_satisfaction_does_not_mark_goal_coverage_gap_as_excellent_answer() {
        let response = normalize_for_compare(
            "I don't have usable source-backed evidence for this turn. What the evidence covers: none. \
             What the evidence misses: everything specific to your research goal. Next search direction: try a narrower query.",
        );
        let entities = vec!["Mastra".to_string(), "LangGraph".to_string()];
        let coverage = entity_coverage(&response, &entities);
        let satisfaction = query_satisfaction(
            &normalize_for_compare(
                "Research Mastra and compare it with LangGraph for TypeScript agent workflows.",
            ),
            &response,
            &entities,
            coverage,
            true,
            true,
            true,
            true,
        );
        assert_eq!(
            satisfaction
                .get("coverage_gap_prevents_answer")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            satisfaction.get("intent_answered").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            satisfaction.get("decision_value").and_then(Value::as_bool),
            Some(false)
        );
        assert!(
            satisfaction
                .get("score")
                .and_then(Value::as_u64)
                .unwrap_or(10)
                < 9
        );
    }

    #[test]
    fn grade_case_counts_initialism_alias_as_user_entity_coverage() {
        let case = json!({
            "prompt": "Research the current Model Context Protocol ecosystem and summarize maturity and risk.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "batch_query",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Model Context Protocol"]
        });
        let payload = json!({
            "response": "According to source evidence, the MCP ecosystem has strong integration momentum, but product teams should avoid overcommitting to immature server behavior. The practical recommendation is to design around the pattern while keeping adapters replaceable and treating security boundaries as still evolving.",
            "pending_tool_request": {
                "status": "executed",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Research query pack",
                "tool_name": "batch_query",
                "tool_key": "batch_query",
                "input": {
                    "source": "web",
                    "query": "Research the current Model Context Protocol ecosystem.",
                    "queries": ["Model Context Protocol ecosystem maturity risk"],
                    "keywords": ["Model Context Protocol", "MCP", "maturity", "risk"],
                    "required_coverage": {"entities": ["Model Context Protocol"], "facets": ["maturity", "risk"]},
                    "aliases": ["MCP"],
                    "aperture": "medium"
                }
            },
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 4,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "MCP ecosystem source",
                    "locator": "https://example.test/mcp",
                    "snippet": "This source describes the MCP ecosystem, maturity signals, risks, and integration behavior with enough detail to support synthesis.",
                    "claim_hints": ["MCP ecosystem maturity varies by implementation."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(grade.coverage_entities, vec!["Model Context Protocol"]);
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
    }

    #[test]
    fn short_derived_initialisms_are_not_used_as_loose_entity_aliases() {
        assert_eq!(derived_initialism_alias("Artificial Intelligence"), None);
        let response =
            normalize_for_compare("AI safety is discussed, but no country coverage appears.");
        assert!(!normalized_response_covers_entity(
            &response,
            "Artificial Intelligence"
        ));
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
    fn real_conversation_source_summary_is_not_a_passing_research_answer() {
        let case = json!({
            "prompt": "what are some scientific breakthroughs 2026?",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            }
        });
        let payload = json!({
            "response": "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet; any decision should stay conservative until coverage improves. Recorded evidence so far: Here's what I found:\n\nweb search: From web retrieval: www.nature.com: New tools drive scientific discovery: evidence from all nobel-prize and major non-nobel breakthroughs Nature; Spring 2026 University of Miami Medicine Magazine Highlights Breakthroughs in Heart, Vision and Cancer Research; Nine scientific breakthroughs I’d like to see in 2026. The current turn does not yet support a complete answer to: what are some scientific breakthroughs 2026?. The current tradeoff is breadth versus confidence: we can stay narrow and source-backed on the covered evidence, or broaden retrieval before making a stronger claim. My recommendation is to treat this as a partial answer.",
            "pending_tool_request": {
                "status": "executed",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "what are some scientific breakthroughs 2026?",
                    "keywords": ["scientific breakthroughs", "2026"],
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "New tools drive scientific discovery",
                    "locator": "https://www.nature.com/example",
                    "snippet": "New tools drive scientific discovery: evidence from Nobel-prize and major non-Nobel breakthroughs.",
                    "claim_hints": ["Scientific discovery depends on new tools."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(!grade.pass, "{:?}", grade.failures);
        assert!(grade
            .failures
            .iter()
            .any(|failure| failure == "source_summary_without_user_answer"));
        assert_eq!(
            grade
                .soft_quality_smoke
                .get("pass")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            grade
                .soft_quality_smoke
                .get("top_blocker")
                .and_then(Value::as_str),
            Some("meta_process_talk_visible")
        );
    }

    #[test]
    fn off_topic_evidence_does_not_count_as_usable_research_data() {
        let payload = json!({
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 3,
                "evidence_refs": [
                    {
                        "title": "Most Concerning Question Mark Ravens Face With Rookie TE Matthew Hibner",
                        "locator": "https://www.si.com/example",
                        "snippet": "Sports Illustrated published a story about the Baltimore Ravens and a rookie tight end.",
                        "claim_hints": ["The Ravens have a roster question."]
                    },
                    {
                        "title": "Clinical gaps and legal loopholes paved the way for the Virginia Tech tragedy",
                        "locator": "https://www.psychologytoday.com/example",
                        "snippet": "A psychology article discusses clinical gaps and legal loopholes.",
                        "claim_hints": ["Clinical gaps shaped a tragedy."]
                    },
                    {
                        "title": "Leaders Seek to Address Big Question Mark Around Private Markets",
                        "locator": "https://www.thinkadvisor.com/example",
                        "snippet": "A finance article discusses private market uncertainty.",
                        "claim_hints": ["Private markets face uncertainty."]
                    }
                ]
            }]
        });

        let quality = retrieval_provider_quality(
            &payload,
            &normalize_for_compare("give me an update on the AI agentic landscape in May 2026"),
        );
        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("low_relevance"),
            "{quality:#?}"
        );
        assert_eq!(
            quality
                .pointer("/prompt_relevance/topic_relevant_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{quality:#?}"
        );
    }

    #[test]
    fn prompt_relevance_strips_instruction_words_and_punctuation() {
        let relevance = evidence_prompt_relevance(
            &json!({
                "tools": [{
                    "name": "web_search",
                    "status": "ok",
                    "evidence_refs": [{
                        "title": "Retail result",
                        "locator": "https://example.test/best-buy",
                        "snippet": "Best Buy store page and shopping deals for electronics in 2026."
                    }]
                }]
            }),
            &normalize_for_compare(
                "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer."
            ),
        );
        let prompt_terms = relevance
            .get("prompt_terms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        assert!(prompt_terms.iter().any(|term| term == "agentic"));
        assert!(prompt_terms.iter().any(|term| term == "framework"));
        assert!(!prompt_terms.iter().any(|term| term == "search"));
        assert!(!prompt_terms.iter().any(|term| term == "best"));
        assert!(!prompt_terms.iter().any(|term| term == "trust"));
        assert!(!prompt_terms.iter().any(|term| term == "page"));
    }

    #[test]
    fn excellent_requires_more_than_one_relevant_evidence_item() {
        let payload = json!({
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 6,
                "content_rich_candidate_count": 4,
                "claim_hint_count": 3,
                "evidence_refs": [
                    {
                        "title": "Single relevant framework page",
                        "locator": "https://example.test/framework",
                        "snippet": "This page discusses one agentic framework and its 2026 roadmap.",
                        "claim_hints": ["One framework has a 2026 roadmap."]
                    },
                    {
                        "title": "Retail page",
                        "locator": "https://example.test/store",
                        "snippet": "Best Buy store page for electronics.",
                        "claim_hints": ["Retail result."]
                    }
                ]
            }]
        });

        let quality = retrieval_provider_quality(
            &payload,
            &normalize_for_compare(
                "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer."
            ),
        );
        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("usable"),
            "{quality:#?}"
        );
        assert_eq!(
            quality.get("allows_excellent").and_then(Value::as_bool),
            Some(false),
            "{quality:#?}"
        );
        assert_eq!(
            quality
                .pointer("/classification_inputs/relevant_evidence_count")
                .and_then(Value::as_u64),
            Some(1),
            "{quality:#?}"
        );
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
    fn generic_required_noun_phrases_do_not_become_entity_coverage_requirements() {
        let case = json!({
            "prompt": "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer.",
            "required_entities": ["agentic framework"]
        });
        let payload = json!({
            "response": "Based on the retrieved evidence, LangGraph is the most defensible production default in 2026. The current evidence favors it on reliability and cost, while other frameworks look better for narrower use cases or prototypes.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 4,
                "content_rich_candidate_count": 4,
                "claim_hint_count": 3,
                "evidence_refs": [{
                    "title": "Framework comparison",
                    "locator": "https://example.test/framework-comparison",
                    "snippet": "LangGraph, CrewAI, and AutoGen are compared for production tradeoffs in 2026.",
                    "claim_hints": ["LangGraph is the most production-ready default among the compared frameworks."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(
            grade.coverage_entities.is_empty(),
            "{:#?}",
            grade.coverage_entities
        );
        assert!(
            !grade
                .failures
                .iter()
                .any(|failure| failure.starts_with("entity_coverage_low")),
            "{:#?}",
            grade.failures
        );
    }

    #[test]
    fn lowercase_hyphenated_product_names_still_count_as_specific_entities() {
        let case = json!({
            "prompt": "Compare browser-use with Playwright for browser agent workflows.",
            "required_entities": ["browser-use", "Playwright"]
        });
        let payload = json!({
            "response": "For browser-agent workflows, browser-use is more agent-native while Playwright is stronger for deterministic automation and testability.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Browser automation comparison",
                    "locator": "https://example.test/browser-compare",
                    "snippet": "browser-use and Playwright serve different needs in browser-agent systems.",
                    "claim_hints": ["browser-use is more agent-native while Playwright is more deterministic."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(
            grade.coverage_entities,
            vec!["browser-use".to_string(), "Playwright".to_string()]
        );
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
        let retrieval_quality =
            retrieval_provider_quality(&payload, "research agent workflow evidence");
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
    fn citation_behavior_accepts_final_package_source_refs() {
        let payload = json!({
            "response": "The answer gives a recommendation while citations are carried as final-package metadata.",
            "response_finalization": {
                "source_refs": [{
                    "citation_id": "source_1",
                    "title": "Usable source",
                    "locator": "https://example.test/source"
                }]
            },
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
        let retrieval_quality =
            retrieval_provider_quality(&payload, "research agent workflow evidence");
        let behavior = citation_behavior(
            &payload,
            "The answer gives a recommendation while citations are carried as final-package metadata.",
            &retrieval_quality,
        );
        assert_eq!(
            behavior.get("citation_signal").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            behavior
                .get("synthesis_ignored_citable_evidence")
                .and_then(Value::as_bool),
            Some(false)
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

    #[test]
    fn bounded_low_evidence_fallback_can_pass_without_earning_excellent() {
        let case = json!({
            "prompt": "Research Mastra for TypeScript agent workflows and compare it with LangGraph.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Mastra", "TypeScript", "LangGraph"]
        });
        let payload = json!({
            "response": "I don't have usable source-backed evidence about Mastra for this turn. The search returned largely off-topic snippets that do not cover Mastra's architecture, strengths, weaknesses, or how it compares to LangGraph for TypeScript agent workflows. Safe guidance given current limits: verify Mastra directly against its official documentation or repository, and treat LangGraph as the better-documented baseline until Mastra-specific evidence is available. Next search direction: try a narrower query for Mastra framework documentation or repository material.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "Mastra LangGraph TypeScript agent workflows",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 4,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Generic AI agent roundup",
                    "locator": "https://example.test/agent-roundup",
                    "snippet": "Mentions LangGraph for agent workflows and generic TypeScript tooling, but not Mastra itself.",
                    "claim_hints": ["LangGraph is used for agent workflows."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.pass, "{:?}", grade.failures);
        assert!(!grade.excellent);
        assert!(grade
            .excellent_blockers
            .contains(&"query_satisfaction_below_excellent".to_string()));
    }

    #[test]
    fn limitation_heavy_opening_blocks_excellent_even_when_answer_is_structured() {
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
            "response": "I found very limited evidence for this comparison, and the recorded evidence is insufficient for a fully source-backed conclusion. What the recorded evidence actually shows is narrow, but the practical tradeoff still points one way: Alpha looks steadier for production reliability, while Beta is better for exploratory flexibility. My bounded recommendation is Alpha for production and Beta for experiments.",
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
                "candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Alpha and Beta production comparison",
                    "locator": "https://example.test/alpha-beta-production",
                    "snippet": "A substantive source comparing Alpha and Beta for production reliability and experimentation tradeoffs.",
                    "claim_hints": ["Alpha is the steadier production default.", "Beta is better for exploratory work."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.pass, "{:?}", grade.failures);
        assert!(!grade.excellent);
        assert!(grade
            .excellent_blockers
            .contains(&"limitation_heavy_answer_shape".to_string()));
        assert_eq!(
            grade
                .excellent_diagnostics
                .pointer("/subgates/excellent_10_answer_not_limitation_heavy")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn grade_case_emits_layered_response_grading_output() {
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
            "response": "According to the docs and release notes, Alpha is the steadier production default, while Beta is stronger for exploration. The practical tradeoff is reliability versus flexibility. My recommendation is Alpha for production and Beta for experiments.",
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
                    "snippet": "A substantive source comparing Alpha and Beta for reliability and flexibility.",
                    "claim_hints": ["Alpha is steadier for production."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(
            grade
                .response_grading_layers
                .pointer("/generic_response_contract/pass")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            grade
                .response_grading_layers
                .pointer("/tool_backed_evidence_contract/pass")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            grade
                .response_grading_layers
                .pointer("/workflow_specific_rubric/pass")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            grade
                .soft_quality_smoke
                .get("pass")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn soft_quality_smoke_allows_mild_evidence_caveat_when_answer_is_still_direct() {
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
            "response": "Based on the limited evidence retrieved and the coverage gaps noted in the state, Alpha is still the safer production default, while Beta is better for exploratory work. The practical tradeoff is reliability versus flexibility, so I would choose Alpha for production and Beta for experiments.",
            "pending_tool_request": {
                "status": "executed",
                "selected_tool_family": "web_research",
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
                    "snippet": "A substantive source comparing Alpha and Beta for reliability and flexibility.",
                    "claim_hints": ["Alpha is steadier for production."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(
            grade
                .soft_quality_smoke
                .get("pass")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn unsupported_claim_signal_allows_explicit_low_signal_rejection_of_best_claim() {
        let case = json!({
            "prompt": "What is the best option for this research task?"
        });
        let response = "The retrieval was low-signal and off-topic, so the evidence does not support naming the best option. Claim: \"X is the best option\". Supported? No.";

        assert!(!unsupported_claim_signal(&case, response));
    }

    #[test]
    fn source_dump_retry_template_is_not_a_good_user_answer() {
        let normalized = normalize_for_compare(
            "This retrieval attempt did not produce enough balanced evidence to make a source-backed comparison. Recorded evidence so far: Here's what I found: web search returned low-signal snippets. Retry with a narrower query.",
        );

        assert!(source_summary_without_answer_signal(&normalized));
    }

    #[test]
    fn evidence_layer_allows_qualified_relevance_denial() {
        let retrieval_quality = json!({
            "tool_executed": true,
            "usable_evidence": true,
            "status": "usable"
        });
        let citation_behavior = json!({
            "evidence_count": 2,
            "citation_signal": true,
            "response_source_signal": true,
            "synthesis_ignored_citable_evidence": false
        });
        let query_satisfaction = json!({
            "scope_covered": true
        });

        let layer = tool_backed_evidence_contract(
            &normalize_for_compare(
                "I found evidence, but it does not cover the named product. The retrieved rows are false positives, so there is no source-backed basis to choose a winner.",
            ),
            &retrieval_quality,
            &citation_behavior,
            true,
            &query_satisfaction,
            false,
        );
        assert_eq!(layer.get("pass").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn evidence_layer_rejects_claim_that_recorded_evidence_does_not_exist() {
        let retrieval_quality = json!({
            "tool_executed": true,
            "usable_evidence": true,
            "status": "usable"
        });
        let citation_behavior = json!({
            "evidence_count": 2,
            "citation_signal": false,
            "response_source_signal": false,
            "synthesis_ignored_citable_evidence": true
        });
        let query_satisfaction = json!({
            "scope_covered": true
        });

        let layer = tool_backed_evidence_contract(
            &normalize_for_compare(
                "No source-backed findings are available yet, so I cannot answer this from the recorded state."
            ),
            &retrieval_quality,
            &citation_behavior,
            true,
            &query_satisfaction,
            false,
        );
        assert_eq!(layer.get("pass").and_then(Value::as_bool), Some(false));
        assert_eq!(
            layer.get("top_blocker").and_then(Value::as_str),
            Some("recorded_evidence_not_used")
        );
        assert_eq!(
            layer
                .pointer("/subgates/evidence_4_does_not_overclaim_or_deny_recorded_state")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn citation_behavior_accepts_domain_style_source_mentions() {
        let behavior = citation_behavior(
            &json!({}),
            "The strongest current signal favors Alpha for production (langchain.com) while Beta remains better for exploration.",
            &json!({
                "usable_evidence": true,
                "evidence_count": 2
            }),
        );
        assert_eq!(
            behavior
                .get("response_source_signal")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            behavior.get("citation_signal").and_then(Value::as_bool),
            Some(true)
        );
    }
}
