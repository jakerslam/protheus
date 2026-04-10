fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn loop_state_path(root: &Path) -> PathBuf {
    state_root(root).join("loop_state.json")
}

fn recursive_loop_path(root: &Path) -> PathBuf {
    state_root(root).join("recursive_loop.jsonl")
}

fn metacognition_journal_path(root: &Path) -> PathBuf {
    state_root(root).join("metacognition_journal.jsonl")
}

fn network_symbiosis_path(root: &Path) -> PathBuf {
    state_root(root).join("network_symbiosis.jsonl")
}

fn proactive_evolution_path(root: &Path) -> PathBuf {
    state_root(root).join("proactive_evolution.jsonl")
}

fn default_loop_state() -> Value {
    json!({
        "version": "1.0",
        "active": false,
        "drift_score": 0.12,
        "exploration_drive": 0.62,
        "merge_count": 0,
        "rollback_count": 0,
        "proactive_evolution_count": 0,
        "last_merge": null,
        "last_rollback": null,
        "swarm": {
            "nodes": 0,
            "share_rate": 0.0,
            "convergence_score": 0.0
        },
        "created_at": now_iso()
    })
}

fn load_loop_state(root: &Path) -> Value {
    read_json(&loop_state_path(root)).unwrap_or_else(default_loop_state)
}

fn store_loop_state(root: &Path, state: &Value) -> Result<(), String> {
    write_json(&loop_state_path(root), state)
}

fn loop_obj_mut(state: &mut Value) -> &mut Map<String, Value> {
    if !state.is_object() {
        *state = default_loop_state();
    }
    state.as_object_mut().expect("loop_state_object")
}

fn mutation_history_path(root: &Path) -> PathBuf {
    crate::core_state_root(root)
        .join("ops")
        .join("binary_blob_runtime")
        .join("mutation_history.jsonl")
}

fn estimate_recent_failure_rate(root: &Path) -> f64 {
    let path = mutation_history_path(root);
    let Ok(raw) = fs::read_to_string(path) else {
        return 0.0;
    };
    let mut total = 0usize;
    let mut denied = 0usize;
    for line in raw.lines().rev().take(64) {
        let Ok(row) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if row.get("allow").is_some() {
            total += 1;
            if !row.get("allow").and_then(Value::as_bool).unwrap_or(false) {
                denied += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        (denied as f64) / (total as f64)
    }
}

fn simulate_regression(proposal: &str, module: &str) -> f64 {
    let h = sha256_hex_str(&format!("{proposal}:{module}"));
    let tail = &h[h.len().saturating_sub(4)..];
    let n = u64::from_str_radix(tail, 16).unwrap_or(0);
    ((n % 100) as f64) / 1000.0
}

fn maybe_token_reward(root: &Path, agent: &str, amount: f64, reason: &str) -> Value {
    if !directive_kernel::action_allowed(root, "tokenomics") {
        return json!({
            "attempted": false,
            "ok": false,
            "reason": "directive_gate_denied"
        });
    }
    let exit = network_protocol::run(
        root,
        &[
            "reward".to_string(),
            format!("--agent={}", clean(agent, 120)),
            format!("--amount={:.8}", amount.max(0.0)),
            format!("--reason={}", clean(reason, 120)),
        ],
    );
    json!({
        "attempted": true,
        "ok": exit == 0,
        "exit_code": exit
    })
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "rsi_ignition_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 220),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

fn state_entry_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

fn command_status(root: &Path) -> i32 {
    let state = load_loop_state(root);
    emit(
        root,
        json!({
            "ok": true,
            "type": "rsi_ignition_status",
            "lane": "core/layer0/ops",
            "loop_state": state,
            "artifact_counts": {
                "recursive_loop_entries": state_entry_count(&recursive_loop_path(root)),
                "metacognition_entries": state_entry_count(&metacognition_journal_path(root)),
                "network_symbiosis_entries": state_entry_count(&network_symbiosis_path(root)),
                "proactive_evolution_entries": state_entry_count(&proactive_evolution_path(root))
            },
            "latest": read_json(&latest_path(root))
        }),
    )
}
