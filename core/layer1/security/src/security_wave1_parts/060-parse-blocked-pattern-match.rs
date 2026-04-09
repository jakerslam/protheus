
fn parse_blocked_pattern_match(path: &str, pattern: &str) -> bool {
    let norm = path.to_ascii_lowercase();
    let mut pat = pattern.trim().to_ascii_lowercase();
    pat = pat
        .trim_start_matches('^')
        .trim_end_matches('$')
        .replace("\\.", ".")
        .replace("\\(", "(")
        .replace("\\)", ")")
        .replace("(ts|js)", "ts|js");
    if pat.contains("ts|js") {
        let lhs = pat.replace("ts|js", "ts");
        let rhs = pat.replace("ts|js", "js");
        return norm == lhs || norm == rhs;
    }
    norm == pat || norm.contains(&pat.replace(".*", ""))
}

fn proposal_touches_recursive_self_improvement(
    row: &Value,
    mutation_paths: &[String],
    summary: &str,
) -> bool {
    if row.get("recursion_depth").is_some() || row.get("recursion_mode").is_some() {
        return true;
    }
    if row.get("recursion").is_some() {
        return true;
    }
    let target = row
        .get("target_system")
        .or_else(|| row.get("target"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if target.contains("self_improvement")
        || target.contains("self-improvement")
        || target.contains("recursive")
        || target.contains("recursion")
    {
        return true;
    }
    if mutation_paths.iter().any(|v| {
        let lower = v.to_ascii_lowercase();
        lower.contains("self_improvement")
            || lower.contains("self-improvement")
            || lower.contains("self_code_evolution")
            || lower.contains("redteam")
    }) {
        return true;
    }
    summary.contains("recursive self-improvement")
        || summary.contains("unbounded recursion")
        || summary.contains("recursion depth")
        || summary.contains("self-improvement depth")
}

fn evaluate_symbiosis_gate(repo_root: &Path, policy: &Value) -> Value {
    let gate = policy
        .get("symbiosis_recursion_gate")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !gate.get("enabled").and_then(Value::as_bool).unwrap_or(true) {
        return json!({
            "enabled": false,
            "evaluated": false,
            "allowed": true,
            "reason": "gate_disabled"
        });
    }
    let signal_policy_rel = gate
        .get("signal_policy_path")
        .and_then(Value::as_str)
        .unwrap_or("client/runtime/config/symbiosis_coherence_policy.json");
    let signal_policy_path = {
        let p = PathBuf::from(signal_policy_rel);
        if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        }
    };
    let signal_policy = read_json_or(&signal_policy_path, json!({}));
    let paths = signal_policy
        .get("paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let resolve = |key: &str, fallback: &str| -> PathBuf {
        let raw = paths
            .get(key)
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_else(|| fallback.to_string());
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        }
    };

    let identity_latest = read_json_or(
        &resolve(
            "identity_latest_path",
            "client/runtime/local/state/autonomy/identity_anchor/latest.json",
        ),
        json!({}),
    );
    let pre_neural_state = read_json_or(
        &resolve(
            "pre_neuralink_state_path",
            "client/runtime/local/state/symbiosis/pre_neuralink_interface/state.json",
        ),
        json!({}),
    );
    let observer_latest = read_json_or(
        &resolve(
            "observer_mirror_latest_path",
            "client/runtime/local/state/autonomy/observer_mirror/latest.json",
        ),
        json!({}),
    );

    let identity_drift = identity_latest
        .get("max_identity_drift_score")
        .or_else(|| identity_latest.get("identity_drift_score"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let consent_state = pre_neural_state
        .get("consent_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    let hold_rate = observer_latest
        .get("summary")
        .and_then(|v| v.get("rates"))
        .and_then(|v| v.get("hold_rate"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let identity_clear = identity_drift <= 0.5;
    let consent_granted = matches!(consent_state.as_str(), "granted" | "active" | "approved");
    let hold_ok = hold_rate <= 0.65;
    let allowed = identity_clear && consent_granted && hold_ok;

    json!({
        "enabled": true,
        "evaluated": true,
        "allowed": allowed,
        "identity_drift_score": identity_drift,
        "consent_state": consent_state,
        "hold_rate": hold_rate,
        "identity_clear": identity_clear,
        "consent_granted": consent_granted,
        "hold_ok": hold_ok
    })
}

fn goal_preservation_load_proposal(repo_root: &Path, args: &CliArgs) -> Option<Value> {
    if let Some(raw) = args
        .flags
        .get("proposal-json")
        .or_else(|| args.flags.get("proposal_json"))
    {
        if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
            return Some(parsed);
        }
    }
    if let Some(raw) = args
        .flags
        .get("proposal-file")
        .or_else(|| args.flags.get("proposal_file"))
    {
        let path = {
            let p = PathBuf::from(raw);
            if p.is_absolute() {
                p
            } else {
                repo_root.join(p)
            }
        };
        return Some(read_json_or(&path, Value::Null));
    }
    None
}

pub fn run_goal_preservation_kernel(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let policy_path = goal_preservation_policy_path(repo_root, &args);
    let policy = goal_preservation_load_policy(&policy_path);
    let state_path = {
        let raw = policy
            .get("output")
            .and_then(|v| v.get("state_path"))
            .and_then(Value::as_str)
            .unwrap_or("local/state/security/goal_preservation/latest.json");
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            runtime_root(repo_root).join(raw)
        }
    };
    let receipts_path = {
        let raw = policy
            .get("output")
            .and_then(|v| v.get("receipts_path"))
            .and_then(Value::as_str)
            .unwrap_or("local/state/security/goal_preservation/receipts.jsonl");
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            runtime_root(repo_root).join(raw)
        }
    };

    if cmd == "status" {
        let latest = read_json_or(&state_path, json!({}));
        return (
            json!({
                "ok": true,
                "type": "goal_preservation_status",
                "policy_version": policy.get("version").cloned().unwrap_or(Value::String("1.0".to_string())),
                "latest": latest
            }),
            0,
        );
    }

    if cmd != "evaluate" {
        return (
            json!({
                "ok": false,
                "type": "goal_preservation_error",
                "reason": format!("unknown_command:{cmd}")
            }),
            2,
        );
    }

    let proposal = goal_preservation_load_proposal(repo_root, &args).unwrap_or(Value::Null);
    if !proposal.is_object() {
        return (
            json!({
                "ok": false,
                "type": "goal_preservation_evaluate",
                "reason": "proposal_missing_or_invalid"
            }),
            1,
        );
    }
    let proposal_obj = proposal.as_object().cloned().unwrap_or_default();
    let mutation_paths = proposal_obj
        .get("mutation_paths")
        .or_else(|| proposal_obj.get("files"))
        .or_else(|| proposal_obj.get("paths"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(normalize_rel_path)
        .collect::<Vec<_>>();
    let summary = clean_text(
        proposal_obj
            .get("summary")
            .or_else(|| proposal_obj.get("patch_summary"))
            .or_else(|| proposal_obj.get("description"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    )
    .to_ascii_lowercase();
    let mut reasons = Vec::<String>::new();
    let mut advisories = Vec::<String>::new();

    let blocked_patterns = policy
        .get("blocked_mutation_paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    let mut blocked_hits = Vec::new();
    for pat in &blocked_patterns {
        for path in &mutation_paths {
            if parse_blocked_pattern_match(path, pat) {
                blocked_hits.push(path.clone());
            }
        }
    }
    if !blocked_hits.is_empty() {
        reasons.push("blocked_mutation_path".to_string());
    }

    let marker_hits = policy
        .get("protected_axiom_markers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .filter(|marker| summary.contains(&marker.to_ascii_lowercase()))
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    if !marker_hits.is_empty() {
        reasons.push("protected_axiom_marker_touched".to_string());
    }

    let constitution_path = {
        let raw = policy
            .get("constitution_path")
            .and_then(Value::as_str)
            .unwrap_or("docs/workspace/AGENT-CONSTITUTION.md");
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        }
    };
    let constitution_hash = fs::read(&constitution_path)
        .ok()
        .map(|bytes| sha256_hex(&String::from_utf8_lossy(&bytes)));
    let expected_hash = proposal_obj
        .get("expected_constitution_hash")
        .and_then(Value::as_str)
        .map(|v| v.trim().to_ascii_lowercase());
    if let (Some(expected), Some(actual)) = (expected_hash, constitution_hash.clone()) {
        if expected != actual {
            reasons.push("constitution_hash_mismatch".to_string());
        }
    }

    let strict_keywords = [
        "disable constitution",
        "rewrite constitution",
        "bypass user veto",
        "remove user control",
        "disable guard",
        "turn off integrity",
    ];
    if strict_keywords.iter().any(|v| summary.contains(v)) {
        reasons.push("alignment_keyword_violation".to_string());
    }

    let recursive_touch =
        proposal_touches_recursive_self_improvement(&proposal, &mutation_paths, &summary);
    let symbiosis_gate = if recursive_touch {
        evaluate_symbiosis_gate(repo_root, &policy)
    } else {
        json!({
            "enabled": policy
                .get("symbiosis_recursion_gate")
                .and_then(|v| v.get("enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            "evaluated": false,
            "allowed": true,
            "reason": "not_recursive_proposal"
        })
    };
    if recursive_touch
        && symbiosis_gate
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            == false
    {
        reasons.push("symbiosis_recursion_gate_blocked".to_string());
    }

    if recursive_touch {
        advisories.push("recursive_change_detected".to_string());
    }
    let allowed = reasons.is_empty();
    let out = json!({
        "ok": true,
        "type": "goal_preservation_evaluate",
        "ts": now_iso(),
        "allowed": allowed,
        "strict_mode": policy.get("strict_mode").and_then(Value::as_bool).unwrap_or(true),
        "proposal_id": proposal_obj.get("proposal_id").cloned().unwrap_or(Value::Null),
        "mutation_paths": mutation_paths,
        "reasons": reasons,
        "advisories": advisories,
        "marker_hits": marker_hits,
        "blocked_path_hits": blocked_hits,
        "constitution_hash": constitution_hash,
        "symbiosis_recursion_gate": symbiosis_gate
    });
    let _ = write_json_atomic(&state_path, &out);
    let _ = append_jsonl(&receipts_path, &out);
    (out, 0)
}

// -------------------------------------------------------------------------------------------------
// Dream Warden Guard
// -------------------------------------------------------------------------------------------------

fn dream_warden_policy_path(repo_root: &Path, args: &CliArgs) -> PathBuf {
    if let Some(v) = args.flags.get("policy") {
        let p = PathBuf::from(v);
        if p.is_absolute() {
            return p;
        }
        return repo_root.join(p);
    }
    std::env::var("DREAM_WARDEN_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_config_path(repo_root, "dream_warden_policy.json"))
}

fn dream_warden_default_policy() -> Value {
    json!({
        "version": "1.0",
        "enabled": true,
        "shadow_only": true,
        "passive_only": true,
        "activation": {
            "min_successful_self_improvement_cycles": 5,
            "min_symbiosis_score": 0.82,
            "min_hours_between_runs": 1
        },
        "thresholds": {
            "critical_fail_cases_trigger": 1,
            "red_team_fail_rate_trigger": 0.15,
            "mirror_hold_rate_trigger": 0.4,
            "low_symbiosis_score_trigger": 0.75,
            "max_patch_candidates": 6
        },
        "signals": {
            "collective_shadow_latest_path": "local/state/autonomy/collective_shadow/latest.json",
            "observer_mirror_latest_path": "local/state/autonomy/observer_mirror/latest.json",
            "red_team_latest_path": "local/state/security/red_team/latest.json",
            "symbiosis_latest_path": "local/state/symbiosis/coherence/latest.json",
            "gated_self_improvement_state_path": "local/state/autonomy/gated_self_improvement/state.json"
        },
        "outputs": {
            "latest_path": "local/state/security/dream_warden/latest.json",
            "history_path": "local/state/security/dream_warden/history.jsonl",
            "receipts_path": "local/state/security/dream_warden/receipts.jsonl",
            "patch_proposals_path": "local/state/security/dream_warden/patch_proposals.jsonl",
            "ide_events_path": "local/state/security/dream_warden/ide_events.jsonl"
        }
    })
}

fn resolve_runtime_path(repo_root: &Path, raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        runtime_root(repo_root).join(raw)
    }
}
