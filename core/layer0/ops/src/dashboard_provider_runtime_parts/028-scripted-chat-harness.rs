#[cfg(test)]
fn live_web_tooling_smoke_enabled() -> bool {
    matches!(
        std::env::var("INFRING_LIVE_WEB_TOOLING_SMOKE")
            .ok()
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(ref value) if value == "1" || value == "true" || value == "yes"
    )
}

#[cfg(test)]
fn scripted_chat_harness_path(root: &Path) -> PathBuf {
    provider_state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_chat_script.json",
    )
}

#[cfg(test)]
fn first_sentence_for_test(raw: &str, max_len: usize) -> String {
    let cleaned = clean_text(raw, max_len.max(1));
    if cleaned.is_empty() {
        return cleaned;
    }
    let boundary = ['.', '!', '?', '\n']
        .iter()
        .filter_map(|marker| cleaned.find(*marker))
        .min()
        .unwrap_or(cleaned.len());
    clean_text(&cleaned[..boundary], max_len.max(1))
}

#[cfg(test)]
fn extract_json_block_after(marker: &str, text: &str) -> Option<Value> {
    let start = text.find(marker)?;
    let json_text = clean_text(&text[start + marker.len()..], 20_000);
    serde_json::from_str::<Value>(&json_text).ok()
}

#[cfg(test)]
fn scripted_compare_prompt(lowered: &str) -> bool {
    lowered.contains("compare openclaw to this system/workspace")
        || lowered.contains("compare this system to openclaw")
}

#[cfg(test)]
fn scripted_batch_query_call(query: &str) -> String {
    format!(
        "<function=batch_query>{{\"source\":\"web\",\"query\":\"{}\",\"aperture\":\"medium\"}}</function>",
        serde_json::to_string(query)
            .unwrap_or_else(|_| "\"\"".to_string())
            .trim_matches('"')
    )
}

#[cfg(test)]
fn scripted_workspace_analyze_call(query: &str) -> String {
    format!(
        "<function=workspace_analyze>{{\"path\":\".\",\"query\":\"{}\",\"full\":true}}</function>",
        serde_json::to_string(query)
            .unwrap_or_else(|_| "\"\"".to_string())
            .trim_matches('"')
    )
}

#[cfg(test)]
fn tool_row_has_successful_findings(row: &Value) -> bool {
    if row.get("is_error").and_then(Value::as_bool).unwrap_or(false)
        || row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
    {
        return false;
    }
    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    if matches!(
        status.as_str(),
        "timeout" | "blocked" | "policy_denied" | "error" | "failed" | "execution_error"
    ) {
        return false;
    }
    let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
    !result.is_empty()
        && !scripted_response_is_no_findings_placeholder(&result)
        && !scripted_response_looks_like_tool_ack_without_findings(&result)
        && !scripted_response_looks_like_unsynthesized_web_snippet_dump(&result)
}

#[cfg(test)]
fn tool_row_is_low_signal(row: &Value) -> bool {
    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
    matches!(status.as_str(), "no_results" | "low_signal" | "partial_no_results")
        || scripted_response_is_no_findings_placeholder(&result)
        || result.to_ascii_lowercase().contains("low-signal")
        || result
            .to_ascii_lowercase()
            .contains("no usable findings were extracted")
        || result
            .to_ascii_lowercase()
            .contains("insufficient source coverage")
        || result
            .to_ascii_lowercase()
            .contains("requires workspace analysis")
}

#[cfg(test)]
fn scripted_response_is_no_findings_placeholder(text: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    lowered.contains("don't have usable tool findings from this turn yet")
        || lowered.contains("low-signal or no-result output")
        || lowered.contains("search returned no useful information")
        || lowered.contains("search returned no useful comparison findings")
}

#[cfg(test)]
fn scripted_response_looks_like_tool_ack_without_findings(text: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    lowered.starts_with("batch query:")
        || lowered.starts_with("key findings:")
        || lowered.starts_with("completed tool steps:")
}

#[cfg(test)]
fn scripted_response_looks_like_unsynthesized_web_snippet_dump(text: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    lowered.contains("from web retrieval:")
        || lowered.contains("bing.com:")
        || lowered.contains("potential sources:")
}

#[cfg(test)]
fn synthesize_test_response_from_tool_rows(user_message: &str) -> Option<String> {
    let tool_rows = extract_json_block_after("Recorded tool outcomes:\n", user_message)?;
    let rows = tool_rows.as_array()?;

    let successful = rows
        .iter()
        .filter(|row| tool_row_has_successful_findings(row))
        .take(2)
        .collect::<Vec<_>>();
    if !successful.is_empty() {
        let evidence = successful
            .iter()
            .map(|row| {
                let tool_name =
                    clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
                let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
                format!(
                    "{}: {}",
                    if tool_name.is_empty() { "tool" } else { &tool_name },
                    first_sentence_for_test(&result, 240)
                )
            })
            .collect::<Vec<_>>();
        let label = successful
            .iter()
            .map(|row| clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 60))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
            .join(" + ");
        return Some(format!(
            "Using the {} results, here is the answer: {}",
            if label.is_empty() {
                "recorded tool"
            } else {
                &label
            },
            clean_text(&evidence.join(" "), 480)
        ));
    }

    if let Some(row) = rows.iter().find(|row| tool_row_is_low_signal(row)) {
        let tool_name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        let lowered = result.to_ascii_lowercase();
        let guidance = if result.is_empty() {
            "Retry with a narrower query or a specific source URL.".to_string()
        } else {
            clean_text(&first_sentence_for_test(&result, 220), 240)
        };
        if lowered.contains("workspace analysis") || lowered.contains("local_subject_requires_workspace_analysis") {
            return Some(
                "Web retrieval alone was not enough for that comparison. I need local workspace evidence plus external web findings to answer it well."
                    .to_string(),
            );
        }
        return Some(format!(
            "The {} step ran, but it came back low-signal/no-results, so I cannot give a source-backed answer from this run. {}",
            if tool_name.is_empty() { "tool" } else { &tool_name },
            guidance
        ));
    }

    let failed = rows.iter().find(|row| {
        row.get("is_error").and_then(Value::as_bool).unwrap_or(false)
            || row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
    });
    failed.map(|row| {
        let tool_name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        if result.is_empty() {
            format!("The {tool_name} step failed before I could finish the answer.")
        } else {
            format!(
                "The {tool_name} step failed before I could finish the answer: {}",
                first_sentence_for_test(&result, 260)
            )
        }
    })
}

#[cfg(test)]
fn infer_test_inline_tool_response(user_message: &str) -> Option<String> {
    let cleaned = clean_text(user_message, 80_000);
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.is_empty() {
        return None;
    }
    if lowered.contains("write the final assistant response now.")
        || lowered.contains("write the final assistant reply now.")
    {
        return synthesize_test_response_from_tool_rows(&cleaned);
    }
    if lowered.contains("run `infring web search` as the next safe step") {
        return Some(
            "`infring web search` needs a query before it can run. Ask me to web search for a specific topic, for example `try to web search \"top AI agent frameworks\"`."
                .to_string(),
        );
    }
    if let Some(rest) = lowered.strip_prefix("tool::web_search:::") {
        let query = clean_text(rest, 600);
        if !query.is_empty() {
            return Some(format!(
                "<function=web_search>{{\"query\":\"{}\",\"source\":\"web\",\"aperture\":\"medium\"}}</function>",
                serde_json::to_string(&query)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if let Some(rest) = lowered.strip_prefix("tool::compare:::") {
        let query = clean_text(rest, 600);
        if !query.is_empty() {
            return Some(scripted_batch_query_call(&query));
        }
    }
    if let Some(rest) = lowered.strip_prefix("tool::fetch:::") {
        let url = clean_text(rest, 2_000);
        if !url.is_empty() {
            return Some(format!(
                "<function=web_fetch>{{\"url\":\"{}\",\"summary_only\":true}}</function>",
                serde_json::to_string(&url)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if let Some(rest) = cleaned.strip_prefix("/file ") {
        let path = clean_text(rest, 2_000);
        if !path.is_empty() {
            return Some(format!(
                "<function=file_read>{{\"path\":\"{}\",\"full\":true}}</function>",
                serde_json::to_string(&path)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if let Some(rest) = cleaned.strip_prefix("/folder ") {
        let path = clean_text(rest, 2_000);
        if !path.is_empty() {
            return Some(format!(
                "<function=folder_export>{{\"path\":\"{}\",\"full\":true}}</function>",
                serde_json::to_string(&path)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if let Some(rest) = cleaned.strip_prefix("/browse ").or_else(|| cleaned.strip_prefix("/web ")) {
        let url = clean_text(rest, 2_000);
        if !url.is_empty() {
            return Some(format!(
                "<function=web_fetch>{{\"url\":\"{}\",\"summary_only\":true}}</function>",
                serde_json::to_string(&url)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if let Some(rest) = cleaned.strip_prefix("/search ").or_else(|| cleaned.strip_prefix("/batch ")) {
        let query = clean_text(rest, 600);
        if !query.is_empty() {
            return Some(scripted_batch_query_call(&query));
        }
    }
    if cleaned.starts_with("/capabilities") || cleaned.starts_with("/tools") {
        return Some("<function=tool_capabilities>{\"scope\":\"agent\"}</function>".to_string());
    }
    if let Some(rest) = cleaned.strip_prefix("/memory set ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let key = clean_text(parts.next().unwrap_or(""), 180);
        let value = clean_text(parts.next().unwrap_or(""), 800);
        if !key.is_empty() {
            return Some(format!(
                "<function=memory_kv_set>{{\"key\":\"{}\",\"value\":\"{}\",\"confirm\":true}}</function>",
                serde_json::to_string(&key)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"'),
                serde_json::to_string(&value)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if lowered.contains("read file ") {
        let path = clean_text(
            cleaned
                .split_once("read file ")
                .map(|(_, tail)| tail)
                .unwrap_or(""),
            2_000,
        );
        if !path.is_empty() {
            return Some(format!(
                "<function=file_read>{{\"path\":\"{}\",\"full\":true}}</function>",
                serde_json::to_string(&path)
                    .unwrap_or_else(|_| "\"\"".to_string())
                    .trim_matches('"')
            ));
        }
    }
    if scripted_compare_prompt(&lowered) {
        let query = clean_text(&cleaned, 600);
        if !query.is_empty() {
            return Some(format!(
                "{}\n{}",
                scripted_workspace_analyze_call(&query),
                scripted_batch_query_call(&query)
            ));
        }
    }
    if lowered.contains("search the web for") || lowered.contains("try to web search") {
        let query = clean_text(&cleaned, 600);
        if !query.is_empty() {
            return Some(scripted_batch_query_call(&query));
        }
    }
    if lowered.contains("what did we decide")
        || lowered.contains("remember")
        || lowered.contains("recall")
    {
        return Some(format!(
            "<function=memory_semantic_query>{{\"query\":\"{}\",\"limit\":8}}</function>",
            serde_json::to_string(&cleaned)
                .unwrap_or_else(|_| "\"\"".to_string())
                .trim_matches('"')
        ));
    }
    None
}
