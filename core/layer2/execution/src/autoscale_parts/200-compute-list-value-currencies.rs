fn canonical_value_currency_alias(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "money" | "cash" | "profit" | "pricing" | "billing" => "revenue".to_string(),
        "customer" | "user" | "customer_value" | "retention" => "user_value".to_string(),
        "reliability" | "stability" | "resilience" => "quality".to_string(),
        "time" | "speed" | "latency" | "hours_saved" | "time_to_value" => {
            "time_savings".to_string()
        }
        "throughput" | "velocity" | "cycle_time" => "delivery".to_string(),
        "insight" | "discovery" => "learning".to_string(),
        other => other.to_string(),
    }
}

pub fn compute_list_value_currencies(
    input: &ListValueCurrenciesInput,
) -> ListValueCurrenciesOutput {
    let mut rows: Vec<String> = Vec::new();
    if !input.value_list.is_empty() {
        rows.extend(input.value_list.iter().map(|v| v.to_string()));
    } else if let Some(csv) = input.value_csv.as_deref() {
        rows.extend(
            csv.split(|ch| matches!(ch, ',' | ';' | '|' | '\n'))
                .map(|row| row.trim().to_string())
                .filter(|row| !row.is_empty()),
        );
    }
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        let token_base = normalize_value_currency_token_with_allowed(&row, &input.allowed_keys);
        let token_alias = canonical_value_currency_alias(&token_base);
        let token = normalize_value_currency_token_with_allowed(&token_alias, &input.allowed_keys);
        if token.is_empty() {
            continue;
        }
        let dedupe_key = token.to_ascii_lowercase();
        if seen.insert(dedupe_key) {
            out.push(token);
        }
    }
    ListValueCurrenciesOutput { currencies: out }
}

pub fn compute_infer_value_currencies_from_directive_bits(
    input: &InferValueCurrenciesFromDirectiveBitsInput,
) -> InferValueCurrenciesFromDirectiveBitsOutput {
    let blob = normalize_spaces(&input.bits.join(" ")).to_ascii_lowercase();
    if blob.is_empty() {
        return InferValueCurrenciesFromDirectiveBitsOutput {
            currencies: Vec::new(),
        };
    }

    let revenue_re =
        Regex::new(r"\b(revenue|mrr|arr|cash|money|usd|dollar|profit|pricing|invoice|paid|payment|billing|income)\b")
            .expect("valid revenue regex");
    let delivery_re =
        Regex::new(r"\b(deliver|delivery|ship|release|milestone|throughput|lead[\s_-]?time|cycle[\s_-]?time|backlog)\b")
            .expect("valid delivery regex");
    let user_re = Regex::new(
        r"\b(customer|user|adoption|engagement|retention|conversion|satisfaction|onboarding)\b",
    )
    .expect("valid user regex");
    let quality_re = Regex::new(
        r"\b(quality|reliab|uptime|error|stability|safety|accuracy|resilience|regression)\b",
    )
    .expect("valid quality regex");
    let time_re = Regex::new(
        r"\b(time[\s_-]*to[\s_-]*(?:value|cash|revenue)|hours?\s+saved|latency|faster|payback|speed|cycle[\s_-]?time)\b",
    )
    .expect("valid time regex");
    let learning_re =
        Regex::new(r"\b(learn|discovery|research|insight|ab[\s_-]?test|hypothesis)\b")
            .expect("valid learning regex");

    let mut inferred: Vec<String> = Vec::new();
    if revenue_re.is_match(&blob) {
        inferred.push("revenue".to_string());
    }
    if delivery_re.is_match(&blob) {
        inferred.push("delivery".to_string());
    }
    if user_re.is_match(&blob) {
        inferred.push("user_value".to_string());
    }
    if quality_re.is_match(&blob) {
        inferred.push("quality".to_string());
    }
    if time_re.is_match(&blob) {
        inferred.push("time_savings".to_string());
    }
    if learning_re.is_match(&blob) {
        inferred.push("learning".to_string());
    }

    let list_out = compute_list_value_currencies(&ListValueCurrenciesInput {
        value_list: inferred,
        value_csv: None,
        allowed_keys: input.allowed_keys.clone(),
    });
    InferValueCurrenciesFromDirectiveBitsOutput {
        currencies: list_out.currencies,
    }
}

pub fn compute_has_linked_objective_entry(
    input: &HasLinkedObjectiveEntryInput,
) -> HasLinkedObjectiveEntryOutput {
    let linked = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
        value: input.objective_id.clone(),
    })
    .objective_id
    .is_some()
        || compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: input.directive_objective_id.clone(),
        })
        .objective_id
        .is_some()
        || compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: input.directive.clone(),
        })
        .objective_id
        .is_some();
    HasLinkedObjectiveEntryOutput { linked }
}

pub fn compute_verified_entry_outcome(
    input: &VerifiedEntryOutcomeInput,
) -> VerifiedEntryOutcomeOutput {
    if input.outcome_verified {
        return VerifiedEntryOutcomeOutput { verified: true };
    }
    let outcome = input
        .outcome
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let verified = matches!(
        outcome.as_str(),
        "verified"
            | "verified_success"
            | "verified_pass"
            | "shipped"
            | "closed_won"
            | "won"
            | "paid"
            | "revenue_verified"
            | "pass"
    );
    VerifiedEntryOutcomeOutput { verified }
}

pub fn compute_verified_revenue_action(
    input: &VerifiedRevenueActionInput,
) -> VerifiedRevenueActionOutput {
    if input.verified || input.outcome_verified {
        return VerifiedRevenueActionOutput { verified: true };
    }
    let status = input
        .status
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let verified = matches!(
        status.as_str(),
        "verified" | "won" | "paid" | "closed_won" | "received"
    );
    VerifiedRevenueActionOutput { verified }
}

pub fn compute_minutes_until_next_utc_day(
    input: &MinutesUntilNextUtcDayInput,
) -> MinutesUntilNextUtcDayOutput {
    let now = input
        .now_ms
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    if !now.is_finite() || now <= 0.0 {
        return MinutesUntilNextUtcDayOutput { minutes: 0.0 };
    }
    let Some(now_dt) = DateTime::<Utc>::from_timestamp_millis(now as i64) else {
        return MinutesUntilNextUtcDayOutput { minutes: 0.0 };
    };
    let next_day = DateTime::<Utc>::from_naive_utc_and_offset(
        now_dt
            .date_naive()
            .and_hms_milli_opt(0, 0, 0, 0)
            .expect("valid midnight")
            + Duration::days(1),
        Utc,
    );
    let delta_ms = (next_day.timestamp_millis() - now_dt.timestamp_millis()).max(0) as f64;
    let minutes = (delta_ms / 60000.0).ceil().max(0.0);
    MinutesUntilNextUtcDayOutput { minutes }
}

pub fn compute_age_hours(input: &AgeHoursInput) -> AgeHoursOutput {
    let date = input.date.as_deref().unwrap_or("").trim();
    if date.is_empty() {
        return AgeHoursOutput { age_hours: 0.0 };
    }
    let Ok(parsed_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return AgeHoursOutput { age_hours: 0.0 };
    };
    let Some(start_naive) = parsed_date.and_hms_milli_opt(0, 0, 0, 0) else {
        return AgeHoursOutput { age_hours: 0.0 };
    };
    let start = DateTime::<Utc>::from_naive_utc_and_offset(start_naive, Utc);
    let now_ms = input
        .now_ms
        .filter(|v| v.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let age_hours = ((now_ms - start.timestamp_millis() as f64) / 3_600_000.0).max(0.0);
    AgeHoursOutput { age_hours }
}

pub fn compute_url_domain(input: &UrlDomainInput) -> UrlDomainOutput {
    let raw = input.url.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return UrlDomainOutput {
            domain: String::new(),
        };
    }
    let Some((_, rest)) = raw.split_once("://") else {
        return UrlDomainOutput {
            domain: String::new(),
        };
    };
    let host_port = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if host_port.is_empty() {
        return UrlDomainOutput {
            domain: String::new(),
        };
    }
    let without_auth = host_port.rsplit('@').next().unwrap_or("");
    let host = if without_auth.starts_with('[') {
        without_auth
            .split(']')
            .next()
            .map(|v| format!("{v}]"))
            .unwrap_or_else(String::new)
    } else {
        without_auth.split(':').next().unwrap_or("").to_string()
    };
    UrlDomainOutput { domain: host }
}

pub fn compute_domain_allowed(input: &DomainAllowedInput) -> DomainAllowedOutput {
    let domain = input
        .domain
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if domain.is_empty() {
        return DomainAllowedOutput { allowed: false };
    }
    if input.allowlist.is_empty() {
        return DomainAllowedOutput { allowed: true };
    }
    let allowed = input.allowlist.iter().any(|raw| {
        let allowed_domain = raw.trim().to_ascii_lowercase();
        if allowed_domain.is_empty() {
            return false;
        }
        domain == allowed_domain || domain.ends_with(&format!(".{allowed_domain}"))
    });
    DomainAllowedOutput { allowed }
}

pub fn compute_is_execute_mode(input: &IsExecuteModeInput) -> IsExecuteModeOutput {
    let mode = input.execution_mode.as_deref().unwrap_or("");
    IsExecuteModeOutput {
        execute_mode: mode == "execute" || mode == "canary_execute",
    }
}

pub fn compute_execution_allowed_by_feature_flag(
    input: &ExecutionAllowedByFeatureFlagInput,
) -> ExecutionAllowedByFeatureFlagOutput {
    if input.shadow_only {
        return ExecutionAllowedByFeatureFlagOutput { allowed: true };
    }
    if input.autonomy_enabled {
        return ExecutionAllowedByFeatureFlagOutput { allowed: true };
    }
    let canary = input.execution_mode.as_deref().unwrap_or("");
    ExecutionAllowedByFeatureFlagOutput {
        allowed: input.canary_allow_with_flag_off && canary == "canary_execute",
    }
}

pub fn compute_is_tier1_objective_id(input: &IsTier1ObjectiveIdInput) -> IsTier1ObjectiveIdOutput {
    let id = input.objective_id.as_deref().unwrap_or("").trim();
    if id.is_empty() {
        return IsTier1ObjectiveIdOutput { tier1: false };
    }
    let re = Regex::new(r"(?i)^T1(?:\b|[_:-])").expect("valid tier1 objective regex");
    IsTier1ObjectiveIdOutput {
        tier1: re.is_match(id),
    }
}

pub fn compute_is_tier1_candidate_objective(
    input: &IsTier1CandidateObjectiveInput,
) -> IsTier1CandidateObjectiveOutput {
    let pulse_tier = compute_normalize_directive_tier(&NormalizeDirectiveTierInput {
        raw_tier: input.directive_pulse_tier,
        fallback: Some(99.0),
    })
    .tier;
    if pulse_tier <= 1 {
        return IsTier1CandidateObjectiveOutput { tier1: true };
    }
    let by_binding = compute_is_tier1_objective_id(&IsTier1ObjectiveIdInput {
        objective_id: input.objective_binding_objective_id.clone(),
    })
    .tier1;
    if by_binding {
        return IsTier1CandidateObjectiveOutput { tier1: true };
    }
    let by_pulse = compute_is_tier1_objective_id(&IsTier1ObjectiveIdInput {
        objective_id: input.directive_pulse_objective_id.clone(),
    })
    .tier1;
    IsTier1CandidateObjectiveOutput { tier1: by_pulse }
}

pub fn compute_needs_execution_quota(
    input: &NeedsExecutionQuotaInput,
) -> NeedsExecutionQuotaOutput {
    if input.shadow_only {
        return NeedsExecutionQuotaOutput { required: false };
    }
    let execute_mode = compute_is_execute_mode(&IsExecuteModeInput {
        execution_mode: input.execution_mode.clone(),
    })
    .execute_mode;
    if !execute_mode {
        return NeedsExecutionQuotaOutput { required: false };
    }
    if !input.min_daily_executions.is_finite() || input.min_daily_executions <= 0.0 {
        return NeedsExecutionQuotaOutput { required: false };
    }
    NeedsExecutionQuotaOutput {
        required: input.executed_today < input.min_daily_executions,
    }
}

pub fn compute_normalize_criteria_metric(
    input: &NormalizeCriteriaMetricInput,
) -> NormalizeCriteriaMetricOutput {
    let normalized = normalize_spaces(input.value.as_deref().unwrap_or(""));
    let metric = Regex::new(r"[\s-]+")
        .expect("valid criteria metric regex")
        .replace_all(&normalized.to_ascii_lowercase(), "_")
        .to_string();
    NormalizeCriteriaMetricOutput { metric }
}

pub fn compute_escape_reg_exp(input: &EscapeRegExpInput) -> EscapeRegExpOutput {
    let value = input.value.as_deref().unwrap_or("");
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(
            ch,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    EscapeRegExpOutput { escaped }
}

pub fn compute_tool_token_mentioned(input: &ToolTokenMentionedInput) -> ToolTokenMentionedOutput {
    let text = input.blob.as_deref().unwrap_or("");
    let tok = input
        .token
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if text.is_empty() || tok.is_empty() {
        return ToolTokenMentionedOutput { mentioned: false };
    }
    let escaped = compute_escape_reg_exp(&EscapeRegExpInput {
        value: Some(tok.clone()),
    })
    .escaped;
    let exact_re = Regex::new(&format!(r"\b{}\b", escaped)).expect("valid exact tool token regex");
    if exact_re.is_match(text) {
        return ToolTokenMentionedOutput { mentioned: true };
    }
    if tok == "bird_x" {
        let bird_re = Regex::new(r"\bbird[\s_-]*x\b").expect("valid bird_x regex");
        if bird_re.is_match(text) {
            return ToolTokenMentionedOutput { mentioned: true };
        }
    }
    ToolTokenMentionedOutput { mentioned: false }
}

pub fn compute_policy_hold_reason_from_event(
    input: &PolicyHoldReasonFromEventInput,
) -> PolicyHoldReasonFromEventOutput {
    let hold_reason = normalize_spaces(input.hold_reason.as_deref().unwrap_or(""));
    let route_block = normalize_spaces(input.route_block_reason.as_deref().unwrap_or(""));
    let explicit = if !hold_reason.is_empty() {
        hold_reason.to_ascii_lowercase()
    } else {
        route_block.to_ascii_lowercase()
    };
    if !explicit.is_empty() {
        return PolicyHoldReasonFromEventOutput { reason: explicit };
    }
    let result = normalize_spaces(input.result.as_deref().unwrap_or("")).to_ascii_lowercase();
    if !result.is_empty() {
        return PolicyHoldReasonFromEventOutput { reason: result };
    }
    PolicyHoldReasonFromEventOutput {
        reason: "policy_hold_unknown".to_string(),
    }
}

pub fn compute_strategy_marker_tokens(
    input: &StrategyMarkerTokensInput,
) -> StrategyMarkerTokensOutput {
    let mut token_set = std::collections::BTreeSet::new();
    let mut text_parts: Vec<String> = Vec::new();
    if let Some(primary) = input.objective_primary.as_ref() {
        text_parts.push(primary.clone());
    }
    if let Some(metric) = input.objective_fitness_metric.as_ref() {
        text_parts.push(metric.clone());
    }
    text_parts.extend(input.objective_secondary.iter().cloned());
    text_parts.extend(input.tags.iter().cloned());

    for part in text_parts {
        let normalized =
            compute_normalize_directive_text(&NormalizeDirectiveTextInput { text: Some(part) })
                .normalized;
        if normalized.is_empty() {
            continue;
        }
        let tokenized = compute_tokenize_directive_text(&TokenizeDirectiveTextInput {
            text: Some(normalized),
            stopwords: Vec::new(),
        })
        .tokens;
        for token in tokenized {
            token_set.insert(token);
        }
    }
    StrategyMarkerTokensOutput {
        tokens: token_set.into_iter().collect(),
    }
}
