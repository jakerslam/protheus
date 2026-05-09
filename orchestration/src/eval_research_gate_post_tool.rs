use super::super::eval_research_golden_utils::*;
use serde_json::Value;

pub(super) fn raw_provider_result_present(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    tool_rows(payload)
        .iter()
        .any(|row| tool_row_has_raw_provider_result(row))
        || raw_provider_result_paths(payload).iter().any(|path| {
            let pointer = format!("/{}", path.replace('.', "/"));
            payload
                .pointer(&pointer)
                .map(value_has_substantive_result)
                .unwrap_or(false)
        })
}

pub(super) fn packaged_tool_result_present(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    if bool_pointer_any(
        payload,
        &[
            "/response_finalization/findings_available",
            "/response_finalization/tool_completion/findings_available",
        ],
    ) {
        return true;
    }
    tool_rows(payload)
        .iter()
        .any(|row| tool_row_has_packaged_result(row))
}

pub(super) fn evidence_extracted(payload: &Value) -> bool {
    evidence_paths(payload).iter().any(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
    })
}

pub(super) fn agent_received_evidence_context(payload: &Value) -> bool {
    if !evidence_extracted(payload) {
        return false;
    }
    if agent_evidence_context_paths(payload).iter().any(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
    }) {
        return true;
    }
    response_has_source_signal(&normalize_for_compare(&assistant_text(payload)))
}

pub(super) fn synthesis_uses_evidence_or_low_evidence_fallback(
    _case: &Value,
    payload: &Value,
    packaged_tool_result: bool,
    evidence_extracted: bool,
) -> bool {
    let response = assistant_text(payload);
    let normalized = normalize_for_compare(&response);
    if normalized.is_empty() {
        return false;
    }
    if !has_tool_execution(payload)
        && response_matches_explicit_missing_tool_context_contract(&normalized)
    {
        return true;
    }
    if !has_tool_execution(payload) && response_acknowledges_missing_tool_context(&normalized) {
        let has_bounded_missing_context_fallback =
            response_has_missing_tool_context_shape(&normalized)
                || response_has_research_shape(&normalized)
                || response_has_low_evidence_signal(&normalized)
                || normalized.contains("what i know")
                || normalized.contains("what we know");
        return has_bounded_missing_context_fallback
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    if tool_result_low_signal(payload) {
        return response_has_low_evidence_signal(&normalized)
            && response_has_research_shape(&normalized)
            && required_entity_coverage(_case, &normalized) >= 0.75
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    if evidence_extracted || packaged_tool_result {
        return response_has_source_signal(&normalized)
            && response_has_research_shape(&normalized)
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    false
}

pub(super) fn raw_provider_result_paths(payload: &Value) -> Vec<String> {
    post_tool_paths(
        payload,
        &[
            "raw",
            "raw_result",
            "raw_results",
            "provider_result",
            "provider_results",
            "search_results",
            "organic_results",
            "web_results",
            "raw_result_ref",
            "raw_result_refs",
        ],
        value_has_raw_provider_artifact,
    )
}

pub(super) fn packaged_tool_result_paths(payload: &Value) -> Vec<String> {
    post_tool_paths(
        payload,
        &[
            "result",
            "summary",
            "findings",
            "sources",
            "citations",
            "evidence",
            "evidence_refs",
            "items",
            "results",
            "data",
        ],
        value_has_substantive_result,
    )
}

pub(super) fn evidence_paths(payload: &Value) -> Vec<String> {
    let mut paths = [
        "evidence",
        "evidence_bundle",
        "evidence_refs",
        "sources",
        "citations",
        "response_workflow.evidence",
        "response_workflow.evidence_bundle",
        "response_workflow.evidence_refs",
        "response_workflow.sources",
        "response_workflow.citations",
        "response_finalization.evidence",
        "response_finalization.evidence_bundle",
        "response_finalization.evidence_refs",
        "response_finalization.tool_completion.evidence_refs",
        "response_finalization.tool_completion.findings",
    ]
    .iter()
    .filter_map(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
            .then(|| (*path).to_string())
    })
    .collect::<Vec<_>>();
    for path in post_tool_paths(
        payload,
        &[
            "evidence",
            "evidence_bundle",
            "evidence_refs",
            "sources",
            "citations",
            "findings",
        ],
        value_has_content,
    ) {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    paths
}

pub(super) fn agent_evidence_context_paths(payload: &Value) -> Vec<String> {
    [
        "response_workflow.final_llm_response.evidence_refs",
        "response_workflow.final_llm_response.evidence_refs_used",
        "response_workflow.final_llm_response.sources",
        "response_workflow.final_prompt_context.evidence_refs",
        "response_workflow.synthesis_context.evidence_refs",
        "response_finalization.evidence_context",
        "response_finalization.synthesis_context.evidence_refs",
        "response_finalization.tool_completion.evidence_refs_used",
    ]
    .iter()
    .filter_map(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
            .then(|| (*path).to_string())
    })
    .collect()
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

fn tool_row_has_raw_provider_result(row: &Value) -> bool {
    [
        "raw",
        "raw_result",
        "raw_results",
        "provider_result",
        "provider_results",
        "search_results",
        "organic_results",
        "web_results",
        "raw_result_ref",
        "raw_result_refs",
    ]
    .iter()
    .any(|key| {
        row.get(*key)
            .map(value_has_raw_provider_artifact)
            .unwrap_or(false)
    })
}

fn tool_row_has_packaged_result(row: &Value) -> bool {
    for key in [
        "sources",
        "citations",
        "evidence",
        "evidence_refs",
        "items",
        "results",
        "data",
    ] {
        if value_has_content(row.get(key).unwrap_or(&Value::Null)) {
            return true;
        }
    }
    let result = str_at(row, &["result"], "");
    value_has_substantive_result(&Value::String(result))
}

fn tool_result_low_signal(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    let finalization =
        normalize_for_compare(&response_finalization_outcome(payload).unwrap_or_default());
    if finalization.contains("low_signal")
        || finalization.contains("no_results")
        || finalization.contains("tool_failure")
    {
        return true;
    }
    for pointer in [
        "/response_finalization/tool_completion/completion_state",
        "/response_finalization/tool_completion/reasoning",
    ] {
        if payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(text_has_low_signal_only)
            .unwrap_or(false)
        {
            return true;
        }
    }
    bool_pointer_any(
        payload,
        &[
            "/response_finalization/tool_completion/final_no_findings",
            "/response_finalization/tool_completion/final_requests_more_tooling",
        ],
    ) || payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().all(tool_row_is_low_signal))
        .unwrap_or(false)
}

fn tool_row_is_low_signal(row: &Value) -> bool {
    let status = normalize_for_compare(&str_at(row, &["status"], ""));
    row_status_is_failure_or_empty(&status)
        || row
            .get("result")
            .and_then(Value::as_str)
            .map(text_has_low_signal_only)
            .unwrap_or(false)
}

fn row_status_is_failure_or_empty(status: &str) -> bool {
    matches!(
        status,
        "low_signal"
            | "no_results"
            | "partial_no_results"
            | "error"
            | "failed"
            | "timeout"
            | "blocked"
            | "policy_denied"
    )
}

fn text_has_low_signal_only(raw: &str) -> bool {
    let normalized = normalize_for_compare(raw);
    [
        "low signal",
        "no usable findings",
        "no usable result",
        "no results",
        "not enough source coverage",
        "limited evidence",
        "limited results",
        "weak evidence",
        "off topic",
        "off-topic",
        "off target",
        "off-target",
        "irrelevant",
        "inconclusive",
        "retrieval missed",
        "retrieval gap",
        "did not produce enough",
        "could not find enough",
        "narrow the query",
        "need a tighter query",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_source_signal(normalized: &str) -> bool {
    [
        "source",
        "evidence",
        "according",
        "docs",
        "release",
        "citation",
        "http://",
        "https://",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_low_evidence_signal(normalized: &str) -> bool {
    [
        "low signal",
        "limited evidence",
        "source coverage",
        "limited results",
        "limited source",
        "weak evidence",
        "off topic",
        "off-topic",
        "off target",
        "off-target",
        "retrieval missed",
        "retrieval gap",
        "inconclusive",
        "insufficient",
        "not enough",
        "no usable findings",
        "caveat",
        "uncertain",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_research_shape(normalized: &str) -> bool {
    normalized.split_whitespace().count() >= 40
        && [
            "tradeoff",
            "trade-off",
            "compare",
            "comparison",
            "versus",
            "vs",
            "recommend",
            "best for",
            "criteria",
            "dimension",
            "bounded conclusion",
            "practical implication",
            "current state",
            "supports",
            "does not support",
            "risk",
            "limitation",
            "uncertainty",
            "evidence",
            "source-backed",
            "maturity",
            "security",
            "evaluate",
            "avoid",
        ]
        .iter()
        .any(|needle| normalized.contains(*needle))
}

fn response_requests_more_scope_without_substance(normalized: &str) -> bool {
    let has_scope_request = [
        "narrow the query",
        "pick 2",
        "pick two",
        "which specific",
        "would you prefer",
        "need a tighter query",
        "provide a specific source",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    if !has_scope_request {
        return false;
    }
    let has_bounded_substance = normalized.split_whitespace().count() >= 45
        && (response_has_research_shape(normalized)
            || response_has_low_evidence_signal(normalized)
            || normalized.contains("supports")
            || normalized.contains("does not support")
            || normalized.contains("bounded"));
    !has_bounded_substance
}

fn response_acknowledges_missing_tool_context(normalized: &str) -> bool {
    [
        "no live web data",
        "no returned tool result",
        "tool result is not present in this turn",
        "tool result is not available in this turn",
        "no retrieved snippets",
        "no retrieved results",
        "i havent actually executed any web search",
        "i do not have the tool result",
        "i don't have the tool result",
        "no recorded tool outcome",
        "would require live research",
        "requires live research",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_missing_tool_context_shape(normalized: &str) -> bool {
    let has_knowns = normalized.contains("what i know")
        || normalized.contains("what we know")
        || normalized.contains("from my current context");
    let has_unknowns = normalized.contains("what i do not know")
        || normalized.contains("what we do not know")
        || normalized.contains("would require live research")
        || normalized.contains("requires live research");
    let has_next_step = normalized.contains("next best")
        || normalized.contains("next useful action")
        || normalized.contains("next step")
        || normalized.contains("next query")
        || normalized.contains("follow up query")
        || normalized.contains("follow-up query")
        || normalized.contains("search query");
    normalized.split_whitespace().count() >= 35 && has_knowns && (has_unknowns || has_next_step)
}

fn response_matches_explicit_missing_tool_context_contract(normalized: &str) -> bool {
    normalized.starts_with(
        "no returned tool result is available in this turn so no receipt backed synthesis is available yet",
    ) && normalized.contains("what we know")
        && normalized.contains("what we do not know")
        && (normalized.contains("source") || normalized.contains("evidence"))
        && normalized.contains("recommend")
        && normalized.contains("next best search query")
}

fn response_uses_internal_runtime_context_as_evidence(normalized: &str) -> bool {
    [
        "identity context",
        "system instruction",
        "system instructions",
        "agent name",
        "hosting this conversation",
        "evident from system",
        "workspace metadata",
        "platform identity",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn required_entity_coverage(case: &Value, normalized_response: &str) -> f64 {
    let entities = string_array_at(case, &["required_entities"]);
    if entities.is_empty() {
        return 1.0;
    }
    let covered = entities
        .iter()
        .filter(|entity| normalized_response.contains(&normalize_for_compare(entity)))
        .count() as u64;
    ratio(covered, entities.len() as u64)
}

fn post_tool_paths(payload: &Value, keys: &[&str], predicate: fn(&Value) -> bool) -> Vec<String> {
    let mut paths = Vec::new();
    for (prefix, rows) in [
        ("tools", payload.get("tools").and_then(Value::as_array)),
        (
            "response_finalization.tool_completion.tool_attempts",
            payload
                .pointer("/response_finalization/tool_completion/tool_attempts")
                .and_then(Value::as_array),
        ),
    ] {
        if let Some(rows) = rows {
            for (idx, row) in rows.iter().enumerate() {
                for key in keys {
                    if row.get(*key).map(predicate).unwrap_or(false) {
                        paths.push(format!("{prefix}.{idx}.{key}"));
                    }
                }
            }
        }
    }
    paths
}

fn bool_pointer_any(payload: &Value, pointers: &[&str]) -> bool {
    pointers.iter().any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn value_has_substantive_result(value: &Value) -> bool {
    match value {
        Value::String(raw) => {
            !raw.trim().is_empty()
                && raw.split_whitespace().count() >= 8
                && !text_has_low_signal_only(raw)
        }
        Value::Array(rows) => !rows.is_empty() && rows.iter().any(value_has_substantive_result),
        Value::Object(map) => !map.is_empty() && map.values().any(value_has_substantive_result),
        other => value_has_content(other),
    }
}

fn value_has_raw_provider_artifact(value: &Value) -> bool {
    match value {
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => !rows.is_empty() && rows.iter().any(value_has_raw_provider_artifact),
        Value::Object(map) => {
            [
                "provider",
                "query",
                "summary",
                "error",
                "links",
                "locator",
                "snippet",
                "title",
                "provider_raw_count",
                "provider_filtered_count",
            ]
            .iter()
            .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
                || map.values().any(value_has_raw_provider_artifact)
        }
        other => value_has_content(other),
    }
}

fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => !rows.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

fn response_finalization_outcome(payload: &Value) -> Option<String> {
    payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 600))
}
