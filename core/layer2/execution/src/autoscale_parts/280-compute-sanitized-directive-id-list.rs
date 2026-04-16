fn normalize_directive_id(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut previous_was_space = false;
    for ch in raw.chars() {
        if ch.is_control() {
            continue;
        }
        if ch.is_whitespace() {
            if !previous_was_space {
                out.push(' ');
                previous_was_space = true;
            }
            continue;
        }
        previous_was_space = false;
        out.push(ch);
        if out.len() >= 120 {
            break;
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn compute_sanitized_directive_id_list(
    input: &SanitizedDirectiveIdListInput,
) -> SanitizedDirectiveIdListOutput {
    let limit = input
        .limit
        .filter(|v| v.is_finite())
        .map(|v| v.max(0.0).floor() as usize)
        .unwrap_or(12usize)
        .min(200usize);
    if limit == 0 {
        return SanitizedDirectiveIdListOutput { ids: Vec::new() };
    }
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for row in input.rows.iter() {
        if out.len() >= limit {
            break;
        }
        let sanitized_raw =
            compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
                value: Some(row.clone()),
            })
            .objective_id;
        let Some(sanitized) = normalize_directive_id(&sanitized_raw) else {
            continue;
        };
        let dedupe_key = sanitized.to_ascii_lowercase();
        if !seen.insert(dedupe_key) {
            continue;
        }
        out.push(sanitized);
    }
    SanitizedDirectiveIdListOutput { ids: out }
}

pub fn compute_parse_first_json_line(input: &ParseFirstJsonLineInput) -> ParseFirstJsonLineOutput {
    let raw = input.text.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return ParseFirstJsonLineOutput { value: None };
    }
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        return ParseFirstJsonLineOutput {
            value: Some(parsed),
        };
    }
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return ParseFirstJsonLineOutput {
                value: Some(parsed),
            };
        }
    }
    ParseFirstJsonLineOutput { value: None }
}

pub fn compute_parse_json_objects_from_text(
    input: &ParseJsonObjectsFromTextInput,
) -> ParseJsonObjectsFromTextOutput {
    let text = input.text.as_deref().unwrap_or("");
    let max_objects = input
        .max_objects
        .filter(|v| v.is_finite())
        .map(|v| v.max(0.0).floor() as usize)
        .unwrap_or(40usize)
        .min(500usize);
    if max_objects == 0 {
        return ParseJsonObjectsFromTextOutput {
            objects: Vec::new(),
        };
    }
    let mut out = Vec::<serde_json::Value>::new();
    for line in text.lines() {
        if out.len() >= max_objects {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if parsed.is_object() {
                out.push(parsed);
            }
        }
    }
    ParseJsonObjectsFromTextOutput { objects: out }
}

pub fn compute_read_path_value(input: &ReadPathValueInput) -> ReadPathValueOutput {
    let Some(mut cur) = input.obj.as_ref() else {
        return ReadPathValueOutput { value: None };
    };
    let parts: Vec<&str> = input
        .path_expr
        .as_deref()
        .unwrap_or("")
        .split('.')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return ReadPathValueOutput { value: None };
    }
    for key in parts {
        let Some(map) = cur.as_object() else {
            return ReadPathValueOutput { value: None };
        };
        let Some(next) = map.get(key) else {
            return ReadPathValueOutput { value: None };
        };
        cur = next;
    }
    ReadPathValueOutput {
        value: Some(cur.clone()),
    }
}

pub fn compute_number_or_null(input: &NumberOrNullInput) -> NumberOrNullOutput {
    let value = input.value.filter(|v| v.is_finite() && *v >= 0.0);
    NumberOrNullOutput { value }
}

pub fn compute_choose_evidence_selection_mode(
    input: &ChooseEvidenceSelectionModeInput,
) -> ChooseEvidenceSelectionModeOutput {
    let eligible_len = input
        .eligible_len
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
        .max(0.0)
        .floor() as u32;
    let sample_window_raw = input
        .evidence_sample_window
        .filter(|v| v.is_finite())
        .unwrap_or(1.0)
        .max(1.0)
        .floor() as u32;
    let window = std::cmp::max(
        1u32,
        std::cmp::min(eligible_len.max(1u32), sample_window_raw),
    );
    let prior_evidence_attempts = input
        .prior_runs
        .iter()
        .filter(|e| {
            e.event_type
                .as_deref()
                .unwrap_or("")
                .trim()
                .eq("autonomy_run")
                && matches!(
                    e.result.as_deref().unwrap_or("").trim(),
                    "score_only_preview" | "score_only_evidence"
                )
        })
        .count() as u32;
    let cursor = if window > 0 {
        prior_evidence_attempts % window
    } else {
        0
    };
    let prefix = input
        .mode_prefix
        .as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or("evidence");
    ChooseEvidenceSelectionModeOutput {
        mode: format!("{prefix}_sample"),
        index: cursor,
        sample_window: window,
        sample_cursor: cursor,
        prior_evidence_attempts,
    }
}

pub fn compute_truthy_flag(input: &TruthyFlagInput) -> TruthyFlagOutput {
    let value = match input.value.as_ref() {
        Some(serde_json::Value::Bool(v)) => *v,
        Some(serde_json::Value::Null) | None => false,
        Some(other) => {
            let text = match other {
                serde_json::Value::String(s) => s.clone(),
                _ => other.to_string(),
            };
            let normalized = text.trim().to_ascii_lowercase();
            normalized == "true" || normalized == "1" || normalized == "yes"
        }
    };
    TruthyFlagOutput { value }
}

pub fn compute_falsey_flag(input: &TruthyFlagInput) -> TruthyFlagOutput {
    let value = match input.value.as_ref() {
        Some(serde_json::Value::Bool(v)) => !*v,
        Some(serde_json::Value::Null) | None => false,
        Some(other) => {
            let text = match other {
                serde_json::Value::String(s) => s.clone(),
                _ => other.to_string(),
            };
            let normalized = text.trim().to_ascii_lowercase();
            normalized == "false" || normalized == "0" || normalized == "no"
        }
    };
    TruthyFlagOutput { value }
}

pub fn compute_stable_selection_index(
    input: &StableSelectionIndexInput,
) -> StableSelectionIndexOutput {
    let n = input
        .size
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
        .max(0.0)
        .floor() as u64;
    if n == 0 {
        return StableSelectionIndexOutput { index: 0 };
    }
    let seed = input.seed.as_deref().unwrap_or("");
    let hex = format!("{:x}", Sha256::digest(seed.as_bytes()));
    let slice = &hex[..std::cmp::min(12, hex.len())];
    let num = u64::from_str_radix(slice, 16).unwrap_or(0);
    StableSelectionIndexOutput {
        index: (num % n) as u32,
    }
}

pub fn compute_as_string_array(input: &AsStringArrayInput) -> AsStringArrayOutput {
    let mut out = Vec::<String>::new();
    match input.value.as_ref() {
        Some(serde_json::Value::Array(rows)) => {
            for row in rows {
                let value = match row {
                    serde_json::Value::String(s) => s.trim().to_string(),
                    serde_json::Value::Null => String::new(),
                    _ => row.to_string().trim().to_string(),
                };
                if !value.is_empty() {
                    out.push(value);
                }
            }
        }
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
        _ => {}
    }
    AsStringArrayOutput { values: out }
}
pub fn compute_uniq_sorted(input: &UniqSortedInput) -> UniqSortedOutput {
    let mut seen = std::collections::BTreeSet::<String>::new();
    for row in input.values.iter() {
        seen.insert(row.clone());
    }
    UniqSortedOutput {
        values: seen.into_iter().collect(),
    }
}

pub fn compute_normalize_model_ids(input: &NormalizeModelIdsInput) -> NormalizeModelIdsOutput {
    let limit = input
        .limit
        .filter(|v| v.is_finite())
        .map(|v| v.max(0.0).floor() as usize)
        .unwrap_or(128usize)
        .min(2000usize);
    if limit == 0 {
        return NormalizeModelIdsOutput { models: Vec::new() };
    }
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for raw in input.models.iter() {
        let value = raw.trim().to_string();
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        out.push(value);
        if out.len() >= limit {
            break;
        }
    }
    NormalizeModelIdsOutput { models: out }
}

pub fn compute_selected_model_from_run_event(
    input: &SelectedModelFromRunEventInput,
) -> SelectedModelFromRunEventOutput {
    let Some(summary) = input.route_summary.as_ref().and_then(|v| v.as_object()) else {
        return SelectedModelFromRunEventOutput { model: None };
    };
    let keys = ["selected_model", "model", "selectedModel", "chosen_model"];
    for key in keys {
        let Some(v) = summary.get(key) else {
            continue;
        };
        let text = match v {
            serde_json::Value::String(s) => s.trim().to_string(),
            _ => v.to_string().trim().to_string(),
        };
        if !text.is_empty() {
            return SelectedModelFromRunEventOutput { model: Some(text) };
        }
    }
    SelectedModelFromRunEventOutput { model: None }
}

pub fn compute_read_first_numeric_metric(
    input: &ReadFirstNumericMetricInput,
) -> ReadFirstNumericMetricOutput {
    let to_non_negative = |value: Option<&serde_json::Value>| -> Option<f64> {
        let number = match value {
            None | Some(serde_json::Value::Null) => Some(0.0),
            Some(serde_json::Value::Number(n)) => n.as_f64(),
            Some(serde_json::Value::Bool(v)) => Some(if *v { 1.0 } else { 0.0 }),
            Some(serde_json::Value::String(s)) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    Some(0.0)
                } else {
                    trimmed.parse::<f64>().ok()
                }
            }
            _ => None,
        };
        number.filter(|v| v.is_finite() && *v >= 0.0)
    };
    for expr in input.path_exprs.iter() {
        for src in input.sources.iter() {
            let read = compute_read_path_value(&ReadPathValueInput {
                obj: Some(src.clone()),
                path_expr: Some(expr.clone()),
            });
            let n = to_non_negative(read.value.as_ref());
            if n.is_some() {
                return ReadFirstNumericMetricOutput { value: n };
            }
        }
    }
    ReadFirstNumericMetricOutput { value: None }
}

pub fn compute_parse_arg(input: &ParseArgInput) -> ParseArgOutput {
    let name = input.name.as_deref().unwrap_or("").trim();
    if name.is_empty() {
        return ParseArgOutput { value: None };
    }
    let pref = format!("--{}=", name);
    for arg in input.args.iter() {
        if arg.starts_with(&pref) {
            return ParseArgOutput {
                value: Some(arg[pref.len()..].to_string()),
            };
        }
    }
    ParseArgOutput { value: None }
}

pub fn compute_date_arg_or_today(input: &DateArgOrTodayInput) -> DateArgOrTodayOutput {
    let candidate = input.value.as_deref().unwrap_or("").trim();
    let looks_like_date = Regex::new(r"^\d{4}-\d{2}-\d{2}$")
        .expect("valid date arg regex")
        .is_match(candidate);
    if looks_like_date {
        return DateArgOrTodayOutput {
            date: candidate.to_string(),
        };
    }
    DateArgOrTodayOutput {
        date: input
            .today
            .as_deref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string()),
    }
}

pub fn compute_has_env_numeric_override(
    input: &HasEnvNumericOverrideInput,
) -> HasEnvNumericOverrideOutput {
    let non_empty = input
        .raw_value
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    HasEnvNumericOverrideOutput {
        has_override: input.present && non_empty,
    }
}

pub fn compute_coalesce_numeric(input: &CoalesceNumericInput) -> CoalesceNumericOutput {
    let primary = input.primary.filter(|v| v.is_finite());
    if primary.is_some() {
        return CoalesceNumericOutput { value: primary };
    }
    let fallback = input.fallback.filter(|v| v.is_finite());
    if fallback.is_some() {
        return CoalesceNumericOutput { value: fallback };
    }
    CoalesceNumericOutput {
        value: input.null_fallback.filter(|v| v.is_finite()),
    }
}

pub fn compute_clamp_number(input: &ClampNumberInput) -> ClampNumberOutput {
    let min = input.min.filter(|v| v.is_finite()).unwrap_or(0.0);
    let max = input.max.filter(|v| v.is_finite()).unwrap_or(min);
    let value = input.value.filter(|v| v.is_finite()).unwrap_or(min);
    ClampNumberOutput {
        value: if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        },
    }
}

pub fn compute_list_proposal_files(input: &ListProposalFilesInput) -> ListProposalFilesOutput {
    let mut files = input
        .entries
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| {
            Regex::new(r"^\d{4}-\d{2}-\d{2}\.json$")
                .expect("valid proposal filename regex")
                .is_match(v)
        })
        .collect::<Vec<String>>();
    files.sort();
    ListProposalFilesOutput { files }
}

pub fn compute_latest_proposal_date(input: &LatestProposalDateInput) -> LatestProposalDateOutput {
    let max_date = input.max_date.as_deref().unwrap_or("").trim();
    let mut dates = input
        .files
        .iter()
        .map(|f| f.trim().trim_end_matches(".json").to_string())
        .filter(|d| {
            !d.is_empty()
                && Regex::new(r"^\d{4}-\d{2}-\d{2}$")
                    .expect("valid ymd regex")
                    .is_match(d)
        })
        .filter(|d| max_date.is_empty() || d.as_str() <= max_date)
        .collect::<Vec<String>>();
    dates.sort();
    LatestProposalDateOutput { date: dates.pop() }
}
