fn workflow_has_manual_toolbox_candidate_menu(workflow: &Value) -> bool {
    workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        == Some("first_gate_pending_llm_tool_choice")
        || workflow
            .get("system_events")
            .and_then(Value::as_array)
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("kind")
                        .or_else(|| event.get("name"))
                        .or_else(|| event.get("type"))
                        .and_then(Value::as_str)
                        == Some("manual_toolbox_candidate_menu")
                })
            })
            .unwrap_or(false)
}

fn record_manual_toolbox_pending_request(workflow: &mut Value, response_text: &str, message: &str) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    let pending_request = manual_toolbox_pending_request_from_response(response_text, message);
    let Some(pending_request) = pending_request else {
        return;
    };
    record_manual_toolbox_pending_request_value(workflow, pending_request);
}

fn record_manual_toolbox_pending_request_value(workflow: &mut Value, mut pending_request: Value) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    if let Some((tool_name, input)) = pending_request
        .get("tool_name")
        .and_then(Value::as_str)
        .zip(pending_request.get("input").cloned())
    {
        if let Ok(repaired_input) =
            crate::infring_tooling_core_v1_bridge::repair_and_validate_args(tool_name, &input)
        {
            pending_request["input"] = repaired_input;
        }
    }
    workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
    workflow["response"] = Value::String(String::new());
    workflow["visible_response_source"] = Value::String("json_private_tool_request".to_string());
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_pending_tool_confirmation".to_string());
    if let Some(events) = workflow
        .get_mut("system_events")
        .and_then(Value::as_array_mut)
    {
        events.push(turn_workflow_event(
            "manual_toolbox_pending_tool_request",
            pending_request,
        ));
    }
}

fn manual_toolbox_pending_request_from_latent_candidates(
    latent_tool_candidates: &Value,
    message: &str,
) -> Option<Value> {
    let candidates = latent_tool_candidates.as_array()?;
    let valid = candidates
        .iter()
        .filter_map(|candidate| {
            let workflow_only = candidate
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !workflow_only {
                return None;
            }
            let family_key = clean_text(
                candidate
                    .get("selected_tool_family")
                    .or_else(|| candidate.get("tool_family"))
                    .or_else(|| candidate.get("family"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let tool_key = clean_text(
                candidate
                    .get("selected_tool_key")
                    .or_else(|| candidate.get("tool_key"))
                    .or_else(|| candidate.get("tool_name"))
                    .or_else(|| candidate.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let tool_label = clean_text(
                candidate
                    .get("selected_tool_label")
                    .or_else(|| candidate.get("label"))
                    .or_else(|| candidate.get("tool_label"))
                    .and_then(Value::as_str)
                    .unwrap_or(&tool_key),
                120,
            );
            let input = candidate
                .get("input")
                .or_else(|| candidate.get("request_payload"))
                .or_else(|| candidate.get("proposed_input"))
                .cloned()
                .filter(Value::is_object)?;
            manual_toolbox_pending_request_from_parts(
                &family_key,
                &tool_key,
                &tool_label,
                input,
                message,
            )
            .map(|mut pending| {
                pending["source"] = Value::String("latent_candidate_recovery".to_string());
                pending["recovery_contract"] = Value::String(
                    "single_valid_workflow_only_candidate_after_private_gate_failure_or_terminal_invariant_recovery".to_string(),
                );
                pending
            })
        })
        .collect::<Vec<_>>();
    if valid.len() == 1 {
        valid.into_iter().next()
    } else {
        None
    }
}

fn workflow_repair_recovered_request_payload(
    family_key: &str,
    tool_key: &str,
    mut input: Value,
    message: &str,
) -> Value {
    if canonical_manual_toolbox_tool_name(family_key, tool_key) != "batch_query" {
        return input;
    }
    let Some(policy) = workflow_batch_query_recovery_repair_policy(family_key) else {
        return input;
    };
    let Some(map) = input.as_object_mut() else {
        return input;
    };
    let query = map
        .get("query")
        .or_else(|| map.get("context"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 1_200))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| clean_text(message, 1_200));
    if query.is_empty() {
        return input;
    }

    let source_class_terms = workflow_recovery_policy_terms(&policy, "source_class_terms", 6);
    let generic_facet_terms = workflow_recovery_policy_terms(&policy, "generic_facet_terms", 8);
    let entities = workflow_recovery_entity_terms(&query, message);
    let facets = workflow_recovery_facet_terms(&query, message, &generic_facet_terms);
    let broad = workflow_recovery_request_needs_metadata(&query, message, &entities);
    let mut added_fields = Vec::<String>::new();

    if broad {
        if workflow_json_array_is_empty(map.get("queries")) {
            let queries = workflow_recovery_query_lanes(&query, &entities, &facets, &source_class_terms);
            if !queries.is_empty() {
                map.insert("queries".to_string(), json!(queries));
                added_fields.push("queries".to_string());
            }
        }
        if workflow_json_array_is_empty(map.get("keywords")) {
            let keywords = workflow_recovery_keywords(&entities, &facets, &source_class_terms);
            if !keywords.is_empty() {
                map.insert("keywords".to_string(), json!(keywords));
                added_fields.push("keywords".to_string());
            }
        }
        let required_coverage = workflow_recovery_required_coverage(map.get("required_coverage"), &entities, &facets);
        if required_coverage.is_some() {
            map.insert(
                "required_coverage".to_string(),
                required_coverage.unwrap_or_else(|| json!({})),
            );
            added_fields.push("required_coverage".to_string());
        }
        if !map.contains_key("aliases") {
            map.insert("aliases".to_string(), json!([]));
            added_fields.push("aliases".to_string());
        }
        if !map.contains_key("negative_terms") {
            map.insert("negative_terms".to_string(), json!([]));
            added_fields.push("negative_terms".to_string());
        }
    }

    let marker_field = policy
        .get("narrow_lookup_marker_field")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 80))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "query_metadata_policy".to_string());
    if !map.contains_key(&marker_field) {
        map.insert(
            marker_field,
            json!({
                "source": "latent_candidate_recovery_contract",
                "classification": if broad {
                    "expanded_query_pack"
                } else {
                    "narrow_lookup_or_initial_discovery"
                },
                "reason": if broad {
                    "request_shape_needed_multiple_evidence_lanes"
                } else {
                    "request_shape_did_not_require_extra_metadata"
                },
                "metadata_fields_added": added_fields
            }),
        );
    }
    input
}

fn workflow_batch_query_recovery_repair_policy(family_key: &str) -> Option<Value> {
    let contract = workflow_tool_menu_contract_for_family(family_key);
    let policy = contract.pointer(
        "/latent_candidate_recovery_contract/request_contract_repair/batch_query",
    )?;
    if policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Some(policy.clone())
    } else {
        None
    }
}

fn workflow_recovery_policy_terms(policy: &Value, field: &str, limit: usize) -> Vec<String> {
    policy
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 80))
                .filter(|raw| !raw.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_json_array_is_empty(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
}

fn workflow_recovery_request_needs_metadata(
    query: &str,
    message: &str,
    entities: &[String],
) -> bool {
    let lowered = clean_text(&format!("{query} {message}"), 2_400).to_ascii_lowercase();
    entities.len() > 1
        || [
            "research",
            "compare",
            "comparison",
            " versus ",
            " vs ",
            "rank",
            "ranking",
            "evaluate",
            "assessment",
            "assess",
            "recommend",
            "selection",
            "landscape",
            "benchmark",
            "benchmarks",
            "current",
            "currently",
            "recent",
            "latest",
            "right now",
            "as of",
            "maturity",
            "risk",
            "security",
            "production",
            "strength",
            "weak",
            "tradeoff",
            "tradeoffs",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn workflow_recovery_entity_terms(query: &str, message: &str) -> Vec<String> {
    let raw = clean_text(&format!("{query} {message}"), 2_400);
    let mut out = Vec::<String>::new();
    let mut current = Vec::<String>::new();
    for raw_token in raw.split_whitespace() {
        let entity_boundary = workflow_recovery_entity_boundary_after(raw_token);
        let token = workflow_recovery_clean_token(raw_token);
        if token.is_empty() {
            workflow_recovery_flush_entity_phrase(&mut out, &mut current);
            continue;
        }
        if workflow_recovery_token_looks_like_entity(&token) {
            current.push(token);
            if entity_boundary {
                workflow_recovery_flush_entity_phrase(&mut out, &mut current);
            }
            continue;
        }
        workflow_recovery_flush_entity_phrase(&mut out, &mut current);
    }
    workflow_recovery_flush_entity_phrase(&mut out, &mut current);
    workflow_recovery_dedupe_limit(out, 8)
}

fn workflow_recovery_clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_ascii_punctuation() && !matches!(ch, '-' | '_' | '/' | '+')
    })
    .chars()
    .filter(|ch| !ch.is_control())
    .collect::<String>()
}

fn workflow_recovery_entity_boundary_after(raw: &str) -> bool {
    raw.chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| matches!(ch, ',' | ';' | ':' | ')' | ']' | '}'))
        .unwrap_or(false)
}

fn workflow_recovery_token_looks_like_entity(token: &str) -> bool {
    if workflow_recovery_entity_stopword(token) {
        return false;
    }
    let letters = token.chars().filter(|ch| ch.is_alphabetic()).count();
    if letters < 2 {
        return false;
    }
    let uppercase = token.chars().filter(|ch| ch.is_uppercase()).count();
    token
        .chars()
        .next()
        .map(|ch| ch.is_uppercase())
        .unwrap_or(false)
        || uppercase >= 2
        || token.contains('.')
}

fn workflow_recovery_entity_stopword(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "as"
            | "april"
            | "august"
            | "best"
            | "current"
            | "december"
            | "for"
            | "from"
            | "give"
            | "i"
            | "if"
            | "in"
            | "january"
            | "july"
            | "june"
            | "march"
            | "may"
            | "me"
            | "november"
            | "october"
            | "of"
            | "on"
            | "or"
            | "practical"
            | "recommendation"
            | "research"
            | "search"
            | "september"
            | "tell"
            | "the"
            | "to"
            | "tradeoff"
            | "tradeoffs"
            | "use"
            | "what"
            | "which"
            | "with"
    )
}

fn workflow_recovery_flush_entity_phrase(out: &mut Vec<String>, current: &mut Vec<String>) {
    if current.is_empty() {
        return;
    }
    let phrase = clean_text(&current.join(" "), 120);
    current.clear();
    if !phrase.is_empty() {
        out.push(phrase);
    }
}

fn workflow_recovery_facet_terms(
    query: &str,
    message: &str,
    generic_facet_terms: &[String],
) -> Vec<String> {
    let lowered = clean_text(&format!("{query} {message}"), 2_400).to_ascii_lowercase();
    let mut out = Vec::<String>::new();
    for term in generic_facet_terms {
        if lowered.contains(&term.to_ascii_lowercase()) {
            out.push(term.clone());
        }
    }
    for raw_token in lowered.split(|ch: char| !ch.is_alphanumeric() && ch != '-') {
        let token = clean_text(raw_token, 80);
        if token.len() < 4 || workflow_recovery_facet_stopword(&token) {
            continue;
        }
        out.push(token);
    }
    workflow_recovery_dedupe_limit(out, 10)
}

fn workflow_recovery_facet_stopword(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "after"
            | "also"
            | "and"
            | "are"
            | "based"
            | "between"
            | "could"
            | "from"
            | "give"
            | "have"
            | "into"
            | "right"
            | "search"
            | "some"
            | "that"
            | "their"
            | "there"
            | "this"
            | "what"
            | "when"
            | "where"
            | "which"
            | "with"
            | "would"
    )
}

fn workflow_recovery_query_lanes(
    query: &str,
    entities: &[String],
    facets: &[String],
    source_class_terms: &[String],
) -> Vec<String> {
    let mut lanes = vec![clean_text(query, 400)];
    let facet_suffix = clean_text(&facets.iter().take(3).cloned().collect::<Vec<_>>().join(" "), 120);
    if !entities.is_empty() {
        for entity in entities.iter().take(5) {
            let suffix = if facet_suffix.is_empty() {
                source_class_terms
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "primary sources".to_string())
            } else {
                facet_suffix.clone()
            };
            lanes.push(clean_text(&format!("{entity} {suffix}"), 400));
        }
    } else {
        for source_term in source_class_terms.iter().take(3) {
            lanes.push(clean_text(&format!("{query} {source_term}"), 400));
        }
    }
    if let Some(source_term) = source_class_terms.get(1) {
        lanes.push(clean_text(&format!("{query} {source_term}"), 400));
    }
    workflow_recovery_dedupe_limit(lanes, 8)
}

fn workflow_recovery_keywords(
    entities: &[String],
    facets: &[String],
    source_class_terms: &[String],
) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    terms.extend(entities.iter().cloned());
    terms.extend(facets.iter().take(8).cloned());
    terms.extend(source_class_terms.iter().take(3).cloned());
    workflow_recovery_dedupe_limit(terms, 16)
}

fn workflow_recovery_required_coverage(
    existing: Option<&Value>,
    entities: &[String],
    facets: &[String],
) -> Option<Value> {
    let mut value = existing.cloned().unwrap_or_else(|| json!({}));
    let map = value.as_object_mut()?;
    if workflow_json_array_is_empty(map.get("entities")) && !entities.is_empty() {
        map.insert("entities".to_string(), json!(entities));
    }
    if workflow_json_array_is_empty(map.get("facets")) && !facets.is_empty() {
        map.insert("facets".to_string(), json!(facets));
    }
    Some(value)
}

fn workflow_recovery_dedupe_limit(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for value in values {
        let cleaned = clean_text(&value, 160);
        if cleaned.is_empty() {
            continue;
        }
        let normalized = cleaned.to_ascii_lowercase();
        if out
            .iter()
            .any(|existing| existing.to_ascii_lowercase() == normalized)
        {
            continue;
        }
        out.push(cleaned);
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn workflow_tool_family_prompt_context(
    previous_category_key: &str,
    previous_category_label: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let families = contract
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let family_menu_json = serde_json::to_string(&families).unwrap_or_else(|_| "[]".to_string());
    contract
        .get("llm_tool_family_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace(
                        "{previous_category_key}",
                        &clean_text(previous_category_key, 120),
                    )
                    .replace(
                        "{previous_category_label}",
                        &clean_text(previous_category_label, 120),
                    )
                    .replace("{tool_family_menu_json}", &family_menu_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_selection_prompt_context(family_key: &str, family_label: &str) -> String {
    let family_key = clean_text(family_key, 120);
    let family_label = clean_text(family_label, 120);
    let contract = workflow_tool_menu_contract_for_family(&family_key);
    let tools = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tools_json = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
    let allowed_tool_keys_json = tools
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(workflow_option_key)
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>()
        })
        .map(|keys| serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "[]".to_string());
    contract
        .get("llm_tool_selection_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_family_label}", &family_label)
                    .replace("{selected_tool_keys_json}", &allowed_tool_keys_json)
                    .replace("{selected_tool_menu_json}", &tools_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_payload_prompt_context(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
) -> String {
    let family_key = clean_text(family_key, 120);
    let tool_key = clean_text(tool_key, 120);
    let tool_label = clean_text(tool_label, 120);
    let contract = workflow_tool_menu_contract_for_family(&family_key);
    let tool = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .unwrap_or_else(|| json!({}));
    let request_format_json =
        serde_json::to_string(tool.get("request_format").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    let request_example_json =
        serde_json::to_string(tool.get("request_example").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    contract
        .get("llm_tool_payload_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_tool_key}", &tool_key)
                    .replace("{selected_tool_label}", &tool_label)
                    .replace("{selected_tool_request_format_json}", &request_format_json)
                    .replace(
                        "{selected_tool_request_example_json}",
                        &request_example_json,
                    ),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn manual_toolbox_private_gate_max_attempts() -> u64 {
    let contract = default_workflow_tool_menu_contract();
    let base_gate_count = contract
        .get("gate_order")
        .and_then(Value::as_array)
        .and_then(|gates| {
            gates
                .iter()
                .position(|gate| gate.as_str() == Some("gate_4_request_payload_input"))
                .map(|idx| idx as u64 + 1)
        })
        .unwrap_or(4);
    let retry_limit = contract
        .get("private_gate_retry_limit")
        .or_else(|| contract.get("workflow_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(4);
    base_gate_count.saturating_add(retry_limit).clamp(4, 8)
}

fn workflow_private_gate_retry_prompt_context(
    current_gate_id: &str,
    message: &str,
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let fallback = "INTERNAL RETRY — output is never shown to the user. The previous response for `{current_gate_id}` was rejected with reason `{last_reject_reason}`. Previous excerpt: {last_invalid_excerpt}. If the excerpt is empty, treat it as an empty response. Re-read the current gate system instruction and output only the exact JSON artifact required by that gate. Do not answer the user directly, do not write prose, and do not include markdown.";
    let template = contract
        .get("private_gate_retry_instruction")
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    let excerpt = if last_invalid_excerpt.trim().is_empty() {
        "(empty response)"
    } else {
        last_invalid_excerpt
    };
    clean_text(
        &format!(
            "{}\n\nContext-only user message. Do not answer it directly. Use it only to produce the artifact required for the current workflow gate:\n{}",
            template
                .replace("{current_gate_id}", &clean_text(current_gate_id, 120))
                .replace(
                    "{last_reject_reason}",
                    &clean_text(last_reject_reason, 160)
                )
                .replace("{last_invalid_excerpt}", &clean_text(excerpt, 320)),
            message
        ),
        8_000,
    )
}

fn workflow_tool_family_selection_from_response(response: &str) -> Option<(String, String)> {
    let contract = default_workflow_tool_menu_contract();
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool_family")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "family"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "tool_family_key")
                })
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_family")
                })
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "category"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "gate"))
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let family_key = workflow_family_key_for_selection(&contract, &token);
    if family_key.is_empty() {
        return None;
    }
    contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .and_then(|families| {
            families.iter().find_map(|family| {
                (workflow_option_key(family) == family_key)
                    .then(|| (family_key.clone(), workflow_option_label(family)))
            })
        })
}

fn workflow_tool_selection_from_response(
    family_key: &str,
    response: &str,
) -> Option<(String, String)> {
    let contract = workflow_tool_menu_contract_for_family(family_key);
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "selected_tool"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "tool_key"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_key")
                })
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let tool_key = workflow_tool_key_for_selection(&contract, family_key, &token);
    if tool_key.is_empty() {
        return None;
    }
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                (workflow_option_key(tool) == tool_key)
                    .then(|| (tool_key.clone(), workflow_option_label(tool)))
            })
        })
}

fn workflow_selected_tool_request_format_keys(family_key: &str, tool_key: &str) -> Vec<String> {
    workflow_tool_menu_contract_for_family(family_key)
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .and_then(|tool| tool.get("request_format").cloned())
        .and_then(|format| format.as_object().cloned())
        .map(|format| {
            format
                .keys()
                .map(|key| normalized_workflow_token(key))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_payload_object_matches_selected_tool(
    value: &Value,
    family_key: &str,
    tool_key: &str,
) -> bool {
    let Some(payload) = value.as_object() else {
        return false;
    };
    let expected_keys = workflow_selected_tool_request_format_keys(family_key, tool_key);
    if expected_keys.is_empty() {
        return false;
    }
    let reserved_keys = [
        "gate",
        "tool",
        "tool_name",
        "selected_tool",
        "selected_tool_name",
        "selected_tool_key",
        "tool_family",
        "selected_tool_family",
        "category",
        "final_answer",
        "message",
        "response",
        "content",
        "visible_response",
    ]
    .into_iter()
    .map(normalized_workflow_token)
    .collect::<Vec<_>>();
    let payload_keys = payload
        .keys()
        .map(|key| normalized_workflow_token(key))
        .collect::<Vec<_>>();
    !payload_keys
        .iter()
        .any(|key| reserved_keys.iter().any(|reserved| reserved == key))
        && expected_keys
            .iter()
            .any(|expected| payload_keys.iter().any(|key| key == expected))
}

fn workflow_request_payload_from_json_response(
    request: &Value,
    family_key: &str,
    tool_key: &str,
) -> Option<Value> {
    workflow_tool_request_object_field(
        request,
        &default_workflow_tool_menu_contract(),
        "request_payload",
    )
    .and_then(|value| workflow_tool_request_payload_from_json_value(&value))
    .or_else(|| {
        workflow_payload_object_matches_selected_tool(request, family_key, tool_key)
            .then(|| request.clone())
    })
}

fn workflow_request_payload_from_response(
    family_key: &str,
    tool_key: &str,
    response: &str,
) -> Option<Value> {
    workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_request_payload_from_json_response(&request, family_key, tool_key)
        })
        .or_else(|| {
            manual_toolbox_payload_json(response).and_then(|request| {
                workflow_request_payload_from_json_response(&request, family_key, tool_key)
            })
        })
        .filter(Value::is_object)
}

fn manual_toolbox_pending_request_from_parts(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
    input: Value,
    message: &str,
) -> Option<Value> {
    let tool_name = canonical_manual_toolbox_tool_name(family_key, tool_key);
    if tool_name.is_empty() || !input.is_object() {
        return None;
    }
    let input = workflow_repair_recovered_request_payload(family_key, tool_key, input, message);
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600)
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "split_manual_toolbox_gates",
        "tool_name": tool_name,
        "tool_key": clean_text(tool_key, 120),
        "selected_tool_key": clean_text(tool_key, 120),
        "selected_tool_family": clean_text(family_key, 120),
        "selected_tool_label": clean_text(tool_label, 120),
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn manual_toolbox_active_gate_id(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "gate_1_work_category_menu"
    } else if family_key.is_empty() {
        "gate_2_tool_family_menu"
    } else if tool_key.is_empty() {
        "gate_3_tool_menu"
    } else {
        "gate_4_request_payload_input"
    }
}

fn manual_toolbox_pending_direct_response_path(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_llm_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_llm_tool_family"
    } else if tool_key.is_empty() {
        "gate_3_pending_llm_tool_choice"
    } else {
        "gate_4_pending_llm_tool_request"
    }
}

fn manual_toolbox_pending_stage_status(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_tool_family_selection"
    } else if tool_key.is_empty() {
        "gate_3_pending_tool_selection"
    } else {
        "gate_4_pending_request_payload"
    }
}

fn manual_toolbox_invalid_reject_reason(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "tool_category_without_selection_diagnostic_only"
    } else if family_key.is_empty() {
        "tool_family_without_selection_diagnostic_only"
    } else if tool_key.is_empty() {
        "tool_without_selection_diagnostic_only"
    } else {
        "tool_request_without_payload_submission_diagnostic_only"
    }
}
