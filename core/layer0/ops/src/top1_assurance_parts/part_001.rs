fn load_policy(_root: &Path, policy_path: &Path) -> Top1Policy {
    let mut policy = default_policy();
    let raw = read_json(policy_path).unwrap_or(Value::Null);

    if let Some(version) = raw.get("version").and_then(Value::as_str) {
        let clean = version.trim();
        if !clean.is_empty() {
            policy.version = clean.to_string();
        }
    }
    policy.strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(policy.strict_default);

    if let Some(node) = raw.get("proof_coverage") {
        if let Some(v) = node.get("map_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.proof_coverage.map_path = clean.to_string();
            }
        }
        policy.proof_coverage.min_proven_ratio = node
            .get("min_proven_ratio")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(0.0, 1.0))
            .unwrap_or(policy.proof_coverage.min_proven_ratio);
        policy.proof_coverage.check_toolchains_default = node
            .get("check_toolchains_default")
            .and_then(Value::as_bool)
            .unwrap_or(policy.proof_coverage.check_toolchains_default);
    }

    if let Some(node) = raw.get("proof_vm") {
        if let Some(v) = node.get("dockerfile_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.proof_vm.dockerfile_path = clean.to_string();
            }
        }
        if let Some(v) = node.get("replay_script_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.proof_vm.replay_script_path = clean.to_string();
            }
        }
        if let Some(v) = node.get("manifest_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.proof_vm.manifest_path = clean.to_string();
            }
        }
    }

    if let Some(node) = raw.get("size_gate") {
        if let Some(v) = node.get("binary_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.size_gate.binary_path = clean.to_string();
            }
        }
        policy.size_gate.min_mb = node
            .get("min_mb")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(policy.size_gate.min_mb)
            .clamp(0.0, 2048.0);
        policy.size_gate.max_mb = node
            .get("max_mb")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(policy.size_gate.max_mb)
            .clamp(0.0, 4096.0);
        if policy.size_gate.max_mb < policy.size_gate.min_mb {
            policy.size_gate.max_mb = policy.size_gate.min_mb;
        }
        policy.size_gate.require_static = node
            .get("require_static")
            .and_then(Value::as_bool)
            .unwrap_or(policy.size_gate.require_static);
    }

    if let Some(node) = raw.get("benchmark") {
        if let Some(v) = node.get("benchmark_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.benchmark.benchmark_path = clean.to_string();
            }
        }
        policy.benchmark.cold_start_max_ms = node
            .get("cold_start_max_ms")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(policy.benchmark.cold_start_max_ms)
            .clamp(1.0, 120000.0);
        policy.benchmark.idle_rss_max_mb = node
            .get("idle_rss_max_mb")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(policy.benchmark.idle_rss_max_mb)
            .clamp(1.0, 10240.0);
        policy.benchmark.tasks_per_sec_min = node
            .get("tasks_per_sec_min")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(policy.benchmark.tasks_per_sec_min)
            .clamp(1.0, 10_000_000.0);
        policy.benchmark.sample_ms = node
            .get("sample_ms")
            .and_then(Value::as_u64)
            .unwrap_or(policy.benchmark.sample_ms)
            .clamp(100, 10_000);
    }

    if let Some(node) = raw.get("comparison") {
        if let Some(v) = node.get("snapshot_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.comparison.snapshot_path = clean.to_string();
            }
        }
        if let Some(v) = node.get("output_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.comparison.output_path = clean.to_string();
            }
        }
    }

    if let Some(node) = raw.get("outputs") {
        if let Some(v) = node.get("latest_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.outputs.latest_path = clean.to_string();
            }
        }
        if let Some(v) = node.get("history_path").and_then(Value::as_str) {
            let clean = v.trim();
            if !clean.is_empty() {
                policy.outputs.history_path = clean.to_string();
            }
        }
    }

    if !policy_path.exists() {
        let _ = write_json(
            policy_path,
            &json!({
                "version": policy.version,
                "strict_default": policy.strict_default,
                "proof_coverage": {
                    "map_path": policy.proof_coverage.map_path,
                    "min_proven_ratio": policy.proof_coverage.min_proven_ratio,
                    "check_toolchains_default": policy.proof_coverage.check_toolchains_default
                },
                "proof_vm": {
                    "dockerfile_path": policy.proof_vm.dockerfile_path,
                    "replay_script_path": policy.proof_vm.replay_script_path,
                    "manifest_path": policy.proof_vm.manifest_path
                },
                "size_gate": {
                    "binary_path": policy.size_gate.binary_path,
                    "min_mb": policy.size_gate.min_mb,
                    "max_mb": policy.size_gate.max_mb,
                    "require_static": policy.size_gate.require_static
                },
                "benchmark": {
                    "benchmark_path": policy.benchmark.benchmark_path,
                    "cold_start_max_ms": policy.benchmark.cold_start_max_ms,
                    "idle_rss_max_mb": policy.benchmark.idle_rss_max_mb,
                    "tasks_per_sec_min": policy.benchmark.tasks_per_sec_min,
                    "sample_ms": policy.benchmark.sample_ms
                },
                "comparison": {
                    "snapshot_path": policy.comparison.snapshot_path,
                    "output_path": policy.comparison.output_path
                },
                "outputs": {
                    "latest_path": policy.outputs.latest_path,
                    "history_path": policy.outputs.history_path
                }
            }),
        );
    }

    policy
}

fn microbench_tasks_per_sec(sample_ms: u64) -> f64 {
    let target = Duration::from_millis(sample_ms);
    let started = Instant::now();
    let mut tasks: u64 = 0;
    while started.elapsed() < target {
        let payload = format!("task-{tasks}");
        let _ = Sha256::digest(payload.as_bytes());
        tasks = tasks.saturating_add(1);
    }
    let secs = started.elapsed().as_secs_f64();
    if secs <= 0.0 {
        0.0
    } else {
        tasks as f64 / secs
    }
}

fn extract_metric(payload: &Value, keys: &[&str]) -> Option<f64> {
    let mut cursor = payload;
    for (idx, key) in keys.iter().enumerate() {
        if idx + 1 == keys.len() {
            return cursor.get(*key).and_then(Value::as_f64);
        }
        cursor = cursor.get(*key)?;
    }
    None
}

fn run_proof_coverage(
    root: &Path,
    policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let map_rel = parsed
        .flags
        .get("map-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.proof_coverage.map_path.as_str());
    let map_path = root.join(map_rel);
    let check_toolchains = parse_bool(
        parsed.flags.get("check-toolchains"),
        policy.proof_coverage.check_toolchains_default,
    );
    let execute_proofs = parse_bool(parsed.flags.get("execute-proofs"), false);
    let execute_optional_proofs = parse_bool(parsed.flags.get("execute-optional-proofs"), false);

    let mut errors = Vec::<String>::new();
    let map = read_json(&map_path).unwrap_or(Value::Null);
    if map.is_null() {
        errors.push("coverage_map_missing_or_invalid".to_string());
    }

    if map
        .get("schema_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "core_formal_coverage_map"
    {
        errors.push("coverage_map_schema_id_invalid".to_string());
    }

    let surfaces = map
        .get("surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if surfaces.is_empty() {
        errors.push("coverage_map_surfaces_missing".to_string());
    }

    let mut proven = 0usize;
    let mut partial = 0usize;
    let mut unproven = 0usize;
    let mut invalid_surfaces = Vec::<String>::new();
    let mut artifact_rows = Vec::<Value>::new();
    let mut proof_command_rows = Vec::<Value>::new();

    for row in &surfaces {
        let id = row
            .get("id")
            .or_else(|| row.get("crate"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if id.is_empty() {
            invalid_surfaces.push("missing_surface_id".to_string());
            continue;
        }
        match status.as_str() {
            "proven" => proven += 1,
            "partial" => partial += 1,
            "unproven" => unproven += 1,
            _ => invalid_surfaces.push(format!("invalid_surface_status:{id}")),
        }

        let artifact_rel = row
            .get("artifact")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let artifact_exists = if artifact_rel.is_empty() {
            false
        } else {
            root.join(&artifact_rel).exists()
        };
        if artifact_rel.is_empty() {
            errors.push(format!("surface_artifact_missing::{id}"));
        } else if !artifact_exists {
            errors.push(format!("surface_artifact_not_found::{id}"));
        }
        artifact_rows.push(json!({
            "surface_id": id,
            "artifact": artifact_rel,
            "exists": artifact_exists
        }));

        let command_specs = row
            .get("proof_commands")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (idx, command_spec) in command_specs.iter().enumerate() {
            let command_id = command_spec
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let required = command_spec
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let argv = command_spec
                .get("argv")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if argv.is_empty() {
                if required {
                    errors.push(format!("proof_command_missing_argv::{id}::{idx}"));
                }
                proof_command_rows.push(json!({
                    "surface_id": id,
                    "id": if command_id.is_empty() { format!("cmd_{idx}") } else { command_id.clone() },
                    "required": required,
                    "executed": false,
                    "ok": false,
                    "error": "missing_argv"
                }));
                continue;
            }
            let bin = argv.first().cloned().unwrap_or_default();
            let args = argv.into_iter().skip(1).collect::<Vec<_>>();
            let should_execute = execute_proofs && (required || execute_optional_proofs);
            let run = if should_execute {
                run_command_with(&bin, &args)
            } else {
                json!({
                    "ok": true,
                    "status": 0,
                    "elapsed_ms": 0,
                    "stdout": if execute_proofs && !required {
                        "skipped_optional"
                    } else {
                        "skipped"
                    },
                    "stderr": ""
                })
            };
            let ok = run.get("ok").and_then(Value::as_bool).unwrap_or(false);
            if should_execute && required && !ok {
                errors.push(format!(
                    "proof_command_failed::{id}::{}",
                    if command_id.is_empty() {
                        format!("cmd_{idx}")
                    } else {
                        command_id.clone()
                    }
                ));
            }
            proof_command_rows.push(json!({
                "surface_id": id,
                "id": if command_id.is_empty() { format!("cmd_{idx}") } else { command_id },
                "required": required,
                "executed": should_execute,
                "ok": if should_execute { ok } else { true },
                "skipped_optional": execute_proofs && !required && !should_execute,
                "bin": bin,
                "args": args,
                "run": run
            }));
        }
    }

    if !invalid_surfaces.is_empty() {
        errors.extend(invalid_surfaces.clone());
    }

    let total = proven + partial + unproven;
    let proven_ratio = if total == 0 {
        0.0
    } else {
        proven as f64 / total as f64
    };

    if total > 0 && proven == 0 {
        errors.push("coverage_map_requires_at_least_one_proven_surface".to_string());
    }
    if proven_ratio < policy.proof_coverage.min_proven_ratio {
        errors.push("coverage_ratio_below_policy_floor".to_string());
    }

    let tool_checks = if check_toolchains {
        vec![
            (
                "kani_toolchain",
                true,
                run_toolchain_check("kani_toolchain"),
            ),
            (
                "prusti_toolchain",
                false,
                run_toolchain_check("prusti_toolchain"),
            ),
            (
                "lean_toolchain",
                false,
                run_toolchain_check("lean_toolchain"),
            ),
        ]
    } else {
        Vec::new()
    };

    let mut toolchain_rows = Vec::<Value>::new();
    for (id, required, row) in tool_checks {
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if required && !ok {
            errors.push(format!("required_toolchain_missing::{id}"));
        }
        toolchain_rows.push(json!({
            "id": id,
            "required": required,
            "ok": ok,
            "row": row
        }));
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "map_path": map_rel,
        "check_toolchains": check_toolchains,
        "execute_proofs": execute_proofs,
        "execute_optional_proofs": execute_optional_proofs,
        "proven": proven,
        "partial": partial,
        "unproven": unproven,
        "surface_count": total,
        "proven_ratio": (proven_ratio * 10000.0).round() / 10000.0,
        "min_proven_ratio": policy.proof_coverage.min_proven_ratio,
        "artifacts": artifact_rows,
        "proof_commands": proof_command_rows,
        "toolchains": toolchain_rows,
        "errors": errors
    })
}

