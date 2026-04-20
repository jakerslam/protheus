fn recursion_request(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let signal = if let Some(signal) = payload.get("signal") {
        signal.clone()
    } else {
        load_signal(root, payload)?
    };
    let (requested_depth, parsed_unbounded) = parse_depth_request(
        payload
            .get("requested_depth")
            .or_else(|| payload.get("requestedDepth")),
    );
    let require_unbounded = bool_value(payload.get("require_unbounded"), false) || parsed_unbounded;
    let allowed_depth = signal
        .get("recursion_gate")
        .and_then(|v| v.get("allowed_depth"))
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64()
                    .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            }
        });
    let unbounded_allowed = signal
        .get("recursion_gate")
        .and_then(|v| v.get("unbounded_allowed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let mut blocked = false;
    if signal.get("available").and_then(Value::as_bool) != Some(true) {
        reasons.push(Value::String("symbiosis_signal_unavailable".to_string()));
    } else {
        if require_unbounded && !unbounded_allowed {
            blocked = true;
            reasons.push(Value::String("symbiosis_unbounded_not_allowed".to_string()));
        }
        if let (Some(requested), Some(allowed)) = (requested_depth, allowed_depth) {
            if requested > allowed {
                blocked = true;
                reasons.push(Value::String("symbiosis_depth_exceeds_allowed".to_string()));
            }
        }
    }

    let shadow_only = if payload.contains_key("shadow_only_override") {
        bool_value(payload.get("shadow_only_override"), true)
    } else {
        signal
            .get("shadow_only")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    let blocked_hard = blocked && !shadow_only;

    Ok(json!({
        "ok": !blocked_hard,
        "available": signal.get("available").and_then(Value::as_bool).unwrap_or(false),
        "blocked": blocked,
        "blocked_hard": blocked_hard,
        "shadow_violation": blocked && shadow_only,
        "shadow_only": shadow_only,
        "reason_codes": reasons,
        "requested_depth": requested_depth,
        "requested_unbounded": require_unbounded,
        "allowed_depth": allowed_depth,
        "unbounded_allowed": unbounded_allowed,
        "coherence_score": signal.get("coherence_score").and_then(Value::as_f64),
        "coherence_tier": signal.get("coherence_tier").cloned().unwrap_or(Value::Null),
        "sustained_high_samples": signal
            .get("recursion_gate")
            .and_then(|v| v.get("sustained_high_samples"))
            .and_then(Value::as_i64),
        "latest_path_rel": signal.get("latest_path_rel").cloned().unwrap_or_else(|| {
            signal
                .get("source_paths")
                .and_then(|v| v.get("latest_path"))
                .cloned()
                .unwrap_or(Value::Null)
        })
    }))
}

fn profile_state_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_path(
        root,
        payload.get("profile_path"),
        "local/state/symbiosis/coherence/profile_state.json",
    )
}

fn default_profile_state() -> Value {
    json!({
        "version": "1.0",
        "updated_at": now_iso(),
        "settings": {
            "tone": "collaborative",
            "depth": 0.62,
            "initiative": 0.54,
            "tool_aggressiveness": 0.34,
            "response_style": "balanced",
            "detail_level": "standard",
            "proactivity_tolerance": "medium",
            "risk_appetite": "balanced"
        },
        "edit_later_path": profile_edit_later_path(),
        "deltas": []
    })
}

fn profile_edit_later_path() -> &'static str {
    "protheus-ops symbiosis-coherence-kernel profile-update --payload='{\"response_style\":\"direct\",\"detail_level\":\"detailed\",\"proactivity_tolerance\":\"high\",\"risk_appetite\":\"cautious\"}'"
}

fn normalize_choice(raw: Option<&Value>, fallback: &str, allowed: &[&str]) -> String {
    let token = normalize_token(raw, 32);
    if token.is_empty() {
        return fallback.to_string();
    }
    if allowed.iter().any(|item| *item == token) {
        token
    } else {
        fallback.to_string()
    }
}

fn normalize_response_style(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["direct", "balanced", "coaching"])
}

fn normalize_detail_level(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["concise", "standard", "detailed"])
}

fn normalize_proactivity_tolerance(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["low", "medium", "high"])
}

fn normalize_risk_appetite(raw: Option<&Value>, fallback: &str) -> String {
    normalize_choice(raw, fallback, &["cautious", "balanced", "aggressive"])
}

fn profile_checklist(settings: &Value) -> Value {
    let response_style = normalize_response_style(settings.get("response_style"), "balanced");
    let detail_level = normalize_detail_level(settings.get("detail_level"), "standard");
    let proactivity_tolerance =
        normalize_proactivity_tolerance(settings.get("proactivity_tolerance"), "medium");
    let risk_appetite = normalize_risk_appetite(settings.get("risk_appetite"), "balanced");
    json!([
        {
            "key": "response_style",
            "label": "Response style",
            "current": response_style,
            "options": ["direct", "balanced", "coaching"],
            "description": "Controls concise-vs-guided response tone."
        },
        {
            "key": "detail_level",
            "label": "Detail level",
            "current": detail_level,
            "options": ["concise", "standard", "detailed"],
            "description": "Controls default answer depth."
        },
        {
            "key": "proactivity_tolerance",
            "label": "Proactivity tolerance",
            "current": proactivity_tolerance,
            "options": ["low", "medium", "high"],
            "description": "Controls how often proactive suggestions are surfaced."
        },
        {
            "key": "risk_appetite",
            "label": "Risk appetite",
            "current": risk_appetite,
            "options": ["cautious", "balanced", "aggressive"],
            "description": "Controls conservative vs bold default suggestions."
        }
    ])
}

fn normalize_tone(raw: Option<&Value>, fallback: &str) -> String {
    let token = normalize_token(raw, 32);
    if token.is_empty() {
        return fallback.to_string();
    }
    match token.as_str() {
        "direct" | "concise" => "direct".to_string(),
        "neutral" | "balanced" => "neutral".to_string(),
        "collaborative" | "supportive" => "collaborative".to_string(),
        _ => fallback.to_string(),
    }
}

fn signed_delta(value: Option<&Value>, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback).clamp(-0.35, 0.35)
}

fn load_profile_state(path: &Path) -> Value {
    let loaded = read_json_value(path, default_profile_state());
    if loaded.is_object() {
        loaded
    } else {
        default_profile_state()
    }
}

fn profile_summary(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let state = load_profile_state(&path);
    let settings = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    let checklist = profile_checklist(&settings);
    let deltas = state
        .get("deltas")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_summary",
        "path_rel": rel_path(root, &path),
        "settings": settings,
        "first_run_checklist": checklist,
        "edit_later_path": state
            .get("edit_later_path")
            .cloned()
            .unwrap_or_else(|| Value::String(profile_edit_later_path().to_string())),
        "delta_count": deltas.len(),
        "last_delta": deltas.last().cloned().unwrap_or(Value::Null),
        "updated_at": state.get("updated_at").cloned().unwrap_or(Value::Null)
    }))
}

fn profile_update(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let mut state = load_profile_state(&path);
    let existing = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    let existing_tone = normalize_tone(existing.get("tone"), "collaborative");
    let existing_depth = clamp_number(existing.get("depth"), 0.0, 1.0, 0.62);
    let existing_initiative = clamp_number(existing.get("initiative"), 0.0, 1.0, 0.54);
    let existing_tool = clamp_number(existing.get("tool_aggressiveness"), 0.0, 1.0, 0.34);
    let existing_response_style =
        normalize_response_style(existing.get("response_style"), "balanced");
    let existing_detail_level = normalize_detail_level(existing.get("detail_level"), "standard");
    let existing_proactivity = normalize_proactivity_tolerance(
        existing.get("proactivity_tolerance"),
        "medium",
    );
    let existing_risk = normalize_risk_appetite(existing.get("risk_appetite"), "balanced");

    let explicit = payload.get("explicit_feedback").and_then(Value::as_object);
    let implicit = payload.get("interaction_signals").and_then(Value::as_object);

    let target_tone = normalize_tone(
        payload
            .get("tone")
            .or_else(|| explicit.and_then(|row| row.get("tone"))),
        existing_tone.as_str(),
    );
    let response_style = normalize_response_style(
        payload
            .get("response_style")
            .or_else(|| explicit.and_then(|row| row.get("response_style"))),
        existing_response_style.as_str(),
    );
    let detail_level = normalize_detail_level(
        payload
            .get("detail_level")
            .or_else(|| explicit.and_then(|row| row.get("detail_level"))),
        existing_detail_level.as_str(),
    );
    let proactivity_tolerance = normalize_proactivity_tolerance(
        payload
            .get("proactivity_tolerance")
            .or_else(|| explicit.and_then(|row| row.get("proactivity_tolerance"))),
        existing_proactivity.as_str(),
    );
    let risk_appetite = normalize_risk_appetite(
        payload
            .get("risk_appetite")
            .or_else(|| explicit.and_then(|row| row.get("risk_appetite"))),
        existing_risk.as_str(),
    );
    let depth = (
        clamp_number(
            payload
                .get("depth")
                .or_else(|| explicit.and_then(|row| row.get("depth"))),
            0.0,
            1.0,
            existing_depth,
        ) + signed_delta(explicit.and_then(|row| row.get("depth_delta")), 0.0)
            + signed_delta(implicit.and_then(|row| row.get("depth_delta")), 0.0)
    )
    .clamp(0.0, 1.0);
    let initiative = (
        clamp_number(
            payload
                .get("initiative")
                .or_else(|| explicit.and_then(|row| row.get("initiative"))),
            0.0,
            1.0,
            existing_initiative,
        ) + signed_delta(explicit.and_then(|row| row.get("initiative_delta")), 0.0)
            + signed_delta(implicit.and_then(|row| row.get("initiative_delta")), 0.0)
    )
    .clamp(0.0, 1.0);
    let tool_aggressiveness = (
        clamp_number(
            payload
                .get("tool_aggressiveness")
                .or_else(|| explicit.and_then(|row| row.get("tool_aggressiveness"))),
            0.0,
            1.0,
            existing_tool,
        ) + signed_delta(
            explicit.and_then(|row| row.get("tool_aggressiveness_delta")),
            0.0,
        ) + signed_delta(
            implicit.and_then(|row| row.get("tool_aggressiveness_delta")),
            0.0,
        )
    )
    .clamp(0.0, 1.0);
    let now = now_iso();
    let next_settings = json!({
        "tone": target_tone,
        "depth": round_to(depth, 4),
        "initiative": round_to(initiative, 4),
        "tool_aggressiveness": round_to(tool_aggressiveness, 4),
        "response_style": response_style,
        "detail_level": detail_level,
        "proactivity_tolerance": proactivity_tolerance,
        "risk_appetite": risk_appetite
    });

    let delta = json!({
        "ts": now,
        "source": normalize_token(payload.get("source"), 64),
        "from": existing,
        "to": next_settings,
        "explicit_feedback": explicit.cloned().map(Value::Object).unwrap_or(Value::Null),
        "interaction_signals": implicit.cloned().map(Value::Object).unwrap_or(Value::Null)
    });
    let mut deltas = state
        .get("deltas")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    deltas.push(delta.clone());
    let start = deltas.len().saturating_sub(120);
    deltas = deltas[start..].to_vec();

    state["version"] = Value::String("1.0".to_string());
    state["updated_at"] = Value::String(now.clone());
    state["edit_later_path"] = Value::String(profile_edit_later_path().to_string());
    state["settings"] = next_settings.clone();
    state["deltas"] = Value::Array(deltas);
    write_json(&path, &state)?;

    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_update",
        "path_rel": rel_path(root, &path),
        "settings": next_settings,
        "delta": delta,
        "first_run_checklist": profile_checklist(&state["settings"]),
        "edit_later_path": profile_edit_later_path()
    }))
}

fn profile_reset(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let mut state = default_profile_state();
    state["updated_at"] = Value::String(now_iso());
    state["edit_later_path"] = Value::String(profile_edit_later_path().to_string());
    write_json(&path, &state)?;
    let settings = state.get("settings").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_reset",
        "path_rel": rel_path(root, &path),
        "settings": settings.clone(),
        "first_run_checklist": profile_checklist(&settings),
        "edit_later_path": profile_edit_later_path()
    }))
}

fn profile_checklist_cmd(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let state = load_profile_state(&path);
    let settings = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_checklist",
        "path_rel": rel_path(root, &path),
        "settings": settings.clone(),
        "first_run_checklist": profile_checklist(&settings),
        "edit_later_path": state
            .get("edit_later_path")
            .cloned()
            .unwrap_or_else(|| Value::String(profile_edit_later_path().to_string()))
    }))
}

fn with_execution_receipt(command: &str, status: &str, payload: Value) -> Value {
    json!({
        "execution_receipt": {
            "lane": "symbiosis_coherence_kernel",
            "command": command,
            "status": status,
            "source": "OPENCLAW-TOOLING-WEB-098",
            "tool_runtime_class": "receipt_wrapped"
        },
        "payload": payload
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };

    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("symbiosis_coherence_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();

    let result = match command {
        "load-policy" => Ok(json!({
            "ok": true,
            "policy": load_policy(root, &payload)
        })),
        "evaluate" => evaluate_signal(root, &payload),
        "load" => load_signal(root, &payload),
        "recursion-request" => recursion_request(root, &payload),
        "profile-summary" => profile_summary(root, &payload),
        "profile-update" => profile_update(root, &payload),
        "profile-reset" => profile_reset(root, &payload),
        "profile-checklist" => profile_checklist_cmd(root, &payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err("symbiosis_coherence_kernel_unknown_command".to_string()),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                "symbiosis_coherence_kernel",
                with_execution_receipt(command, "success", payload),
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_receipt(
                "symbiosis_coherence_kernel",
                with_execution_receipt(
                    command,
                    "error",
                    json!({
                        "ok": false,
                        "error": err,
                        "error_kind": "command_failed",
                        "retryable": false
                    }),
                ),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, value: &Value) {
        let path = root.join(rel);
        lane_utils::write_json(&path, value).unwrap();
    }

    #[test]
    fn evaluate_signal_persists_latest_and_recursion_gate() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let policy_path = root.join("client/runtime/config/symbiosis_coherence_policy.json");
        write(
            root,
            "client/runtime/config/symbiosis_coherence_policy.json",
            &json!({
                "version": "1.0",
                "shadow_only": true,
                "paths": {
                    "state_path": "local/state/symbiosis/coherence/state.json",
                    "latest_path": "local/state/symbiosis/coherence/latest.json",
                    "receipts_path": "local/state/symbiosis/coherence/receipts.jsonl",
                    "identity_latest_path": "local/state/autonomy/identity_anchor/latest.json",
                    "pre_neuralink_state_path": "local/state/symbiosis/pre_neuralink_interface/state.json",
                    "deep_symbiosis_state_path": "local/state/symbiosis/deep_understanding/state.json",
                    "observer_mirror_latest_path": "local/state/autonomy/observer_mirror/latest.json"
                }
            }),
        );
        write(
            root,
            "local/state/autonomy/identity_anchor/latest.json",
            &json!({"summary":{"identity_drift_score":0.12,"max_identity_drift_score":0.58,"blocked":0,"checked":10}}),
        );
        write(
            root,
            "local/state/symbiosis/pre_neuralink_interface/state.json",
            &json!({"consent_state":"granted","signals_total":20,"routed_total":18,"blocked_total":1}),
        );
        write(
            root,
            "local/state/symbiosis/deep_understanding/state.json",
            &json!({"samples":60,"style":{"directness":0.9,"brevity":0.8,"proactive_delta":0.85}}),
        );
        write(
            root,
            "local/state/autonomy/observer_mirror/latest.json",
            &json!({"observer":{"mood":"stable"},"summary":{"rates":{"ship_rate":0.8,"hold_rate":0.1}}}),
        );

        let payload = json!({
            "policy_path": policy_path,
            "persist": true
        });
        let out = evaluate_signal(root, payload.as_object().unwrap()).unwrap();
        assert_eq!(out["available"], Value::Bool(true));
        assert!(out["coherence_score"].as_f64().unwrap() > 0.7);
        assert!(out["recursion_gate"]["allowed_depth"].as_i64().unwrap() >= 3);
        assert!(root
            .join("local/state/symbiosis/coherence/latest.json")
            .exists());
    }

    #[test]
    fn recursion_request_flags_depth_violation() {
        let signal = json!({
            "available": true,
            "shadow_only": true,
            "coherence_score": 0.82,
            "coherence_tier": "high",
            "latest_path_rel": "local/state/symbiosis/coherence/latest.json",
            "recursion_gate": {
                "allowed_depth": 4,
                "unbounded_allowed": false,
                "sustained_high_samples": 3
            }
        });
        let dir = tempdir().unwrap();
        let payload = json!({
            "signal": signal,
            "requested_depth": 7
        });
        let out = recursion_request(dir.path(), payload.as_object().unwrap()).unwrap();
        assert_eq!(out["blocked"], Value::Bool(true));
        assert_eq!(out["blocked_hard"], Value::Bool(false));
        assert!(out["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "symbiosis_depth_exceeds_allowed"));
    }

    #[test]
    fn profile_update_and_reset_are_receipted_and_persisted() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let update_payload = json!({
            "source": "feedback",
            "explicit_feedback": {
                "tone": "direct",
                "depth_delta": 0.1,
                "initiative_delta": -0.05
            },
            "interaction_signals": {
                "tool_aggressiveness_delta": 0.08
            }
        });
        let updated = profile_update(root, update_payload.as_object().unwrap()).unwrap();
        assert_eq!(updated["ok"], Value::Bool(true));
        assert_eq!(
            updated["settings"]["tone"].as_str(),
            Some("direct")
        );

        let summary = profile_summary(root, &Map::new()).unwrap();
        assert_eq!(summary["ok"], Value::Bool(true));
        assert!(summary["delta_count"].as_u64().unwrap_or(0) >= 1);

        let reset = profile_reset(root, &Map::new()).unwrap();
        assert_eq!(reset["ok"], Value::Bool(true));
        assert_eq!(
            reset["settings"]["tone"].as_str(),
            Some("collaborative")
        );
    }
}
