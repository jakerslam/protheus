const WEB_TOOLING_ACTION_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";

fn is_invisible_unicode(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x206A..=0x206F
            | 0xFEFF
            | 0xE0000..=0xE007F
    )
}

fn strip_invisible_unicode(raw: &str) -> String {
    crate::contract_lane_utils::strip_invisible_unicode(raw)
}

fn parse_i64_loose(value: Option<&Value>) -> i64 {
    value
        .and_then(|row| {
            row.as_i64()
                .or_else(|| row.as_u64().map(|num| num as i64))
                .or_else(|| {
                    row.as_str()
                        .and_then(|text| clean_text(text, 40).parse::<i64>().ok())
                })
        })
        .unwrap_or(0)
}

fn normalize_web_tooling_error_code(raw: &str) -> String {
    crate::tool_output_match_filter::normalize_web_tooling_error_code(raw)
}

fn add_error_count(error_codes: &mut Map<String, Value>, code: &str, count: i64) {
    let normalized = clean_text(code, 120).to_ascii_lowercase();
    if normalized.is_empty() || count <= 0 {
        return;
    }
    let next = error_codes
        .get(&normalized)
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .saturating_add(count);
    error_codes.insert(normalized, Value::from(next));
}

pub(crate) fn sanitize_web_tooling_query(raw: &str) -> String {
    let stripped = strip_invisible_unicode(raw);
    clean_text(&stripped, 1200)
}

pub(crate) fn canonicalize_web_tooling_query(query: &str, profile: &Value) -> String {
    let sanitized = sanitize_web_tooling_query(query);
    if sanitized.is_empty() {
        return sanitized;
    }
    let mode = clean_text(
        profile
            .pointer("/query_policy/mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        40,
    )
    .to_ascii_lowercase();
    let prefer_official_docs =
        as_bool(profile.pointer("/query_policy/prefer_official_docs"), true);
    if sanitized.to_ascii_lowercase().contains("site:") {
        return sanitized;
    }
    if !matches!(mode.as_str(), "domain_first" | "balanced") {
        return sanitized;
    }
    if !prefer_official_docs && mode != "domain_first" {
        return sanitized;
    }
    let primary_domain = profile
        .get("allowed_domains")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 140).to_ascii_lowercase())
        .filter(|domain| !domain.is_empty());
    if let Some(domain) = primary_domain {
        return format!("site:{domain} {sanitized}");
    }
    sanitized
}

pub(crate) fn summarize_web_tooling_history(root: &Path, max_rows: usize) -> Value {
    let history_path = state_path(root, WEB_TOOLING_ACTION_HISTORY_REL);
    let raw = fs::read_to_string(&history_path).unwrap_or_default();
    let capped_rows = max_rows.clamp(20, 500);
    if raw.trim().is_empty() {
        return json!({
            "history_path": history_path.to_string_lossy().to_string(),
            "window_events": 0,
            "total_calls": 0,
            "search_calls": 0,
            "fetch_calls": 0,
            "successful_calls": 0,
            "failed_calls": 0,
            "no_result_calls": 0,
            "error_ratio": 0.0,
            "error_codes": {},
            "recent_errors": [],
            "last_error_code": Value::Null
        });
    }

    let mut total_calls = 0_i64;
    let mut search_calls = 0_i64;
    let mut fetch_calls = 0_i64;
    let mut successful_calls = 0_i64;
    let mut failed_calls = 0_i64;
    let mut no_result_calls = 0_i64;
    let mut error_codes = Map::<String, Value>::new();
    let mut recent_errors = Vec::<Value>::new();
    let mut event_count = 0_i64;

    for line in raw.lines().rev().take(capped_rows * 4) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = serde_json::from_str::<Value>(trimmed).unwrap_or(Value::Null);
        if parsed.is_null() {
            continue;
        }
        event_count += 1;
        let ts = clean_text(parsed.get("ts").and_then(Value::as_str).unwrap_or(""), 80);
        let diagnostics = parsed
            .pointer("/payload/response_finalization/tool_diagnostics")
            .or_else(|| parsed.pointer("/response_finalization/tool_diagnostics"))
            .or_else(|| parsed.pointer("/payload/tool_diagnostics"))
            .or_else(|| parsed.get("tool_diagnostics"));
        if let Some(diag) = diagnostics {
            total_calls = total_calls.saturating_add(parse_i64_loose(diag.get("total_calls")));
            search_calls = search_calls.saturating_add(parse_i64_loose(diag.get("search_calls")));
            fetch_calls = fetch_calls.saturating_add(parse_i64_loose(diag.get("fetch_calls")));
            successful_calls =
                successful_calls.saturating_add(parse_i64_loose(diag.get("successful_calls")));
            failed_calls = failed_calls.saturating_add(parse_i64_loose(diag.get("failed_calls")));
            no_result_calls =
                no_result_calls.saturating_add(parse_i64_loose(diag.get("no_result_calls")));
            if let Some(codes) = diag.get("error_codes").and_then(Value::as_object) {
                for (code, count) in codes {
                    add_error_count(&mut error_codes, code, parse_i64_loose(Some(count)).max(1));
                    if recent_errors.len() < capped_rows {
                        recent_errors.push(json!({
                            "ts": ts,
                            "error_code": clean_text(code, 120),
                            "message": "tool_diagnostics"
                        }));
                    }
                }
            }
        }
        let direct_error = clean_text(
            parsed
                .pointer("/payload/error")
                .or_else(|| parsed.pointer("/payload/response/error"))
                .or_else(|| parsed.get("error"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            260,
        );
        if !direct_error.is_empty() {
            let code = normalize_web_tooling_error_code(&direct_error);
            add_error_count(&mut error_codes, &code, 1);
            failed_calls = failed_calls.saturating_add(1);
            if recent_errors.len() < capped_rows {
                recent_errors.push(json!({
                    "ts": ts,
                    "error_code": code,
                    "message": direct_error
                }));
            }
        }
    }

    let last_error_code = recent_errors
        .first()
        .and_then(|row| row.get("error_code").and_then(Value::as_str))
        .map(|value| Value::String(clean_text(value, 120)))
        .unwrap_or(Value::Null);
    let error_ratio = if total_calls > 0 {
        (failed_calls as f64) / (total_calls as f64)
    } else {
        0.0
    };

    json!({
        "history_path": history_path.to_string_lossy().to_string(),
        "window_events": event_count,
        "total_calls": total_calls,
        "search_calls": search_calls,
        "fetch_calls": fetch_calls,
        "successful_calls": successful_calls,
        "failed_calls": failed_calls,
        "no_result_calls": no_result_calls,
        "error_ratio": error_ratio,
        "error_codes": Value::Object(error_codes),
        "recent_errors": recent_errors,
        "last_error_code": last_error_code
    })
}
