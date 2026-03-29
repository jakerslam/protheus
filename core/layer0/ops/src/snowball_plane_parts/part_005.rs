fn run_control(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "pause".to_string()),
        20,
    )
    .to_ascii_lowercase();
    if strict && !matches!(op.as_str(), "pause" | "resume" | "abort") {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_control",
            "action": "control",
            "errors": ["snowball_control_op_invalid"],
            "op": op
        });
    }

    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut cycle = cycles_map
        .get(&cycle_id)
        .cloned()
        .unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"running"}));
    cycle["control"] = json!({
        "op": op,
        "ts": crate::now_iso()
    });
    cycle["stage"] = Value::String(match op.as_str() {
        "pause" => "paused".to_string(),
        "resume" => "running".to_string(),
        "abort" => "aborted".to_string(),
        _ => "running".to_string(),
    });
    cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_control",
        "lane": "core/layer0/ops",
        "action": "control",
        "cycle_id": cycle_id,
        "control": cycle.get("control").cloned().unwrap_or(Value::Null),
        "stage": cycle.get("stage").cloned().unwrap_or(Value::String("running".to_string())),
        "claim_evidence": [
            {
                "id": "V6-APP-023.5",
                "claim": "snowball_status_and_controls_are_live_and_receipted_through_conduit",
                "evidence": {
                    "cycle_id": cycle_id,
                    "op": op
                }
            },
            {
                "id": "V6-APP-023.6",
                "claim": "snowball_status_and_compact_controls_surface_cycle_stage_batch_outcomes_and_regression_state",
                "evidence": {
                    "cycle_id": cycle_id,
                    "op": op,
                    "stage": cycle.get("stage").cloned().unwrap_or(Value::Null)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_status(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let cycle = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .and_then(|map| map.get(&cycle_id))
        .cloned();
    let mut out = json!({
        "ok": true,
        "type": "snowball_plane_status",
        "lane": "core/layer0/ops",
        "cycle_id": cycle_id,
        "cycle": cycle,
        "latest_path": latest_path(root).display().to_string(),
        "controls": ["pause", "resume", "abort", "compact"],
        "claim_evidence": [
            {
                "id": "V6-APP-023.5",
                "claim": "snowball_status_and_controls_are_live_and_receipted_through_conduit",
                "evidence": {
                    "active_cycle_id": cycles.get("active_cycle_id").cloned().unwrap_or(Value::Null)
                }
            },
            {
                "id": "V6-APP-023.6",
                "claim": "snowball_status_and_compact_controls_surface_cycle_stage_batch_outcomes_and_regression_state",
                "evidence": {
                    "active_cycle_id": cycles.get("active_cycle_id").cloned().unwrap_or(Value::Null),
                    "has_cycle": cycle.is_some()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn dispatch(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let action = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match action.as_str() {
        "status" => run_status(root, parsed),
        "start" => run_start(root, parsed, strict),
        "melt-refine" | "melt" | "refine" | "regress" => run_melt_refine(root, parsed, strict),
        "compact" => run_compact(root, parsed, strict),
        "fitness-review" => run_fitness_review(root, parsed, strict),
        "archive-discarded" => run_archive_discarded(root, parsed, strict),
        "publish-benchmarks" => run_publish_benchmarks(root, parsed, strict),
        "promote" => run_promote(root, parsed, strict),
        "prime-update" => run_prime_update(root, parsed, strict),
        "backlog-pack" | "backlog" => run_backlog_pack(root, parsed, strict),
        "control" => run_control(root, parsed, strict),
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_error",
            "action": action,
            "errors": ["snowball_action_unknown"]
        }),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let action = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(action.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit_action = if action == "regress" {
        "melt-refine"
    } else {
        action.as_str()
    };
    let conduit = if action != "status" {
        Some(conduit_enforcement(root, &parsed, strict, conduit_action))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "snowball_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = dispatch(root, &parsed, strict);
    if action == "status" {
        print_payload(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_writes_cycle_registry() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_start(
            root.path(),
            &crate::parse_args(&[
                "start".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c17".to_string(),
                "--drops=core-hardening,app-runtime".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(cycles_path(root.path()).exists());
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let gate = conduit_enforcement(
            root.path(),
            &crate::parse_args(&[
                "start".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ]),
            true,
            "start",
        );
        assert_eq!(gate.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn compact_writes_snapshot() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = run_start(
            root.path(),
            &crate::parse_args(&[
                "start".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c18".to_string(),
                "--allow-high-risk=1".to_string(),
            ]),
            true,
        );
        let out = run_compact(
            root.path(),
            &crate::parse_args(&[
                "compact".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c18".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let snap_path = out
            .get("snapshot")
            .and_then(|v| v.get("path"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!snap_path.is_empty());
    }

    #[test]
    fn backlog_pack_orders_items_by_dependencies_then_priority() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = run_start(
            root.path(),
            &crate::parse_args(&[
                "start".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c35".to_string(),
            ]),
            true,
        );
        let unresolved = json!([
            {"id":"deploy","priority":0,"depends_on":["verify"]},
            {"id":"verify","priority":2,"depends_on":[]},
            {"id":"package","priority":1,"depends_on":["verify"]}
        ]);
        let out = run_backlog_pack(
            root.path(),
            &crate::parse_args(&[
                "backlog-pack".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c35".to_string(),
                format!("--unresolved-json={}", unresolved),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let items = out
            .pointer("/backlog/items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let order = items
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(order.first().copied(), Some("verify"));
        let verify_idx = order.iter().position(|id| *id == "verify").expect("verify");
        let package_idx = order
            .iter()
            .position(|id| *id == "package")
            .expect("package");
        let deploy_idx = order.iter().position(|id| *id == "deploy").expect("deploy");
        assert!(verify_idx < package_idx);
        assert!(verify_idx < deploy_idx);
        assert_eq!(
            items
                .first()
                .and_then(|row| row.get("dependency_cycle_break"))
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn compact_scores_assimilation_and_archives_discarded_blob_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        let report_path = root
            .path()
            .join("docs/client/reports/benchmark_matrix_run_2026-03-06.json");
        let report = json!({
            "openclaw_measured": {"cold_start_ms": 5.0, "idle_memory_mb": 9.0, "install_size_mb": 10.0, "tasks_per_sec": 10000.0, "security_systems": 83.0, "channel_adapters": 6.0, "llm_providers": 3.0},
            "pure_workspace_measured": {"cold_start_ms": 4.0, "idle_memory_mb": 1.4, "install_size_mb": 0.7, "tasks_per_sec": 12000.0, "security_systems": 83.0, "channel_adapters": 0.0, "llm_providers": 0.0},
            "pure_workspace_tiny_max_measured": {"cold_start_ms": 3.0, "idle_memory_mb": 1.3, "install_size_mb": 0.5, "tasks_per_sec": 12100.0, "security_systems": 83.0, "channel_adapters": 0.0, "llm_providers": 0.0}
        });
        let _ = write_json(&report_path, &report);
        let _ = run_start(
            root.path(),
            &crate::parse_args(&[
                "start".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c39".to_string(),
            ]),
            true,
        );
        let assimilations = json!([
            {"id":"tiny-allocator","metric_gain":true,"pure_tiny_strength":true,"intelligence_gain":true,"tiny_hardware_fit":true},
            {"id":"big-ui-runtime","metric_gain":false,"pure_tiny_strength":false,"intelligence_gain":false,"tiny_hardware_fit":false}
        ]);
        let out = run_compact(
            root.path(),
            &crate::parse_args(&[
                "compact".to_string(),
                "--strict=1".to_string(),
                "--cycle-id=c39".to_string(),
                format!("--assimilations-json={}", assimilations),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/assimilation/kept_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.pointer("/assimilation/discarded_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        let prime_path = out
            .pointer("/prime_directive_compacted_state/path")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!prime_path.is_empty());
        assert!(Path::new(prime_path).exists());
        let blob_rows = out
            .pointer("/assimilation/discarded_blob_index/items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(blob_rows.len(), 1);
    }
}

