fn build_fitness_review(
    _root: &Path,
    cycle_id: &str,
    bench_delta: &Value,
    reliability_before: f64,
    reliability_after: f64,
    assimilation_plan: &[Value],
) -> Value {
    let reliability_gate_pass = reliability_after >= reliability_before;
    let metrics_gate_pass = bench_delta
        .get("improved_metric_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    let tiny_strengthened = bench_delta
        .pointer("/pure_workspace_tiny_max/improved_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
        || bench_delta
            .pointer("/pure_workspace/improved_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0;

    let mut survivors = Vec::<Value>::new();
    let mut demoted = Vec::<Value>::new();
    let mut rejected = Vec::<Value>::new();
    for candidate in assimilation_plan {
        if !candidate.is_object() {
            continue;
        }
        let mut normalized = candidate.clone();
        let lower = normalized
            .get("idea")
            .and_then(Value::as_str)
            .or_else(|| normalized.get("id").and_then(Value::as_str))
            .unwrap_or("")
            .to_ascii_lowercase();
        let intelligence_fallback = lower.contains("rsi")
            || lower.contains("organism")
            || lower.contains("memory")
            || lower.contains("planner")
            || lower.contains("learn")
            || lower.contains("inference");
        let hardware_fallback = lower.contains("tiny")
            || lower.contains("embedded")
            || lower.contains("mcu")
            || lower.contains("pure")
            || lower.contains("low-power")
            || lower.contains("edge");
        let metric_gate = candidate_gate_bool(
            &normalized,
            &["metric_gain", "metrics_pass"],
            metrics_gate_pass,
        );
        let tiny_gate = candidate_gate_bool(
            &normalized,
            &["pure_tiny_strength", "tiny_strengthened", "pure_mode_gain"],
            tiny_strengthened,
        );
        let intelligence_gate = candidate_gate_bool(
            &normalized,
            &["intelligence_gain", "rsi_gain", "organism_gain"],
            intelligence_fallback,
        );
        let hardware_gate = candidate_gate_bool(
            &normalized,
            &["tiny_hardware_fit", "hardware_fit", "embedded_fit"],
            hardware_fallback,
        );
        let gates = json!({
            "metrics": metric_gate,
            "tiny_pure": tiny_gate,
            "rsi_organism": intelligence_gate,
            "tiny_hardware": hardware_gate,
            "reliability": reliability_gate_pass
        });
        let all_pass = gates
            .as_object()
            .map(|obj| obj.values().all(|value| value.as_bool().unwrap_or(false)))
            .unwrap_or(false);
        let demote = !all_pass
            && !metric_gate
            && (tiny_gate || intelligence_gate || hardware_gate)
            && reliability_gate_pass;
        let status = if all_pass {
            "survivor"
        } else if demote {
            "demoted_optional"
        } else {
            "rejected"
        };
        let rejection_reason = if status == "survivor" {
            "none"
        } else {
            first_failed_gate(&gates)
        };
        normalized["gate_results"] = gates.clone();
        normalized["status"] = Value::String(status.to_string());
        normalized["review_score"] = Value::from(score_candidate(&gates, bench_delta));
        normalized["evaluated_at"] = Value::String(crate::now_iso());
        normalized["rejection_reason"] = Value::String(rejection_reason.to_string());
        normalized["resurrection_metadata"] = json!({
            "cycle_id": cycle_id,
            "recheck_after": "next_snowball_cycle",
            "reason": rejection_reason
        });
        match status {
            "survivor" => survivors.push(normalized),
            "demoted_optional" => demoted.push(normalized),
            _ => rejected.push(normalized),
        }
    }

    json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "generated_at": crate::now_iso(),
        "bench_delta": bench_delta,
        "reliability": {
            "before": reliability_before,
            "after": reliability_after,
            "pass": reliability_gate_pass
        },
        "summary": {
            "survivor_count": survivors.len(),
            "demoted_count": demoted.len(),
            "rejected_count": rejected.len(),
            "improved_metric_count": bench_delta.get("improved_metric_count").and_then(Value::as_u64).unwrap_or(0),
            "regressed_metric_count": bench_delta.get("regressed_metric_count").and_then(Value::as_u64).unwrap_or(0),
            "tiny_strengthened": tiny_strengthened
        },
        "survivors": survivors,
        "demoted": demoted,
        "rejected": rejected
    })
}

fn load_review(root: &Path, cycle_id: &str) -> Option<Value> {
    read_json(&fitness_review_path(root, cycle_id))
}

fn load_cycle_value(cycles: &Value, cycle_id: &str) -> Option<Value> {
    cycles
        .get("cycles")
        .and_then(Value::as_object)
        .and_then(|map| map.get(cycle_id))
        .cloned()
}

fn format_with_commas(raw: f64) -> String {
    let base = format!("{raw:.1}");
    let parts = base.split('.').collect::<Vec<_>>();
    let integer = parts.first().copied().unwrap_or("0");
    let fraction = parts.get(1).copied().unwrap_or("0");
    let mut out = String::new();
    let bytes = integer.as_bytes();
    for (idx, ch) in bytes.iter().enumerate() {
        if idx > 0 && (bytes.len() - idx) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch as char);
    }
    out.push('.');
    out.push_str(fraction);
    out
}

fn readme_sync_summary(report: &Value, readme_text: &str) -> Value {
    let snippets = [
        (
            "rich_cold_start",
            format!(
                "{:.1} ms",
                report
                    .pointer("/infring_measured/cold_start_ms")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ),
        ),
        (
            "rich_idle_memory",
            format!(
                "{:.1} MB",
                report
                    .pointer("/infring_measured/idle_memory_mb")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ),
        ),
        (
            "pure_throughput",
            format!(
                "{} ops/sec",
                format_with_commas(
                    report
                        .pointer("/pure_workspace_measured/tasks_per_sec")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                )
            ),
        ),
        (
            "tiny_throughput",
            format!(
                "{} ops/sec",
                format_with_commas(
                    report
                        .pointer("/pure_workspace_tiny_max_measured/tasks_per_sec")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                )
            ),
        ),
    ];
    let rows = snippets
        .iter()
        .map(|(name, snippet)| {
            json!({
                "name": name,
                "snippet": snippet,
                "present": readme_text.contains(snippet)
            })
        })
        .collect::<Vec<_>>();
    let synced = rows
        .iter()
        .all(|row| row.get("present").and_then(Value::as_bool) == Some(true));
    json!({
        "synced": synced,
        "checks": rows
    })
}

fn default_assimilation_items(cycle_id: &str, drops: &[String]) -> Vec<Value> {
    drops
        .iter()
        .map(|drop| {
            let lower = drop.to_ascii_lowercase();
            let tiny_hint = lower.contains("tiny")
                || lower.contains("pure")
                || lower.contains("embedded")
                || lower.contains("mcu")
                || lower.contains("rpi");
            let intelligence_hint = lower.contains("rsi")
                || lower.contains("organism")
                || lower.contains("memory")
                || lower.contains("planner")
                || lower.contains("reason");
            json!({
                "id": format!("assim-{}-{}", cycle_id, clean_id(Some(drop.as_str()), "idea")),
                "idea": drop,
                "source": "snowball_drop",
                "metric_gain": true,
                "pure_tiny_strength": tiny_hint || lower.contains("ops") || lower.contains("core"),
                "intelligence_gain": intelligence_hint || lower.contains("core"),
                "tiny_hardware_fit": tiny_hint || lower.contains("core")
            })
        })
        .collect()
}

fn as_bool_opt(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

fn candidate_gate_bool(candidate: &Value, keys: &[&str], fallback: bool) -> bool {
    for key in keys {
        if let Some(result) = as_bool_opt(candidate.get(*key)) {
            return result;
        }
    }
    fallback
}

fn archive_discarded_blobs(
    root: &Path,
    cycle_id: &str,
    discarded: &[Value],
) -> (Vec<Value>, Value) {
    let dir = discarded_blob_dir(root, cycle_id);
    let _ = fs::create_dir_all(&dir);
    let mut index_rows = Vec::<Value>::new();
    for entry in discarded {
        let id = clean_id(entry.get("id").and_then(Value::as_str), "discarded");
        let encoded = serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string());
        let blob_hash = sha256_hex_str(&encoded);
        let path = dir.join(format!("{blob_hash}.blob"));
        let mut bytes = Vec::<u8>::with_capacity(encoded.len() + 8);
        bytes.extend_from_slice(b"SNOWV1\0");
        bytes.extend_from_slice(encoded.as_bytes());
        let _ = fs::write(&path, &bytes);
        index_rows.push(json!({
            "id": id,
            "path": path.display().to_string(),
            "sha256": blob_hash,
            "bytes": bytes.len()
        }));
    }
    let index = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "written_at": crate::now_iso(),
        "items": index_rows
    });
    let index_path = discarded_blob_index_path(root, cycle_id);
    let _ = write_json(&index_path, &index);
    (
        index
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        index,
    )
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "start" => vec!["V6-APP-023.1", "V6-APP-023.5", "V6-APP-023.6"],
        "melt-refine" | "regress" => vec!["V6-APP-023.2", "V6-APP-023.5", "V6-APP-023.6"],
        "compact" => vec![
            "V6-APP-023.3",
            "V6-APP-023.7",
            "V6-APP-023.9",
            "V6-APP-023.11",
            "V6-APP-023.5",
            "V6-APP-023.6",
        ],
        "fitness-review" => vec!["V6-APP-023.7", "V6-APP-023.5", "V6-APP-023.6"],
        "archive-discarded" => vec!["V6-APP-023.9"],
        "publish-benchmarks" => vec!["V6-APP-023.10"],
        "promote" => vec!["V6-APP-023.8"],
        "prime-update" => vec!["V6-APP-023.11"],
        "backlog-pack" => vec!["V6-APP-023.4", "V6-APP-023.5", "V6-APP-023.6"],
        "control" | "status" => vec!["V6-APP-023.5", "V6-APP-023.6"],
        _ => vec!["V6-APP-023.5", "V6-APP-023.6"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_rows = claim_ids_for_action(action)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "snowball_controls_route_through_layer0_conduit_with_fail_closed_denials",
                "evidence": {
                    "action": clean(action, 120),
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    build_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "snowball_conduit_enforcement",
        "core/layer0/ops/snowball_plane",
        bypass_requested,
        claim_rows,
    )
}

fn load_cycles(root: &Path) -> Value {
    read_json(&cycles_path(root)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "active_cycle_id": Value::Null,
            "cycles": {}
        })
    })
}

fn store_cycles(root: &Path, cycles: &Value) {
    let _ = write_json(&cycles_path(root), cycles);
}

fn active_or_requested_cycle(parsed: &crate::ParsedArgs, cycles: &Value, fallback: &str) -> String {
    clean_id(
        parsed
            .flags
            .get("cycle-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("cycle").map(String::as_str))
            .or_else(|| cycles.get("active_cycle_id").and_then(Value::as_str))
            .or(Some(fallback)),
        fallback,
    )
}

fn classify_drop_risk(drop: &str) -> &'static str {
    let lower = drop.to_ascii_lowercase();
    if lower.contains("prod")
        || lower.contains("deploy")
        || lower.contains("security")
        || lower.contains("payment")
    {
        "high"
    } else if lower.contains("migration") || lower.contains("schema") || lower.contains("runtime") {
        "medium"
    } else {
        "low"
    }
}

fn dependencies_from_json(
    drops: &[String],
    deps_json: Option<Value>,
) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::<String, Vec<String>>::new();
    for drop in drops {
        out.insert(drop.clone(), Vec::new());
    }
    if let Some(obj) = deps_json.and_then(|v| v.as_object().cloned()) {
        for (key, value) in obj {
            let k = clean(key, 80).to_ascii_lowercase();
            if !out.contains_key(&k) {
                continue;
            }
            let deps = value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 80).to_ascii_lowercase())
                .filter(|v| out.contains_key(v))
                .collect::<Vec<_>>();
            out.insert(k, deps);
        }
    }
    out
}

#[derive(Clone)]
struct BacklogItem {
    id: String,
    priority: i64,
    depends_on: Vec<String>,
    payload: Value,
    original_index: usize,
}

