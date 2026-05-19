use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::super::eval_research_golden_utils::{
    clean_text, normalize_for_compare, read_json, str_at, string_array_at,
};

pub(super) fn request_pack_for_case(
    case: &Value,
    report_request: Option<&Value>,
    default_tool: &str,
) -> Value {
    if let Some(request) = report_request {
        return json!({
            "request_pack_source": "research_report_pending_tool_request",
            "tool_name": str_at(request, &["tool_name"], default_tool),
            "input": request.get("input").cloned().unwrap_or_else(|| json!({}))
        });
    }
    if let Some(request) = case.get("tooling_request").and_then(Value::as_object) {
        return json!({
            "request_pack_source": "case_tooling_request",
            "tool_name": request
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or(default_tool),
            "input": Value::Object(request.clone())
        });
    }
    let prompt = str_at(case, &["prompt"], "");
    let required_entities = string_array_at(case, &["required_entities"]);
    let keywords = derived_keywords(&prompt, &required_entities);
    json!({
        "request_pack_source": "derived_minimal_prompt_request",
        "tool_name": default_tool,
        "input": {
            "source": "web",
            "query": prompt,
            "queries": [prompt],
            "keywords": keywords,
            "required_coverage": {
                "entities": required_entities,
                "facets": []
            },
            "aperture": "medium",
            "query_metadata_policy": {
                "classification": "derived_prompt_request"
            }
        }
    })
}

fn derived_keywords(prompt: &str, required_entities: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for entity in required_entities {
        let cleaned = clean_text(entity, 160);
        if !cleaned.is_empty() && !out.iter().any(|current| current == &cleaned) {
            out.push(cleaned);
        }
    }
    let normalized = normalize_for_compare(prompt);
    for token in normalized.split_whitespace() {
        let cleaned = clean_text(token, 64);
        if cleaned.len() < 4 {
            continue;
        }
        if matches!(
            cleaned.as_str(),
            "with"
                | "that"
                | "from"
                | "into"
                | "give"
                | "what"
                | "when"
                | "where"
                | "which"
                | "would"
                | "about"
                | "using"
                | "research"
                | "compare"
                | "practical"
                | "tradeoffs"
        ) {
            continue;
        }
        if !out
            .iter()
            .any(|current| normalize_for_compare(current) == cleaned)
        {
            out.push(cleaned);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

pub(super) fn load_request_pack_index(path: &str) -> BTreeMap<String, Value> {
    let report = read_json(path);
    report
        .get("cases")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let case_id = str_at(row, &["case_id"], "");
                    if case_id.is_empty() {
                        return None;
                    }
                    let request = row
                        .pointer("/response_diagnostics/pending_tool_request")
                        .or_else(|| {
                            row.pointer(
                                "/turn_sequence/initial_response_diagnostics/pending_tool_request",
                            )
                        })
                        .cloned()?;
                    Some((case_id, request))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}
