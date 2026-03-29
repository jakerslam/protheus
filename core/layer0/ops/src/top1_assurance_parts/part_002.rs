fn run_proof_vm(
    root: &Path,
    policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let docker_rel = parsed
        .flags
        .get("dockerfile-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.proof_vm.dockerfile_path.as_str());
    let replay_rel = parsed
        .flags
        .get("replay-script-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.proof_vm.replay_script_path.as_str());
    let manifest_rel = parsed
        .flags
        .get("manifest-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.proof_vm.manifest_path.as_str());
    let write_manifest = parse_bool(parsed.flags.get("write-manifest"), true);

    let docker_path = root.join(docker_rel);
    let replay_path = root.join(replay_rel);
    let manifest_path = root.join(manifest_rel);

    let mut errors = Vec::<String>::new();
    if !docker_path.exists() {
        errors.push("proof_vm_dockerfile_missing".to_string());
    }
    if !replay_path.exists() {
        errors.push("proof_vm_replay_script_missing".to_string());
    }

    let docker_sha = sha256_file(&docker_path).ok();
    let replay_sha = sha256_file(&replay_path).ok();

    #[cfg(unix)]
    let replay_executable = {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(&replay_path)
            .ok()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };

    #[cfg(not(unix))]
    let replay_executable = replay_path.exists();

    if !replay_executable {
        errors.push("proof_vm_replay_script_not_executable".to_string());
    }

    let ok = errors.is_empty();
    let manifest = json!({
        "ok": ok,
        "type": "top1_proof_vm_manifest",
        "ts": now_iso(),
        "dockerfile_path": docker_rel,
        "dockerfile_sha256": docker_sha,
        "replay_script_path": replay_rel,
        "replay_script_sha256": replay_sha,
        "replay_script_executable": replay_executable,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "proof_vm_replay_contract",
                "claim": "proof_vm_replay_artifacts_are_reproducible_and_hash_pinned",
                "evidence": {
                    "dockerfile": docker_rel,
                    "replay_script": replay_rel
                }
            }
        ]
    });

    if write_manifest {
        let _ = write_json(&manifest_path, &manifest);
    }

    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "dockerfile_path": docker_rel,
        "replay_script_path": replay_rel,
        "manifest_path": manifest_rel,
        "manifest_written": write_manifest,
        "dockerfile_sha256": docker_sha,
        "replay_script_sha256": replay_sha,
        "replay_script_executable": replay_executable,
        "errors": manifest.get("errors").cloned().unwrap_or(Value::Array(Vec::new()))
    })
}

fn run_size_gate(
    root: &Path,
    policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let binary_rel = parsed
        .flags
        .get("binary-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.size_gate.binary_path.as_str());
    let min_mb = parse_f64(
        parsed.flags.get("min-mb"),
        policy.size_gate.min_mb,
        0.0,
        4096.0,
    );
    let max_mb = parse_f64(
        parsed.flags.get("max-mb"),
        policy.size_gate.max_mb,
        min_mb,
        8192.0,
    );
    let require_static = parse_bool(
        parsed.flags.get("require-static"),
        policy.size_gate.require_static,
    );
    let binary_path = root.join(binary_rel);

    let exists = binary_path.exists();
    let bytes = fs::metadata(&binary_path).map(|m| m.len()).unwrap_or(0);
    let size_mb = (bytes as f64) / (1024.0 * 1024.0);

    let file_probe = if exists {
        let p = normalize_rel(&binary_path);
        run_command_with("file", &[p.as_str()])
    } else {
        json!({"ok": false, "status": 1, "stderr": "binary_missing"})
    };
    let static_detected = file_probe
        .get("stdout")
        .and_then(Value::as_str)
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower.contains("statically linked") || lower.contains("static-pie")
        })
        .unwrap_or(false);

    let mut errors = Vec::<String>::new();
    if !exists {
        errors.push("binary_missing".to_string());
    }
    if exists && !(size_mb >= min_mb && size_mb <= max_mb) {
        errors.push("binary_size_out_of_range".to_string());
    }
    if require_static && exists && !static_detected {
        errors.push("binary_not_static".to_string());
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "binary_path": binary_rel,
        "exists": exists,
        "size_bytes": bytes,
        "size_mb": (size_mb * 1000.0).round() / 1000.0,
        "min_mb": min_mb,
        "max_mb": max_mb,
        "require_static": require_static,
        "static_detected": static_detected,
        "file_probe": file_probe,
        "errors": errors
    })
}

fn collect_benchmark_metrics(
    root: &Path,
    benchmark_path: &Path,
    sample_ms: u64,
    refresh: bool,
) -> Value {
    let existing = read_json(benchmark_path).unwrap_or(Value::Null);

    let status_args = parse_args(&["status".to_string()]);
    let runtime = status_runtime_efficiency_floor(root, &status_args).json;

    let cold_start_ms = extract_metric(&existing, &["metrics", "cold_start_ms"])
        .or_else(|| extract_metric(&existing, &["openclaw_measured", "cold_start_ms"]))
        .or_else(|| extract_metric(&runtime, &["latest", "metrics", "cold_start_p95_ms"]))
        .or_else(|| extract_metric(&runtime, &["metrics", "cold_start_p95_ms"]));

    let idle_rss_mb = extract_metric(&existing, &["metrics", "idle_rss_mb"])
        .or_else(|| extract_metric(&existing, &["openclaw_measured", "idle_memory_mb"]))
        .or_else(|| extract_metric(&runtime, &["latest", "metrics", "idle_rss_p95_mb"]))
        .or_else(|| extract_metric(&runtime, &["metrics", "idle_rss_p95_mb"]));

    let install_size_mb = extract_metric(&existing, &["metrics", "install_size_mb"])
        .or_else(|| extract_metric(&existing, &["openclaw_measured", "install_size_mb"]))
        .or_else(|| extract_metric(&runtime, &["latest", "metrics", "full_install_total_mb"]))
        .or_else(|| extract_metric(&runtime, &["metrics", "full_install_total_mb"]));

    let tasks_per_sec = extract_metric(&existing, &["metrics", "tasks_per_sec"])
        .or_else(|| extract_metric(&existing, &["openclaw_measured", "tasks_per_sec"]))
        .unwrap_or_else(|| microbench_tasks_per_sec(sample_ms));

    let generated = json!({
        "ok": true,
        "type": "top1_benchmark_metrics",
        "ts": now_iso(),
        "metrics": {
            "cold_start_ms": cold_start_ms,
            "idle_rss_mb": idle_rss_mb,
            "install_size_mb": install_size_mb,
            "tasks_per_sec": (tasks_per_sec * 100.0).round() / 100.0
        },
        "runtime_efficiency_source": runtime,
        "refresh": refresh,
        "sample_ms": sample_ms,
        "source_path": rel_path(root, benchmark_path)
    });

    if refresh {
        let _ = write_json(benchmark_path, &generated);
        let _ = write_json(
            &root.join("core/local/state/ops/top1_assurance/benchmark_latest.json"),
            &generated,
        );
        let _ = write_json(
            &root.join("local/state/ops/top1_assurance/benchmark_latest.json"),
            &generated,
        );
    }

    generated
}

fn run_benchmark_thresholds(
    root: &Path,
    policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let bench_rel = parsed
        .flags
        .get("benchmark-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.benchmark.benchmark_path.as_str());
    let sample_ms = parse_u64(
        parsed.flags.get("sample-ms"),
        policy.benchmark.sample_ms,
        100,
        10_000,
    );
    let refresh = parse_bool(parsed.flags.get("refresh"), true);

    let cold_max = parse_f64(
        parsed.flags.get("cold-start-max-ms"),
        policy.benchmark.cold_start_max_ms,
        0.1,
        120_000.0,
    );
    let idle_max = parse_f64(
        parsed.flags.get("idle-rss-max-mb"),
        policy.benchmark.idle_rss_max_mb,
        0.1,
        8192.0,
    );
    let tasks_min = parse_f64(
        parsed.flags.get("tasks-per-sec-min"),
        policy.benchmark.tasks_per_sec_min,
        1.0,
        100_000_000.0,
    );

    let benchmark_path = root.join(bench_rel);
    let metrics_row = collect_benchmark_metrics(root, &benchmark_path, sample_ms, refresh);

    let cold = metrics_row
        .get("metrics")
        .and_then(|m| m.get("cold_start_ms"))
        .and_then(Value::as_f64);
    let idle = metrics_row
        .get("metrics")
        .and_then(|m| m.get("idle_rss_mb"))
        .and_then(Value::as_f64);
    let tasks = metrics_row
        .get("metrics")
        .and_then(|m| m.get("tasks_per_sec"))
        .and_then(Value::as_f64);

    let mut errors = Vec::<String>::new();
    let mut checks = Map::<String, Value>::new();

    let cold_ok = cold.map(|v| v <= cold_max).unwrap_or(false);
    let idle_ok = idle.map(|v| v <= idle_max).unwrap_or(false);
    let tasks_ok = tasks.map(|v| v >= tasks_min).unwrap_or(false);

    checks.insert("cold_start_max".to_string(), json!(cold_ok));
    checks.insert("idle_rss_max".to_string(), json!(idle_ok));
    checks.insert("tasks_per_sec_min".to_string(), json!(tasks_ok));

    if !cold_ok {
        errors.push("cold_start_threshold_failed".to_string());
    }
    if !idle_ok {
        errors.push("idle_rss_threshold_failed".to_string());
    }
    if !tasks_ok {
        errors.push("tasks_per_sec_threshold_failed".to_string());
    }

    let ok = errors.is_empty();

    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "benchmark_path": bench_rel,
        "thresholds": {
            "cold_start_max_ms": cold_max,
            "idle_rss_max_mb": idle_max,
            "tasks_per_sec_min": tasks_min
        },
        "metrics": metrics_row.get("metrics").cloned().unwrap_or(Value::Null),
        "checks": checks,
        "errors": errors
    })
}

fn render_matrix_markdown(
    generated_at: &str,
    projects: &Map<String, Value>,
    source_benchmark: &str,
    source_snapshot: &str,
) -> String {
    let mut lines = Vec::<String>::new();
    lines.push("# Protheus vs X (CI Generated)".to_string());
    lines.push(String::new());
    lines.push(format!("Generated at: `{generated_at}`"));
    lines.push(format!("Source benchmark: `{source_benchmark}`"));
    lines.push(format!("Source snapshot: `{source_snapshot}`"));
    lines.push(String::new());
    lines.push(
        "| Project | Cold Start (ms) | Idle RSS (MB) | Install (MB) | Tasks/sec |".to_string(),
    );
    lines.push("|---|---:|---:|---:|---:|".to_string());

    let mut names = projects.keys().cloned().collect::<Vec<_>>();
    names.sort();

    for name in names {
        let row = projects.get(&name).and_then(Value::as_object).cloned();
        let Some(row) = row else {
            continue;
        };
        let cold = row
            .get("cold_start_ms")
            .and_then(Value::as_f64)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "n/a".to_string());
        let idle = row
            .get("idle_memory_mb")
            .or_else(|| row.get("idle_rss_mb"))
            .and_then(Value::as_f64)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "n/a".to_string());
        let install = row
            .get("install_size_mb")
            .and_then(Value::as_f64)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "n/a".to_string());
        let tasks = row
            .get("tasks_per_sec")
            .and_then(Value::as_f64)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "n/a".to_string());

        lines.push(format!(
            "| {name} | {cold} | {idle} | {install} | {tasks} |"
        ));
    }

    lines.push(String::new());
    lines.push(
        "This table is generated from receipted benchmark artifacts; manual edits are overwritten."
            .to_string(),
    );

    lines.join("\n") + "\n"
}

