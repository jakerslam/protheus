
fn dream_warden_resolve_runtime_path(
    repo_root: &Path,
    candidate: Option<&str>,
    fallback: &str,
) -> String {
    let chosen = candidate.unwrap_or(fallback).trim();
    let safe = if chosen.is_empty()
        || chosen.contains('\0')
        || chosen.contains('\n')
        || chosen.contains('\r')
        || chosen.contains("..")
    {
        fallback
    } else {
        chosen
    };
    resolve_runtime_path(repo_root, safe)
        .to_string_lossy()
        .to_string()
}

fn dream_warden_load_policy(repo_root: &Path, policy_path: &Path) -> Value {
    let raw = read_json_or(policy_path, json!({}));
    let mut policy = dream_warden_default_policy();
    if let Some(version) = raw.get("version").and_then(Value::as_str) {
        policy["version"] = Value::String(clean_text(version, 40));
    }
    for key in ["enabled", "shadow_only", "passive_only"] {
        if let Some(v) = raw.get(key).and_then(Value::as_bool) {
            policy[key] = Value::Bool(v);
        }
    }
    if let Some(activation) = raw.get("activation").and_then(Value::as_object) {
        if activation
            .get("min_successful_self_improvement_cycles")
            .is_some()
        {
            let n = number_i64(
                activation.get("min_successful_self_improvement_cycles"),
                5,
                0,
                100_000,
            );
            policy["activation"]["min_successful_self_improvement_cycles"] =
                Value::Number(n.into());
        }
        if activation.get("min_symbiosis_score").is_some() {
            let n = number_f64(activation.get("min_symbiosis_score"), 0.82, 0.0, 1.0);
            policy["activation"]["min_symbiosis_score"] = json!(n);
        }
        if activation.get("min_hours_between_runs").is_some() {
            let n = number_i64(activation.get("min_hours_between_runs"), 1, 0, 720);
            policy["activation"]["min_hours_between_runs"] = Value::Number(n.into());
        }
    }
    if let Some(thresholds) = raw.get("thresholds").and_then(Value::as_object) {
        for (key, fallback, lo, hi) in [
            ("critical_fail_cases_trigger", 1.0, 0.0, 100_000.0),
            ("red_team_fail_rate_trigger", 0.15, 0.0, 1.0),
            ("mirror_hold_rate_trigger", 0.4, 0.0, 1.0),
            ("low_symbiosis_score_trigger", 0.75, 0.0, 1.0),
            ("max_patch_candidates", 6.0, 1.0, 64.0),
        ] {
            if let Some(v) = thresholds.get(key) {
                if key == "critical_fail_cases_trigger" || key == "max_patch_candidates" {
                    let n = number_i64(Some(v), fallback as i64, lo as i64, hi as i64);
                    policy["thresholds"][key] = Value::Number(n.into());
                } else {
                    let n = number_f64(Some(v), fallback, lo, hi);
                    policy["thresholds"][key] = json!(n);
                }
            }
        }
    }
    if let Some(signals) = raw.get("signals").and_then(Value::as_object) {
        for key in [
            "collective_shadow_latest_path",
            "observer_mirror_latest_path",
            "red_team_latest_path",
            "symbiosis_latest_path",
            "gated_self_improvement_state_path",
        ] {
            let fallback = policy["signals"][key].as_str().unwrap_or_default();
            policy["signals"][key] = Value::String(dream_warden_resolve_runtime_path(
                repo_root,
                signals.get(key).and_then(Value::as_str),
                fallback,
            ));
        }
    }
    if let Some(outputs) = raw.get("outputs").and_then(Value::as_object) {
        for key in [
            "latest_path",
            "history_path",
            "receipts_path",
            "patch_proposals_path",
            "ide_events_path",
        ] {
            let fallback = policy["outputs"][key].as_str().unwrap_or_default();
            policy["outputs"][key] = Value::String(dream_warden_resolve_runtime_path(
                repo_root,
                outputs.get(key).and_then(Value::as_str),
                fallback,
            ));
        }
    }
    policy
}

fn dream_warden_count_successful_cycles(gsi_state: &Value) -> i64 {
    let mut count = 0_i64;
    let proposals = gsi_state
        .get("proposals")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for row in proposals.values() {
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(status.as_str(), "gated_pass" | "live_ready" | "live_merged") {
            count += 1;
        }
    }
    count
}

fn dream_warden_last_run_info(history_path: &Path) -> (Option<String>, Option<f64>) {
    let rows = read_jsonl(history_path);
    let last = rows.last().cloned().unwrap_or_else(|| json!({}));
    let last_ts = last
        .get("ts")
        .and_then(Value::as_str)
        .map(|v| v.to_string());
    let hours_since = last_ts
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| {
            let now = Utc::now().timestamp_millis() as f64;
            let then = v.timestamp_millis() as f64;
            (now - then) / 3_600_000.0
        });
    (last_ts, hours_since)
}

fn dream_warden_patch_proposals(policy: &Value, signals: &Value, run_id: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let max = number_i64(
        policy
            .get("thresholds")
            .and_then(|v| v.get("max_patch_candidates")),
        6,
        1,
        64,
    ) as usize;
    let critical_fail = signals
        .get("collective_shadow")
        .and_then(|v| v.get("red_team"))
        .and_then(|v| v.get("critical_fail_cases"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let red_fail = signals
        .get("red_team")
        .and_then(|v| v.get("summary"))
        .and_then(|v| v.get("fail_rate"))
        .and_then(Value::as_f64)
        .or_else(|| {
            signals
                .get("collective_shadow")
                .and_then(|v| v.get("red_team"))
                .and_then(|v| v.get("fail_rate"))
                .and_then(Value::as_f64)
        })
        .unwrap_or(0.0);
    let hold_rate = signals
        .get("observer_mirror")
        .and_then(|v| v.get("summary"))
        .and_then(|v| v.get("rates"))
        .and_then(|v| v.get("hold_rate"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let coherence = signals
        .get("symbiosis")
        .and_then(|v| v.get("coherence_score"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let critical_trigger = number_i64(
        policy
            .get("thresholds")
            .and_then(|v| v.get("critical_fail_cases_trigger")),
        1,
        0,
        100_000,
    );
    if critical_fail >= critical_trigger {
        out.push(json!({
            "run_id": run_id,
            "proposal_type": "critical_fail_case_containment",
            "summary": "Strengthen containment around failing red-team surfaces.",
            "priority": "high"
        }));
    }
    let red_trigger = number_f64(
        policy
            .get("thresholds")
            .and_then(|v| v.get("red_team_fail_rate_trigger")),
        0.15,
        0.0,
        1.0,
    );
    if red_fail >= red_trigger {
        out.push(json!({
            "run_id": run_id,
            "proposal_type": "red_team_fail_rate_hardening",
            "summary": "Reduce red-team fail-rate via targeted controls and retries.",
            "priority": "high"
        }));
    }
    let hold_trigger = number_f64(
        policy
            .get("thresholds")
            .and_then(|v| v.get("mirror_hold_rate_trigger")),
        0.4,
        0.0,
        1.0,
    );
    if hold_rate >= hold_trigger {
        out.push(json!({
            "run_id": run_id,
            "proposal_type": "mirror_hold_rate_relief",
            "summary": "Investigate high hold-rate and reduce unnecessary holds.",
            "priority": "medium"
        }));
    }
    let low_sym_trigger = number_f64(
        policy
            .get("thresholds")
            .and_then(|v| v.get("low_symbiosis_score_trigger")),
        0.75,
        0.0,
        1.0,
    );
    if coherence < low_sym_trigger {
        out.push(json!({
            "run_id": run_id,
            "proposal_type": "symbiosis_recovery",
            "summary": "Recover symbiosis coherence before risky adaptations.",
            "priority": "medium"
        }));
    }
    out.truncate(max);
    out
}

pub fn run_dream_warden_guard(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let policy_path = dream_warden_policy_path(repo_root, &args);
    let policy = dream_warden_load_policy(repo_root, &policy_path);
    let outputs = policy
        .get("outputs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let latest_path = PathBuf::from(
        outputs
            .get("latest_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let history_path = PathBuf::from(
        outputs
            .get("history_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let receipts_path = PathBuf::from(
        outputs
            .get("receipts_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let patch_path = PathBuf::from(
        outputs
            .get("patch_proposals_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let ide_path = PathBuf::from(
        outputs
            .get("ide_events_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );

    if cmd == "status" {
        let latest = read_json_or(&latest_path, json!({}));
        return (
            json!({
                "ok": true,
                "type": "dream_warden_status",
                "latest": latest,
                "activation_ready": latest.get("activation_ready").and_then(Value::as_bool).unwrap_or(false),
                "run_id": latest.get("run_id").cloned().unwrap_or(Value::Null)
            }),
            0,
        );
    }
    if cmd != "run" {
        return (
            json!({
                "ok": false,
                "type": "dream_warden_error",
                "error": format!("unknown_command:{cmd}")
            }),
            2,
        );
    }

    let apply_requested = bool_from_str(args.flags.get("apply").map(String::as_str), false);
    let passive_only = policy
        .get("passive_only")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if apply_requested && passive_only {
        let out = json!({
            "ok": false,
            "type": "dream_warden_run",
            "error": "passive_mode_violation_apply_requested",
            "stasis_recommendation": true,
            "passive_only": true
        });
        let _ = write_json_atomic(&latest_path, &out);
        let _ = append_jsonl(&history_path, &out);
        let _ = append_jsonl(&receipts_path, &out);
        return (out, 1);
    }

    let date = date_arg_or_today(args.positional.get(1));
    let run_id = format!(
        "dwd_{}_{}",
        Utc::now().timestamp_millis().to_string(),
        std::process::id()
    );

    let signals = policy
        .get("signals")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let collective_shadow = read_json_or(
        &PathBuf::from(
            signals
                .get("collective_shadow_latest_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        json!({}),
    );
    let observer_mirror = read_json_or(
        &PathBuf::from(
            signals
                .get("observer_mirror_latest_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        json!({}),
    );
    let red_team = read_json_or(
        &PathBuf::from(
            signals
                .get("red_team_latest_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        json!({}),
    );
    let symbiosis = read_json_or(
        &PathBuf::from(
            signals
                .get("symbiosis_latest_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        json!({}),
    );
    let gsi_state = read_json_or(
        &PathBuf::from(
            signals
                .get("gated_self_improvement_state_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        json!({}),
    );
    let successful_cycles = dream_warden_count_successful_cycles(&gsi_state);
    let min_cycles = number_i64(
        policy
            .get("activation")
            .and_then(|v| v.get("min_successful_self_improvement_cycles")),
        5,
        0,
        100_000,
    );
    let coherence_score = symbiosis
        .get("coherence_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let min_symbiosis = number_f64(
        policy
            .get("activation")
            .and_then(|v| v.get("min_symbiosis_score")),
        0.82,
        0.0,
        1.0,
    );
    let min_hours = number_i64(
        policy
            .get("activation")
            .and_then(|v| v.get("min_hours_between_runs")),
        1,
        0,
        720,
    ) as f64;
    let (_last_ts, hours_since_last) = dream_warden_last_run_info(&history_path);
    let throttled = hours_since_last.map(|v| v < min_hours).unwrap_or(false);
    let activation_ready =
        successful_cycles >= min_cycles && coherence_score >= min_symbiosis && !throttled;

    let merged_signals = json!({
        "collective_shadow": collective_shadow,
        "observer_mirror": observer_mirror,
        "red_team": red_team,
        "symbiosis": symbiosis
    });
    let patch_proposals = dream_warden_patch_proposals(&policy, &merged_signals, &run_id);
    let out = json!({
        "ok": true,
        "type": "dream_warden_run",
        "date": date,
        "ts": now_iso(),
        "run_id": run_id,
        "mode": "active_shadow_observer",
        "shadow_only": policy.get("shadow_only").and_then(Value::as_bool).unwrap_or(true),
        "passive_only": passive_only,
        "apply_requested": apply_requested,
        "apply_executed": false,
        "activation_ready": activation_ready,
        "activation": {
            "successful_cycles": successful_cycles,
            "min_successful_cycles": min_cycles,
            "coherence_score": coherence_score,
            "min_symbiosis_score": min_symbiosis,
            "hours_since_last_run": hours_since_last,
            "min_hours_between_runs": min_hours,
            "throttled": throttled
        },
        "patch_proposals_count": patch_proposals.len(),
        "patch_proposals": patch_proposals
    });

    let _ = write_json_atomic(&latest_path, &out);
    let _ = append_jsonl(&history_path, &out);
    let _ = append_jsonl(&receipts_path, &out);
    for row in out
        .get("patch_proposals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let _ = append_jsonl(&patch_path, &row);
        let ide = json!({
            "ts": now_iso(),
            "type": "dream_warden_patch_proposal",
            "run_id": out.get("run_id").cloned().unwrap_or(Value::Null),
            "proposal": row
        });
        let _ = append_jsonl(&ide_path, &ide);
    }

    (out, 0)
}

// -------------------------------------------------------------------------------------------------
// Directive Hierarchy Controller
// -------------------------------------------------------------------------------------------------
