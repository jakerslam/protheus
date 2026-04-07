pub fn compute_conclave_high_risk_flags(
    input: &ConclaveHighRiskFlagsInput,
) -> ConclaveHighRiskFlagsOutput {
    let payload = input.payload.as_ref().and_then(|v| v.as_object());
    let max_divergence = input.max_divergence.unwrap_or(0.45);
    let min_confidence = input.min_confidence.unwrap_or(0.6);
    let mut flags: Vec<String> = Vec::new();

    let winner = clean_text_runtime(
        &value_to_string(payload.and_then(|row| row.get("winner"))),
        120,
    );
    if payload.is_none()
        || payload
            .and_then(|row| row.get("ok"))
            .and_then(|v| v.as_bool())
            != Some(true)
        || winner.is_empty()
    {
        push_unique(&mut flags, "no_consensus".to_string());
    }

    let divergence =
        parse_number_like(payload.and_then(|row| row.get("max_divergence"))).unwrap_or(0.0);
    if !divergence.is_finite() || divergence > max_divergence {
        push_unique(&mut flags, "high_divergence".to_string());
    }

    let persona_outputs = payload
        .and_then(|row| row.get("persona_outputs"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let confidences = persona_outputs
        .iter()
        .filter_map(|row| row.as_object())
        .filter_map(|row| parse_number_like(row.get("confidence")))
        .filter(|n| n.is_finite())
        .collect::<Vec<_>>();
    if !confidences.is_empty()
        && confidences.iter().fold(f64::INFINITY, |acc, n| acc.min(*n)) < min_confidence
    {
        push_unique(&mut flags, "low_confidence".to_string());
    }

    let mut corpus_rows = vec![
        clean_text_runtime(input.query.as_deref().unwrap_or(""), 2400),
        clean_text_runtime(input.summary.as_deref().unwrap_or(""), 1200),
        clean_text_runtime(
            &value_to_string(payload.and_then(|row| row.get("suggested_resolution"))),
            1600,
        ),
    ];
    for row in &persona_outputs {
        if let Some(map) = row.as_object() {
            corpus_rows.push(clean_text_runtime(
                &value_to_string(map.get("recommendation")),
                1200,
            ));
            if let Some(reasoning) = map.get("reasoning").and_then(|v| v.as_array()) {
                for reason in reasoning {
                    corpus_rows.push(clean_text_runtime(&value_to_string(Some(reason)), 240));
                }
            }
        }
    }
    let corpus = corpus_rows.join("\n").to_lowercase();
    for keyword in &input.high_risk_keywords {
        if keyword.is_empty() {
            continue;
        }
        if corpus.contains(&keyword.to_lowercase()) {
            let token = normalize_token_runtime(keyword, 80);
            let flag = if token.is_empty() {
                "keyword:risk".to_string()
            } else {
                format!("keyword:{token}")
            };
            push_unique(&mut flags, flag);
        }
    }

    ConclaveHighRiskFlagsOutput { flags }
}

fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == &value) {
            out.push(value);
        }
    }
    out
}

fn value_to_csv_list(value: Option<&Value>) -> Vec<String> {
    let Some(v) = value else {
        return Vec::new();
    };
    if let Some(arr) = v.as_array() {
        return arr
            .iter()
            .map(|row| value_to_string(Some(row)))
            .collect::<Vec<_>>();
    }
    value_to_string(Some(v))
        .split(',')
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
}

pub fn compute_tokenize_text(input: &TokenizeTextInput) -> TokenizeTextOutput {
    let max_tokens = input.max_tokens.unwrap_or(64).clamp(0, 256) as usize;
    let text = clean_text_runtime(input.value.as_deref().unwrap_or(""), 1200).to_lowercase();
    let raw = text
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    let tokens = dedupe_preserve_order(
        raw.split(' ')
            .map(|row| row.trim())
            .filter(|row| row.len() >= 3)
            .map(|row| row.to_string())
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(max_tokens)
    .collect::<Vec<_>>();
    TokenizeTextOutput { tokens }
}

pub fn compute_normalize_list(input: &NormalizeListInput) -> NormalizeListOutput {
    let max_len = input.max_len.unwrap_or(80).clamp(1, 400) as usize;
    let mut values = value_to_csv_list(input.value.as_ref())
        .iter()
        .map(|row| normalize_token_runtime(row, max_len))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    values = dedupe_preserve_order(values);
    values.truncate(64);
    NormalizeListOutput { items: values }
}

pub fn compute_normalize_text_list(input: &NormalizeTextListInput) -> NormalizeTextListOutput {
    let max_len = input.max_len.unwrap_or(180).clamp(1, 2000) as usize;
    let max_items = input.max_items.unwrap_or(64).clamp(0, 1024) as usize;
    let mut out = Vec::new();
    for row in value_to_csv_list(input.value.as_ref()) {
        let next = clean_text_runtime(&row, max_len);
        if next.is_empty() {
            continue;
        }
        if out.iter().any(|existing| existing == &next) {
            continue;
        }
        out.push(next);
        if out.len() >= max_items {
            break;
        }
    }
    NormalizeTextListOutput { items: out }
}

pub fn compute_parse_json_from_stdout(
    input: &ParseJsonFromStdoutInput,
) -> ParseJsonFromStdoutOutput {
    let text = input.raw.as_deref().unwrap_or("").trim();
    if text.is_empty() {
        return ParseJsonFromStdoutOutput { parsed: None };
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return ParseJsonFromStdoutOutput {
            parsed: Some(value),
        };
    }
    let lines = text
        .split('\n')
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    for line in lines.iter().rev() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            return ParseJsonFromStdoutOutput {
                parsed: Some(value),
            };
        }
    }
    ParseJsonFromStdoutOutput { parsed: None }
}

pub fn compute_parse_args(input: &ParseArgsInput) -> ParseArgsOutput {
    let mut positional = Vec::new();
    let mut map = serde_json::Map::new();
    let argv = &input.argv;
    let mut idx = 0usize;
    while idx < argv.len() {
        let tok = argv[idx].clone();
        if !tok.starts_with("--") {
            positional.push(tok);
            idx += 1;
            continue;
        }
        if let Some(eq) = tok.find('=') {
            let key = tok.chars().skip(2).take(eq - 2).collect::<String>();
            let value = tok.chars().skip(eq + 1).collect::<String>();
            map.insert(key, Value::String(value));
            idx += 1;
            continue;
        }
        let key = tok.chars().skip(2).collect::<String>();
        if idx + 1 < argv.len() && !argv[idx + 1].starts_with("--") {
            map.insert(key, Value::String(argv[idx + 1].clone()));
            idx += 2;
            continue;
        }
        map.insert(key, Value::Bool(true));
        idx += 1;
    }
    map.insert(
        "_".to_string(),
        Value::Array(
            positional
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    ParseArgsOutput {
        args: Value::Object(map),
    }
}

pub fn compute_library_match_score(input: &LibraryMatchScoreInput) -> LibraryMatchScoreOutput {
    let token_score = compute_jaccard_similarity(&JaccardSimilarityInput {
        left_tokens: input.query_signature_tokens.clone(),
        right_tokens: input.row_signature_tokens.clone(),
    })
    .similarity;
    let trit_score = compute_trit_similarity(&TritSimilarityInput {
        query_vector: input.query_trit_vector.clone(),
        entry_trit: Some(Value::from(input.row_outcome_trit.unwrap_or(0))),
    })
    .similarity;
    let query_target = input.query_target.as_deref().unwrap_or("");
    let row_target = input.row_target.as_deref().unwrap_or("");
    let target_score = if query_target == row_target { 1.0 } else { 0.0 };
    let token_weight = input.token_weight.unwrap_or(0.0);
    let trit_weight = input.trit_weight.unwrap_or(0.0);
    let target_weight = input.target_weight.unwrap_or(0.0);
    let total_weight = (token_weight + trit_weight + target_weight).max(0.0001);
    let score = ((token_score * token_weight)
        + (trit_score * trit_weight)
        + (target_score * target_weight))
        / total_weight;
    let score = clamp_number(score, 0.0, 1.0);
    let score = (score * 1_000_000.0).round() / 1_000_000.0;
    LibraryMatchScoreOutput { score }
}

pub fn compute_known_failure_pressure(
    input: &KnownFailurePressureInput,
) -> KnownFailurePressureOutput {
    let block_similarity = input.failed_repetition_similarity_block.unwrap_or(0.72);
    let mut fail_count = 0i64;
    let mut hard_block = false;
    let mut max_similarity = 0.0f64;
    for candidate in &input.candidates {
        let row = candidate
            .as_object()
            .and_then(|obj| obj.get("row"))
            .and_then(|v| v.as_object());
        let similarity =
            parse_number_like(candidate.as_object().and_then(|obj| obj.get("similarity")))
                .unwrap_or(0.0);
        if let Some(row_obj) = row {
            let outcome = parse_number_like(row_obj.get("outcome_trit")).unwrap_or(0.0);
            if outcome < 0.0 {
                fail_count += 1;
                if similarity >= block_similarity {
                    hard_block = true;
                }
                if similarity > max_similarity {
                    max_similarity = similarity;
                }
            }
        }
    }
    let max_similarity = (max_similarity * 1_000_000.0).round() / 1_000_000.0;
    KnownFailurePressureOutput {
        fail_count,
        hard_block,
        max_similarity,
    }
}

pub fn compute_has_signal_term_match(input: &HasSignalTermMatchInput) -> HasSignalTermMatchOutput {
    let haystack = input.haystack.as_deref().unwrap_or("");
    let token_set = input
        .token_set
        .iter()
        .map(|row| row.to_string())
        .collect::<BTreeSet<_>>();
    let term = clean_text_runtime(input.term.as_deref().unwrap_or(""), 200).to_lowercase();
    if term.is_empty() {
        return HasSignalTermMatchOutput { matched: false };
    }
    let words = term
        .split_whitespace()
        .map(regex::escape)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return HasSignalTermMatchOutput { matched: false };
    }
    let phrase_re = Regex::new(&format!(r"\b{}\b", words.join(r"\s+"))).ok();
    if let Some(re) = phrase_re {
        if re.is_match(haystack) {
            return HasSignalTermMatchOutput { matched: true };
        }
    }
    let parts = term.split_whitespace().collect::<Vec<_>>();
    if parts.len() == 1 {
        return HasSignalTermMatchOutput {
            matched: token_set.contains(parts[0]),
        };
    }
    HasSignalTermMatchOutput {
        matched: parts.iter().all(|part| token_set.contains(*part)),
    }
}

pub fn compute_count_axiom_signal_groups(
    input: &CountAxiomSignalGroupsInput,
) -> CountAxiomSignalGroupsOutput {
    let normalize_terms = |rows: &Vec<String>| -> Vec<String> {
        rows.iter()
            .map(|row| clean_text_runtime(row, 200).to_lowercase())
            .filter(|row| !row.is_empty())
            .take(32)
            .collect::<Vec<_>>()
    };
    let groups = vec![
        normalize_terms(&input.action_terms),
        normalize_terms(&input.subject_terms),
        normalize_terms(&input.object_terms),
    ];
    let haystack = input.haystack.as_deref().unwrap_or("");
    let token_set = input
        .token_set
        .iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    let mut matched = 0i64;
    let configured = groups.iter().filter(|terms| !terms.is_empty()).count() as i64;
    for terms in &groups {
        if terms.is_empty() {
            continue;
        }
        let hit = terms.iter().any(|term| {
            compute_has_signal_term_match(&HasSignalTermMatchInput {
                haystack: Some(haystack.to_string()),
                token_set: token_set.clone(),
                term: Some(term.to_string()),
            })
            .matched
        });
        if hit {
            matched += 1;
        }
    }
    let required_default = configured;
    let required = input
        .min_signal_groups
        .unwrap_or(required_default)
        .clamp(0, 3);
    CountAxiomSignalGroupsOutput {
        configured_groups: configured,
        matched_groups: matched,
        required_groups: required,
        pass: matched >= required,
    }
}

pub fn compute_effective_first_n_human_veto_uses(
    input: &EffectiveFirstNHumanVetoUsesInput,
) -> EffectiveFirstNHumanVetoUsesOutput {
    let key = normalize_token(input.target.as_deref().unwrap_or("tactical"), 24);
    let configured =
        read_rank_key(input.first_live_uses_require_human_veto.as_ref(), &key, 0).clamp(0, 100_000);
    let minimum = read_rank_key(
        input.minimum_first_live_uses_require_human_veto.as_ref(),
        &key,
        0,
    )
    .clamp(0, 100_000);
    EffectiveFirstNHumanVetoUsesOutput {
        uses: configured.max(minimum),
    }
}

fn decode_input<T>(payload: &Value, key: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Default,
{
    let value = payload.get(key).cloned().unwrap_or_else(|| json!({}));
    serde_json::from_value(value).map_err(|e| format!("inversion_decode_{key}_failed:{e}"))
}
