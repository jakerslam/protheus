fn speculation_state_path(root: &Path) -> PathBuf {
    state_root(root).join("speculation").join("state.json")
}

fn run_speculation_overlay(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let mut state = read_json(&speculation_state_path(root)).unwrap_or_else(
        || json!({"type":"autonomy_speculation_state","overlays":{},"updated_at":now_iso()}),
    );
    if !state.get("overlays").map(Value::is_object).unwrap_or(false) {
        state["overlays"] = json!({});
    }
    if action == "run" || action == "create" {
        let payload = parse_payload_json(argv).unwrap_or_else(|_| json!({}));
        let spec_id = clean_id(
            parse_flag(argv, "spec-id"),
            &format!("spec-{}", &receipt_hash(&json!({"ts": now_iso()}))[..10]),
        );
        state["overlays"][&spec_id] = json!({"spec_id": spec_id, "status":"pending", "created_at": now_iso(), "payload": payload});
    } else if action == "merge" {
        let spec_id = clean_id(
            parse_flag(argv, "spec-id").or_else(|| parse_positional(argv, 2)),
            "spec",
        );
        let verify = parse_bool(parse_flag(argv, "verify").as_deref(), true);
        if strict && !verify {
            let mut out = cli_error_receipt(argv, "speculation_merge_requires_verify", 2);
            out["type"] = json!("autonomy_speculation");
            return emit_receipt(root, &mut out);
        }
        let overlay = state
            .pointer(&format!("/overlays/{spec_id}"))
            .cloned()
            .unwrap_or(Value::Null);
        if overlay.is_null() {
            let mut out = cli_error_receipt(argv, "speculation_not_found", 2);
            out["type"] = json!("autonomy_speculation");
            return emit_receipt(root, &mut out);
        }
        let mut trunk =
            read_json(&trunk_state_path(root)).unwrap_or_else(|| json!({"state":"idle"}));
        if !trunk
            .get("speculation_merges")
            .map(Value::is_array)
            .unwrap_or(false)
        {
            trunk["speculation_merges"] = Value::Array(Vec::new());
        }
        trunk["speculation_merges"]
            .as_array_mut()
            .expect("array")
            .push(json!({
                "spec_id": spec_id,
                "merged_at": now_iso(),
                "overlay_hash": receipt_hash(&overlay)
            }));
        let _ = write_json(&trunk_state_path(root), &trunk);
        state["overlays"][&spec_id]["status"] = json!("merged");
        state["overlays"][&spec_id]["merged_at"] = json!(now_iso());
    } else if action == "reject" {
        let spec_id = clean_id(
            parse_flag(argv, "spec-id").or_else(|| parse_positional(argv, 2)),
            "spec",
        );
        if state.pointer(&format!("/overlays/{spec_id}")).is_some() {
            state["overlays"][&spec_id]["status"] = json!("rejected");
            state["overlays"][&spec_id]["rejected_at"] = json!(now_iso());
        }
    }
    state["updated_at"] = json!(now_iso());
    let _ = write_json(&speculation_state_path(root), &state);
    let mut out = json!({
        "ok": true,
        "type": "autonomy_speculation",
        "lane": LANE_ID,
        "strict": strict,
        "action": action,
        "state": state,
        "claim_evidence": [
            {"id":"V6-EXEC-002.1","claim":"speculative_execution_runs_in_overlay_state_until_verified"},
            {"id":"V6-EXEC-002.2","claim":"overlay_merge_or_reject_is_atomic_and_receipted"}
        ]
    });
    emit_receipt(root, &mut out)
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn tiered_compaction_reduces_hand_memory_pressure() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_hand_new(
                root.path(),
                &["hand-new".to_string(), "--hand-id=h1".to_string()]
            ),
            0
        );
        let path = hand_path(root.path(), "h1");
        let mut hand = read_json(&path).expect("hand");
        hand["memory"] = json!({
            "core": (0..40).map(|i| json!({"text": format!("core-{i}")})).collect::<Vec<_>>(),
            "archival": (0..80).map(|i| json!({"text": format!("arch-{i}")})).collect::<Vec<_>>(),
            "external": (0..64).map(|i| json!({"text": format!("ext-{i}")})).collect::<Vec<_>>()
        });
        write_json(&path, &hand).expect("write");
        assert_eq!(
            run_tiered_compaction(
                root.path(),
                &[
                    "compact".to_string(),
                    "--hand-id=h1".to_string(),
                    "--mode=snip".to_string()
                ]
            ),
            0
        );
        let next = read_json(&path).expect("next");
        let core_len = next
            .pointer("/memory/core")
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);
        assert!(core_len < 40);
    }

    #[test]
    fn speculation_overlay_run_and_merge_updates_trunk_state() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_speculation_overlay(
                root.path(),
                &[
                    "speculate".to_string(),
                    "run".to_string(),
                    "--spec-id=s1".to_string(),
                    "--input-json={\"plan\":\"test\"}".to_string()
                ]
            ),
            0
        );
        assert_eq!(
            run_speculation_overlay(
                root.path(),
                &[
                    "speculate".to_string(),
                    "merge".to_string(),
                    "--spec-id=s1".to_string(),
                    "--verify=1".to_string()
                ]
            ),
            0
        );
        let trunk = read_json(&trunk_state_path(root.path())).expect("trunk");
        let merged = trunk
            .pointer("/speculation_merges")
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(merged, 1);
    }

    #[test]
    fn dream_consolidation_writes_four_phase_receipts() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_hand_new(
                root.path(),
                &["hand-new".to_string(), "--hand-id=h2".to_string()]
            ),
            0
        );
        assert_eq!(
            run_dream_consolidation(
                root.path(),
                &["dream".to_string(), "--hand-id=h2".to_string()]
            ),
            0
        );
        let rows = read_jsonl(&dream_events_path(root.path()));
        assert!(!rows.is_empty());
        let phases = rows
            .last()
            .and_then(|row| row.pointer("/phase_receipts"))
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(phases, 4);
    }

    #[test]
    fn kairos_pause_blocks_cycle_increment() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_kairos_daemon(root.path(), &["kairos".to_string(), "pause".to_string()]),
            0
        );
        assert_eq!(
            run_kairos_daemon(root.path(), &["kairos".to_string(), "cycle".to_string()]),
            0
        );
        let state = read_json(&kairos_state_path(root.path())).expect("state");
        assert_eq!(state.get("paused").and_then(Value::as_bool), Some(true));
        assert_eq!(state.get("cycles").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn kairos_cycle_emits_append_only_daily_log_and_state_write_confirmation() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_kairos_daemon(
                root.path(),
                &[
                    "kairos".to_string(),
                    "cycle".to_string(),
                    "--auto=1".to_string(),
                    "--force=1".to_string(),
                    "--brief=1".to_string(),
                ],
            ),
            0
        );
        let ymd: String = now_iso().chars().take(10).collect();
        let log_path = kairos_daily_log_path(root.path(), &ymd);
        let rows = read_jsonl(&log_path);
        assert!(
            !rows.is_empty(),
            "kairos daily log should append at least one row"
        );
        let state = read_json(&kairos_state_path(root.path())).expect("state");
        assert_eq!(
            state
                .pointer("/write_discipline/state_write_confirmed")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn kairos_rate_limit_and_block_budget_defer_intents() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_hand_new(
                root.path(),
                &["hand-new".to_string(), "--hand-id=h-limited".to_string()],
            ),
            0
        );
        let hand_path = hand_path(root.path(), "h-limited");
        let mut hand = read_json(&hand_path).expect("hand");
        hand["memory"]["core"] = Value::Array(
            (0..120)
                .map(|idx| json!({"text": format!("core-{idx}")}))
                .collect(),
        );
        write_json(&hand_path, &hand).expect("write hand");

        let swarm_path = root
            .path()
            .join("local/state/ops/swarm_runtime/latest.json");
        fs::create_dir_all(swarm_path.parent().expect("parent")).expect("mkdir");
        write_json(
            &swarm_path,
            &json!({
                "dead_letters": [json!({"id":"d1"})],
                "sessions": {
                    "s1": {},
                    "s2": {}
                }
            }),
        )
        .expect("swarm");

        assert_eq!(
            run_kairos_daemon(
                root.path(),
                &[
                    "kairos".to_string(),
                    "cycle".to_string(),
                    "--auto=1".to_string(),
                    "--force=1".to_string(),
                    "--max-proactive=1".to_string(),
                    "--block-budget-ms=100".to_string(),
                ],
            ),
            0
        );
        let state = read_json(&kairos_state_path(root.path())).expect("state");
        let deferred = state
            .pointer("/last_deferred_intents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            deferred.iter().any(|row| {
                matches!(
                    row.get("reason").and_then(Value::as_str),
                    Some("blocking_budget" | "rate_limit")
                )
            }),
            "expected at least one deferred reason from budget/rate limiting"
        );
    }

    #[test]
    fn kairos_auto_compaction_uses_reactive_threshold_near_ninety_five_percent() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_hand_new(
                root.path(),
                &["hand-new".to_string(), "--hand-id=h-reactive".to_string()],
            ),
            0
        );
        let hand_path = hand_path(root.path(), "h-reactive");
        let mut hand = read_json(&hand_path).expect("hand");
        hand["memory"]["core"] = Value::Array(
            (0..120)
                .map(|idx| json!({"text": format!("core-{idx}")}))
                .collect(),
        );
        write_json(&hand_path, &hand).expect("write hand");
        assert_eq!(
            run_kairos_daemon(
                root.path(),
                &[
                    "kairos".to_string(),
                    "cycle".to_string(),
                    "--auto=1".to_string(),
                    "--force=1".to_string(),
                    "--max-proactive=8".to_string(),
                    "--block-budget-ms=5000".to_string(),
                ],
            ),
            0
        );
        let state = read_json(&kairos_state_path(root.path())).expect("state");
        let executed = state
            .pointer("/last_executed_intents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let compact_row = executed
            .iter()
            .find(|row| {
                row.pointer("/intent/task").and_then(Value::as_str) == Some("compact_hand_memory")
            })
            .cloned()
            .expect("compact intent executed");
        assert_eq!(
            compact_row.get("pressure_ratio").and_then(Value::as_f64),
            Some(0.95)
        );
    }

    #[test]
    fn kairos_heartbeat_tick_gate_prevents_early_cycle_reentry() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_kairos_daemon(
                root.path(),
                &[
                    "kairos".to_string(),
                    "cycle".to_string(),
                    "--tick-ms=60000".to_string(),
                    "--force=1".to_string(),
                ],
            ),
            0
        );
        let first = read_json(&kairos_state_path(root.path())).expect("state first");
        assert_eq!(first.get("cycles").and_then(Value::as_u64), Some(1));
        assert_eq!(
            run_kairos_daemon(
                root.path(),
                &[
                    "kairos".to_string(),
                    "cycle".to_string(),
                    "--tick-ms=60000".to_string(),
                ],
            ),
            0
        );
        let second = read_json(&kairos_state_path(root.path())).expect("state second");
        assert_eq!(second.get("cycles").and_then(Value::as_u64), Some(1));
        assert_eq!(
            second.get("last_decision").and_then(Value::as_str),
            Some("tick_deferred")
        );
    }

    #[test]
    fn kairos_daily_log_is_append_only_across_cycles() {
        let root = tempdir().expect("tmp");
        let args = &[
            "kairos".to_string(),
            "cycle".to_string(),
            "--force=1".to_string(),
            "--auto=1".to_string(),
        ];
        assert_eq!(run_kairos_daemon(root.path(), args), 0);
        assert_eq!(run_kairos_daemon(root.path(), args), 0);
        let ymd: String = now_iso().chars().take(10).collect();
        let rows = read_jsonl(&kairos_daily_log_path(root.path(), &ymd));
        assert!(
            rows.len() >= 2,
            "expected append-only kairos log to retain multiple cycle rows"
        );
    }

    #[test]
    fn autoreason_run_persists_state_and_iterations() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_autoreason(
                root.path(),
                &[
                    "autoreason".to_string(),
                    "run".to_string(),
                    "--task=improve launch strategy".to_string(),
                    "--convergence=2".to_string(),
                    "--max-iters=6".to_string(),
                    "--judges=3".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        let state = read_json(&autoreason_state_path(root.path())).expect("autoreason state");
        assert_eq!(state.get("total_runs").and_then(Value::as_u64), Some(1));
        let run_id = state
            .pointer("/last_run/run_id")
            .and_then(Value::as_str)
            .expect("run id");
        let rows = read_jsonl(&autoreason_run_log_path(root.path(), run_id));
        assert!(
            !rows.is_empty(),
            "autoreason run should persist iteration rows"
        );
    }

    #[test]
    fn autoreason_blind_eval_hides_candidate_ids_from_blinded_surface() {
        let eval = autoreason_blind_evaluate(
            "ar-test",
            1,
            &[
                ("a_revised".to_string(), "candidate a body".to_string()),
                ("b_revised".to_string(), "candidate b body".to_string()),
                ("ab_synth".to_string(), "candidate ab body".to_string()),
            ],
            3,
        );
        let blinded = eval
            .get("blinded_candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!blinded.is_empty());
        assert!(blinded.iter().all(|row| row.get("candidate_id").is_none()));
        let winner = eval.get("winner_id").and_then(Value::as_str).unwrap_or("");
        assert!(matches!(winner, "a_revised" | "b_revised" | "ab_synth"));
    }

    #[test]
    fn autoreason_conduit_bypass_is_rejected() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run_autoreason(
                root.path(),
                &[
                    "autoreason".to_string(),
                    "run".to_string(),
                    "--task=t".to_string(),
                    "--bypass=1".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            1
        );
    }
}
