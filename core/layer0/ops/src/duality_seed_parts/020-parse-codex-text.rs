fn parse_codex_text(text: &str) -> Value {
    let base = default_codex();
    let base_obj = base.as_object().cloned().unwrap_or_default();

    let mut version = as_str(base_obj.get("version"));
    let mut section = String::new();
    let mut flux_pairs = Vec::<Value>::new();
    let mut flow_values = Vec::<String>::new();
    let mut balance_rules = Map::<String, Value>::new();
    let mut asymptote = Map::<String, Value>::new();
    let mut warnings = Vec::<String>::new();

    for line in text
        .replace('\r', "")
        .lines()
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty() && !row.starts_with('#'))
    {
        if line.starts_with('[') && line.ends_with(']') {
            section = normalize_word(&line[1..line.len() - 1], 80);
            continue;
        }

        if section == "meta" {
            let chunks = line
                .split('=')
                .map(|row| row.trim().to_string())
                .collect::<Vec<_>>();
            if chunks.len() >= 2 && normalize_word(&chunks[0], 40) == "version" {
                version = clean_text(&chunks[1..].join("="), 40);
            }
            continue;
        }

        if section == "flux_pairs" {
            if line.contains('|') {
                let parts = line
                    .split('|')
                    .map(|row| clean_text(row, 160))
                    .collect::<Vec<_>>();
                if parts.len() >= 2 {
                    let yin = normalize_word(&parts[0], 40);
                    let yang = normalize_word(&parts[1], 40);
                    if yin.is_empty() || yang.is_empty() {
                        continue;
                    }
                    let mut yin_attrs = Vec::<String>::new();
                    let mut yang_attrs = Vec::<String>::new();
                    for part in parts.iter().skip(2) {
                        let kv = part
                            .split('=')
                            .map(|row| row.trim().to_string())
                            .collect::<Vec<_>>();
                        if kv.len() < 2 {
                            continue;
                        }
                        let key = normalize_word(&kv[0], 40);
                        let value = kv[1..].join("=");
                        if matches!(key.as_str(), "yin_attrs" | "yin" | "yinattr" | "yinattrs") {
                            yin_attrs = parse_attrs(&value);
                        } else if matches!(
                            key.as_str(),
                            "yang_attrs" | "yang" | "yangattr" | "yangattrs"
                        ) {
                            yang_attrs = parse_attrs(&value);
                        }
                    }
                    flux_pairs.push(json!({
                        "yin": yin,
                        "yang": yang,
                        "yin_attrs": yin_attrs,
                        "yang_attrs": yang_attrs
                    }));
                }
            } else if line.contains("<->") {
                let parts = line
                    .split("<->")
                    .map(|row| normalize_word(row, 40))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>();
                if parts.len() >= 2 {
                    flux_pairs.push(json!({
                        "yin": parts[0],
                        "yang": parts[1],
                        "yin_attrs": [],
                        "yang_attrs": []
                    }));
                }
            }
            continue;
        }

        if section == "flow_values" {
            for chunk in line.split([',', ';']) {
                let value = clean_text(chunk, 120);
                if !value.contains('/') || value.is_empty() {
                    continue;
                }
                if flow_values.iter().any(|existing| existing == &value) {
                    continue;
                }
                flow_values.push(value);
            }
            continue;
        }

        if section == "balance_rules" || section == "asymptote" {
            let chunks = if line.contains('=') {
                line.split('=')
                    .map(|row| row.trim().to_string())
                    .collect::<Vec<_>>()
            } else {
                line.split(':')
                    .map(|row| row.trim().to_string())
                    .collect::<Vec<_>>()
            };
            if chunks.len() >= 2 {
                let key = normalize_word(&chunks[0], 64);
                let value = normalize_word(&chunks[1..].join("="), 120);
                if key.is_empty() || value.is_empty() {
                    continue;
                }
                if section == "balance_rules" {
                    balance_rules.insert(key, Value::String(value));
                } else {
                    asymptote.insert(key, Value::String(value));
                }
            }
            continue;
        }

        if section == "warnings" {
            let token = normalize_word(&line, 120);
            if !token.is_empty() {
                warnings.push(token);
            }
            continue;
        }
    }

    json!({
        "version": if version.is_empty() { "1.0".to_string() } else { version },
        "flux_pairs": if flux_pairs.is_empty() {
            base_obj.get("flux_pairs").cloned().unwrap_or_else(|| json!([]))
        } else {
            Value::Array(flux_pairs)
        },
        "flow_values": if flow_values.is_empty() {
            base_obj.get("flow_values").cloned().unwrap_or_else(|| json!([]))
        } else {
            Value::Array(flow_values.into_iter().map(Value::String).collect::<Vec<_>>())
        },
        "balance_rules": if balance_rules.is_empty() {
            base_obj.get("balance_rules").cloned().unwrap_or_else(|| json!({}))
        } else {
            Value::Object(balance_rules)
        },
        "asymptote": if asymptote.is_empty() {
            base_obj.get("asymptote").cloned().unwrap_or_else(|| json!({}))
        } else {
            Value::Object(asymptote)
        },
        "warnings": if warnings.is_empty() {
            base_obj.get("warnings").cloned().unwrap_or_else(|| json!([]))
        } else {
            Value::Array(warnings.into_iter().map(Value::String).collect::<Vec<_>>())
        }
    })
}

fn load_codex(policy: &Value) -> Value {
    let codex_path = policy
        .get("codex_path")
        .map(|v| PathBuf::from(as_str(Some(v))))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_REL));
    let text = read_text(&codex_path);
    parse_codex_text(&text)
}

fn default_state() -> Value {
    json!({
        "schema_id": "duality_seed_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "seed_confidence": 1.0,
        "toll_debt": 0.0,
        "toll_events_total": 0,
        "last_toll_update_ts": Value::Null,
        "observations_total": 0,
        "contradictions_total": 0,
        "supports_total": 0,
        "neutral_total": 0,
        "consecutive_contradictions": 0,
        "consecutive_supports": 0,
        "observation_window": [],
        "self_validation": {
            "last_run_ts": Value::Null,
            "confidence": 0.0,
            "scenario_count": 0
        }
    })
}

fn load_state(policy: &Value) -> Value {
    let state_path = policy
        .get("state")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("latest_path"))
        .map(|v| PathBuf::from(as_str(Some(v))))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LATEST_REL));

    let src = read_json(&state_path);
    if !src.is_object() {
        return default_state();
    }
    let src_obj = src.as_object().cloned().unwrap_or_default();
    let base_obj = default_state().as_object().cloned().unwrap_or_default();

    let mut out = base_obj;
    for (key, value) in src_obj {
        out.insert(key, value);
    }

    out.insert(
        "seed_confidence".to_string(),
        json!(clamp_f64(
            as_f64(out.get("seed_confidence")).unwrap_or(1.0),
            0.0,
            1.0,
        )),
    );
    out.insert(
        "toll_debt".to_string(),
        json!(clamp_f64(as_f64(out.get("toll_debt")).unwrap_or(0.0), 0.0, 100.0)),
    );
    out.insert(
        "toll_events_total".to_string(),
        json!(clamp_i64(
            as_i64(out.get("toll_events_total")).unwrap_or(0),
            0,
            100_000_000
        )),
    );
    out.insert(
        "observations_total".to_string(),
        json!(clamp_i64(
            as_i64(out.get("observations_total")).unwrap_or(0),
            0,
            100_000_000
        )),
    );
    out.insert(
        "contradictions_total".to_string(),
        json!(clamp_i64(
            as_i64(out.get("contradictions_total")).unwrap_or(0),
            0,
            100_000_000,
        )),
    );
    out.insert(
        "supports_total".to_string(),
        json!(clamp_i64(
            as_i64(out.get("supports_total")).unwrap_or(0),
            0,
            100_000_000
        )),
    );
    out.insert(
        "neutral_total".to_string(),
        json!(clamp_i64(
            as_i64(out.get("neutral_total")).unwrap_or(0),
            0,
            100_000_000
        )),
    );
    out.insert(
        "consecutive_contradictions".to_string(),
        json!(clamp_i64(
            as_i64(out.get("consecutive_contradictions")).unwrap_or(0),
            0,
            100_000_000,
        )),
    );
    out.insert(
        "consecutive_supports".to_string(),
        json!(clamp_i64(
            as_i64(out.get("consecutive_supports")).unwrap_or(0),
            0,
            100_000_000,
        )),
    );

    if let Some(window) = out.get("observation_window").and_then(Value::as_array) {
        let trimmed = window
            .iter()
            .filter(|row| row.is_object())
            .cloned()
            .rev()
            .take(2000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        out.insert("observation_window".to_string(), Value::Array(trimmed));
    } else {
        out.insert("observation_window".to_string(), Value::Array(Vec::new()));
    }

    Value::Object(out)
}

fn persist_state(policy: &Value, state: &Value) -> Result<Value, String> {
    let state_path = policy
        .get("state")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("latest_path"))
        .map(|v| PathBuf::from(as_str(Some(v))))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LATEST_REL));

    let mut next = default_state().as_object().cloned().unwrap_or_default();
    if let Some(obj) = state.as_object() {
        for (key, value) in obj {
            next.insert(key.clone(), value.clone());
        }
    }
    next.insert("updated_at".to_string(), Value::String(now_iso()));
    let payload = Value::Object(next);
    write_json_atomic(&state_path, &payload)?;
    Ok(payload)
}

fn tokenize_context(context: &Value) -> Vec<String> {
    fn walk(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Null => {}
            Value::Bool(v) => out.push(v.to_string()),
            Value::Number(v) => out.push(v.to_string()),
            Value::String(v) => out.push(v.clone()),
            Value::Array(rows) => {
                for row in rows {
                    walk(row, out);
                }
            }
            Value::Object(obj) => {
                for (key, value) in obj {
                    out.push(key.clone());
                    walk(value, out);
                }
            }
        }
    }

    let mut raw = Vec::<String>::new();
    walk(context, &mut raw);

    let joined = raw.join(" ").to_ascii_lowercase();
    let mut seen = BTreeSet::<String>::new();
    let mut out = Vec::<String>::new();
    for token in joined
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|row| row.trim().to_string())
        .filter(|row| row.len() >= 3)
    {
        if seen.insert(token.clone()) {
            out.push(token);
        }
        if out.len() >= 512 {
            break;
        }
    }
    out
}

fn keyword_sets(codex: &Value) -> (HashSet<String>, HashSet<String>) {
    let mut yin = HashSet::from([
        "order".to_string(),
        "structure".to_string(),
        "stability".to_string(),
        "planning".to_string(),
        "discipline".to_string(),
        "safety".to_string(),
        "containment".to_string(),
        "precision".to_string(),
        "governance".to_string(),
        "control".to_string(),
        "determinism".to_string(),
    ]);
    let mut yang = HashSet::from([
        "chaos".to_string(),
        "energy".to_string(),
        "variation".to_string(),
        "exploration".to_string(),
        "novelty".to_string(),
        "adaptation".to_string(),
        "creativity".to_string(),
        "inversion".to_string(),
        "mutation".to_string(),
        "breakthrough".to_string(),
        "divergence".to_string(),
    ]);

    if let Some(rows) = codex.get("flux_pairs").and_then(Value::as_array) {
        for row in rows {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let yin_token = normalize_word(&as_str(obj.get("yin")), 60);
            let yang_token = normalize_word(&as_str(obj.get("yang")), 60);
            if !yin_token.is_empty() {
                yin.insert(yin_token);
            }
            if !yang_token.is_empty() {
                yang.insert(yang_token);
            }
            if let Some(yin_attrs) = obj.get("yin_attrs").and_then(Value::as_array) {
                for attr in yin_attrs {
                    let token = normalize_word(&as_str(Some(attr)), 60);
                    if !token.is_empty() {
                        yin.insert(token);
                    }
                }
            }
            if let Some(yang_attrs) = obj.get("yang_attrs").and_then(Value::as_array) {
                for attr in yang_attrs {
                    let token = normalize_word(&as_str(Some(attr)), 60);
                    if !token.is_empty() {
                        yang.insert(token);
                    }
                }
            }
        }
    }

    (yin, yang)
}

fn lane_enabled(policy: &Value, lane_raw: &str) -> bool {
    let lane = normalize_token(lane_raw, 120);
    let key = match lane.as_str() {
        "belief_formation" => Some("belief_formation"),
        "inversion_trigger" => Some("inversion_trigger"),
        "assimilation_candidacy" => Some("assimilation_candidacy"),
        "task_decomposition" => Some("task_decomposition"),
        "weaver_arbitration" => Some("weaver_arbitration"),
        "heroic_echo_filtering" => Some("heroic_echo_filtering"),
        _ => None,
    };
    let Some(flag_key) = key else {
        return true;
    };
    let integration = policy
        .get("integration")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    as_bool(integration.get(flag_key), true)
}

fn recommend_adjustment(yin_hits: usize, yang_hits: usize) -> &'static str {
    if yin_hits == 0 && yang_hits == 0 {
        "introduce_balanced_order_and_flux"
    } else if yin_hits > yang_hits {
        "increase_yang_flux"
    } else if yang_hits > yin_hits {
        "increase_yin_order"
    } else {
        "hold_balance_near_zero_point"
    }
}
