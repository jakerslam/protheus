fn resolve_component_path(policy: &Value, key: &str, fallback_rel: &str, root: &Path) -> PathBuf {
    let raw = policy["paths"]
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if raw.is_empty() {
        return root.join(fallback_rel);
    }
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn compute_identity_component(policy: &Value, root: &Path) -> Component {
    let path = resolve_component_path(
        policy,
        "identity_latest_path",
        "local/state/ops/identity/latest.json",
        root,
    );
    let latest = read_json_value(&path, json!({}));
    let summary = latest.get("summary").unwrap_or(&latest);
    let drift_score = clamp_number(
        summary
            .get("identity_drift_score")
            .or_else(|| latest.get("identity_drift_score")),
        0.0,
        1.0,
        0.5,
    );
    let max_drift = clamp_number(
        summary
            .get("max_identity_drift_score")
            .or_else(|| latest.get("max_identity_drift_score")),
        0.01,
        1.0,
        0.58,
    );
    let blocked = clamp_int(
        summary.get("blocked").or_else(|| latest.get("blocked")),
        0,
        1_000_000,
        0,
    );
    let checked = clamp_int(
        summary.get("checked").or_else(|| latest.get("checked")),
        0,
        1_000_000,
        0,
    );
    let drift_ratio = (drift_score / max_drift.max(0.0001)).clamp(0.0, 1.5);
    let blocked_ratio = if checked > 0 {
        (blocked as f64 / checked as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let score = (1.0 - ((drift_ratio * 0.75) + (blocked_ratio * 0.25))).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "drift_score": round_to(drift_score, 6),
            "max_drift_score": round_to(max_drift, 6),
            "blocked": blocked,
            "checked": checked,
            "blocked_ratio": round_to(blocked_ratio, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_pre_neuralink_component(policy: &Value, root: &Path) -> Component {
    let path = resolve_component_path(
        policy,
        "pre_neuralink_state_path",
        "local/state/ops/pre_neuralink/state.json",
        root,
    );
    let state = read_json_value(&path, json!({}));
    let consent_state = {
        let token = normalize_token(state.get("consent_state"), 40);
        if token.is_empty() {
            "paused".to_string()
        } else {
            token
        }
    };
    let consent_score = match consent_state.as_str() {
        "granted" => 1.0,
        "paused" => 0.45,
        _ => 0.1,
    };
    let signals_total = clamp_int(state.get("signals_total"), 0, 1_000_000_000, 0);
    let routed_total = clamp_int(state.get("routed_total"), 0, 1_000_000_000, 0);
    let blocked_total = clamp_int(state.get("blocked_total"), 0, 1_000_000_000, 0);
    let routed_ratio = if signals_total > 0 {
        (routed_total as f64 / signals_total as f64).clamp(0.0, 1.0)
    } else if consent_state == "granted" {
        0.7
    } else {
        0.4
    };
    let blocked_ratio = if signals_total > 0 {
        (blocked_total as f64 / signals_total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let score = ((consent_score * 0.6) + (routed_ratio * 0.3) + ((1.0 - blocked_ratio) * 0.1))
        .clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "consent_state": consent_state,
            "signals_total": signals_total,
            "routed_total": routed_total,
            "blocked_total": blocked_total,
            "routed_ratio": round_to(routed_ratio, 6),
            "blocked_ratio": round_to(blocked_ratio, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_behavioral_component(policy: &Value, root: &Path) -> Component {
    let path = resolve_component_path(
        policy,
        "deep_symbiosis_state_path",
        "local/state/ops/deep_symbiosis/state.json",
        root,
    );
    let state = read_json_value(&path, json!({}));
    let style = state.get("style").unwrap_or(&Value::Null);
    let samples = clamp_int(state.get("samples"), 0, 1_000_000_000, 0);
    let directness = clamp_number(style.get("directness"), 0.0, 1.0, 0.75);
    let brevity = clamp_number(style.get("brevity"), 0.0, 1.0, 0.7);
    let proactive = clamp_number(style.get("proactive_delta"), 0.0, 1.0, 0.65);
    let sample_score = (samples as f64 / 50.0).clamp(0.0, 1.0);
    let style_score = ((directness + brevity + proactive) / 3.0).clamp(0.0, 1.0);
    let score = ((sample_score * 0.45) + (style_score * 0.55)).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "samples": samples,
            "sample_score": round_to(sample_score, 6),
            "style": {
                "directness": round_to(directness, 6),
                "brevity": round_to(brevity, 6),
                "proactive_delta": round_to(proactive, 6),
            },
            "style_score": round_to(style_score, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_mirror_component(policy: &Value, root: &Path) -> Component {
    let path = resolve_component_path(
        policy,
        "observer_mirror_latest_path",
        "local/state/ops/observer_mirror/latest.json",
        root,
    );
    let latest = read_json_value(&path, json!({}));
    let mood = {
        let token = normalize_token(
            nested(&latest, &["observer", "mood"]).or_else(|| latest.get("mood")),
            40,
        );
        if token.is_empty() {
            "unknown".to_string()
        } else {
            token
        }
    };
    let mood_score = match mood.as_str() {
        "stable" => 1.0,
        "guarded" => 0.7,
        "strained" => 0.35,
        _ => 0.6,
    };
    let rates = nested(&latest, &["summary", "rates"]).unwrap_or(&Value::Null);
    let ship_rate = clamp_number(rates.get("ship_rate"), 0.0, 1.0, 0.5);
    let hold_rate = clamp_number(rates.get("hold_rate"), 0.0, 1.0, 0.3);
    let score =
        ((mood_score * 0.5) + (ship_rate * 0.35) + ((1.0 - hold_rate) * 0.15)).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "mood": mood,
            "mood_score": round_to(mood_score, 6),
            "ship_rate": round_to(ship_rate, 6),
            "hold_rate": round_to(hold_rate, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn score_tier(policy: &Value, score: f64) -> &'static str {
    let low_max = clamp_number(policy["thresholds"].get("low_max"), 0.05, 0.95, 0.45);
    let medium_max = clamp_number(policy["thresholds"].get("medium_max"), 0.1, 0.99, 0.75);
    if score < low_max {
        "low"
    } else if score < medium_max {
        "medium"
    } else {
        "high"
    }
}

fn count_consecutive_high(rows: &[Value], high_min: f64) -> i64 {
    let mut streak = 0_i64;
    for row in rows.iter().rev() {
        let score = clamp_number(row.get("score"), 0.0, 1.0, 0.0);
        if score >= high_min {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

fn compute_allowed_depth(
    policy: &Value,
    score: f64,
    tier: &str,
    sustained_high_samples: i64,
) -> i64 {
    if tier == "low" {
        return clamp_int(policy["recursion"].get("low_depth"), 1, 1_000_000, 1);
    }
    if tier == "medium" {
        let low = clamp_number(policy["thresholds"].get("low_max"), 0.05, 0.95, 0.45);
        let medium = clamp_number(policy["thresholds"].get("medium_max"), 0.1, 0.99, 0.75);
        let denom = (medium - low).max(0.0001);
        let progress = ((score - low) / denom).clamp(0.0, 1.0);
        let extra = if progress >= 0.5 { 1 } else { 0 };
        return clamp_int(policy["recursion"].get("medium_depth"), 1, 1_000_000, 2) + extra;
    }
    let base = clamp_int(policy["recursion"].get("high_base_depth"), 1, 1_000_000, 4);
    let gain_interval = clamp_int(
        policy["recursion"].get("high_streak_gain_interval"),
        1,
        1_000_000,
        2,
    );
    let streak_gain = ((sustained_high_samples - 1).max(0) / gain_interval.max(1)).max(0);
    base + streak_gain
}

fn parse_ts(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn is_fresh(ts: Option<&str>, stale_after_minutes: i64) -> bool {
    let Some(raw) = ts else {
        return false;
    };
    let Some(parsed) = parse_ts(raw) else {
        return false;
    };
    parsed >= Utc::now() - Duration::minutes(stale_after_minutes.max(1))
}

fn evaluate_signal(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let policy = load_policy(root, payload);
    if policy["enabled"].as_bool() != Some(true) {
        return Ok(json!({
            "available": false,
            "type": "symbiosis_coherence_signal",
            "ts": now_iso(),
            "policy_path": rel_path(root, Path::new(policy["policy_path"].as_str().unwrap_or_default())),
            "reason": "policy_disabled",
            "shadow_only": true,
        }));
    }

    let identity = compute_identity_component(&policy, root);
    let pre_neuralink = compute_pre_neuralink_component(&policy, root);
    let behavioral = compute_behavioral_component(&policy, root);
    let mirror = compute_mirror_component(&policy, root);

    let weights = &policy["weights"];
    let score = (identity.score * clamp_number(weights.get("identity"), 0.0, 1.0, 0.34)
        + pre_neuralink.score * clamp_number(weights.get("pre_neuralink"), 0.0, 1.0, 0.22)
        + behavioral.score * clamp_number(weights.get("behavioral"), 0.0, 1.0, 0.22)
        + mirror.score * clamp_number(weights.get("mirror"), 0.0, 1.0, 0.22))
    .clamp(0.0, 1.0);
    let rounded_score = round_to(score, 6);
    let tier = score_tier(&policy, rounded_score);

    let mut state = load_state(&policy);
    let now = now_iso();
    let mut recent_scores = state
        .get("recent_scores")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    recent_scores.push(json!({
        "ts": now,
        "score": rounded_score,
        "tier": tier,
    }));
    let max_recent =
        clamp_int(policy["history"].get("max_recent_scores"), 10, 10_000, 200) as usize;
    let start = recent_scores.len().saturating_sub(max_recent);
    let next_recent = recent_scores[start..].to_vec();
    let sustained_high_samples = count_consecutive_high(
        &next_recent,
        clamp_number(policy["thresholds"].get("high_min"), 0.1, 0.99, 0.75),
    );
    let unbounded_allowed_base = rounded_score
        >= clamp_number(policy["thresholds"].get("unbounded_min"), 0.2, 1.0, 0.9)
        && sustained_high_samples
            >= clamp_int(
                policy["thresholds"].get("sustained_high_samples"),
                1,
                1000,
                6,
            );
    let consent_granted = pre_neuralink
        .detail
        .get("consent_state")
        .and_then(Value::as_str)
        .unwrap_or_default()
        == "granted";
    let identity_clear = identity
        .detail
        .get("blocked")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        <= 0;
    let unbounded_allowed = unbounded_allowed_base
        && (!bool_value(
            policy["recursion"].get("require_granted_consent_for_unbounded"),
            true,
        ) || consent_granted)
        && (!bool_value(
            policy["recursion"].get("require_identity_clear_for_unbounded"),
            true,
        ) || identity_clear);
    let allowed_depth = if unbounded_allowed {
        Value::Null
    } else {
        Value::from(compute_allowed_depth(
            &policy,
            rounded_score,
            tier,
            sustained_high_samples,
        ))
    };

    let payload_out = json!({
        "ok": true,
        "available": true,
        "type": "symbiosis_coherence_signal",
        "ts": now,
        "policy_version": policy["version"],
        "policy_path": rel_path(root, Path::new(policy["policy_path"].as_str().unwrap_or_default())),
        "shadow_only": policy["shadow_only"].as_bool().unwrap_or(true),
        "coherence_score": rounded_score,
        "coherence_tier": tier,
        "component_scores": {
            "identity": identity.score,
            "pre_neuralink": pre_neuralink.score,
            "behavioral": behavioral.score,
            "mirror": mirror.score,
        },
        "components": {
            "identity": identity.detail,
            "pre_neuralink": pre_neuralink.detail,
            "behavioral": behavioral.detail,
            "mirror_feedback": mirror.detail,
        },
        "recursion_gate": {
            "allowed_depth": allowed_depth,
            "unbounded_allowed": unbounded_allowed,
            "sustained_high_samples": sustained_high_samples,
            "required_sustained_high_samples": clamp_int(policy["thresholds"].get("sustained_high_samples"), 1, 1000, 6),
            "high_min_score": clamp_number(policy["thresholds"].get("high_min"), 0.1, 0.99, 0.75),
            "unbounded_min_score": clamp_number(policy["thresholds"].get("unbounded_min"), 0.2, 1.0, 0.9),
        },
        "source_paths": {
            "identity_latest_path": identity.source_path,
            "pre_neuralink_state_path": pre_neuralink.source_path,
            "deep_symbiosis_state_path": behavioral.source_path,
            "observer_mirror_latest_path": mirror.source_path,
            "latest_path": rel_path(root, Path::new(policy["paths"]["latest_path"].as_str().unwrap_or_default())),
        }
    });

    if bool_value(payload.get("persist"), true) {
        state["runs"] = Value::from(clamp_int(state.get("runs"), 0, 1_000_000_000, 0) + 1);
        state["recent_scores"] = Value::Array(next_recent);
        save_state(&policy, &state)?;
        write_json(
            Path::new(policy["paths"]["latest_path"].as_str().unwrap_or_default()),
            &payload_out,
        )?;
        append_jsonl(
            Path::new(
                policy["paths"]["receipts_path"]
                    .as_str()
                    .unwrap_or_default(),
            ),
            &payload_out,
        )?;
    }

    Ok(payload_out)
}

fn load_signal(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let policy = load_policy(root, payload);
    let refresh = bool_value(payload.get("refresh"), false);
    let latest_path = PathBuf::from(policy["paths"]["latest_path"].as_str().unwrap_or_default());
    if !refresh {
        let latest = read_json_value(&latest_path, Value::Null);
        if latest.is_object()
            && latest.get("available").and_then(Value::as_bool) == Some(true)
            && is_fresh(
                latest.get("ts").and_then(Value::as_str),
                clamp_int(policy.get("stale_after_minutes"), 1, 24 * 60, 30),
            )
        {
            let mut out = latest;
            out["latest_path"] = Value::String(latest_path.display().to_string());
            out["latest_path_rel"] = Value::String(rel_path(root, &latest_path));
            return Ok(out);
        }
    }
    let evaluated = evaluate_signal(root, payload)?;
    let mut out = evaluated;
    out["latest_path"] = Value::String(latest_path.display().to_string());
    out["latest_path_rel"] = Value::String(rel_path(root, &latest_path));
    Ok(out)
}

fn parse_depth_request(raw: Option<&Value>) -> (Option<i64>, bool) {
    match raw {
        None => (Some(1), false),
        Some(value) => {
            let token = normalize_token(Some(value), 40);
            if matches!(token.as_str(), "unbounded" | "infinite" | "max" | "none") {
                return (None, true);
            }
            let depth = clamp_int(Some(value), 1, 1_000_000_000, 1);
            (Some(depth), false)
        }
    }
}
