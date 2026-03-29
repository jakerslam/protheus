fn run_impl(
    root: &Path,
    cmd: &str,
    snapshot_rel: &str,
    refresh_runtime: bool,
    bar_width: usize,
    throughput_uncached: bool,
    preflight_config: BenchmarkPreflightConfig,
) -> Result<Value, String> {
    let snapshot_path = root.join(snapshot_rel);
    let snapshot = read_json(&snapshot_path)?;
    let benchmark_preflight = benchmark_preflight(preflight_config, throughput_uncached);
    if benchmark_preflight
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        == false
    {
        return Err(format!(
            "benchmark_preflight_failed:{}",
            clean(
                &serde_json::to_string(&benchmark_preflight).unwrap_or_else(|_| "{}".to_string()),
                2000
            )
        ));
    }
    let shared_throughput_sampling = stabilized_tasks_per_sec(
        SHARED_THROUGHPUT_ROUNDS,
        SHARED_THROUGHPUT_SAMPLE_MS,
        throughput_uncached,
    );

    let (openclaw_measured, runtime_receipt) =
        measure_openclaw(root, refresh_runtime, &shared_throughput_sampling)?;
    let (pure_workspace_measured, pure_workspace_tiny_max_measured) =
        measure_pure_workspace(root, shared_throughput_sampling.tasks_per_sec)?;
    let projects = merge_projects(&snapshot, &openclaw_measured)?;
    let mut projects = projects;
    if let Some(ref pure) = pure_workspace_measured {
        projects.insert("InfRing (pure)".to_string(), Value::Object(pure.clone()));
    }
    if let Some(ref pure_tiny_max) = pure_workspace_tiny_max_measured {
        projects.insert(
            "InfRing (tiny-max)".to_string(),
            Value::Object(pure_tiny_max.clone()),
        );
    }

    let mut categories = Vec::<Value>::new();
    let mut ascii_report = Vec::<String>::new();
    ascii_report.push("Benchmarks: Measured, Not Marketed".to_string());
    if let Some(context) = snapshot.get("benchmark_context").and_then(Value::as_str) {
        ascii_report.push(context.to_string());
    }

    for category in CATEGORIES {
        let report = category_report(category, &projects, bar_width)?;
        if let Some(lines) = report.get("ascii_lines").and_then(Value::as_array) {
            for line in lines {
                if let Some(text) = line.as_str() {
                    ascii_report.push(text.to_string());
                }
            }
        }
        ascii_report.push(String::new());
        categories.push(report);
    }

    let mut out = json!({
        "ok": true,
        "type": "competitive_benchmark_matrix",
        "lane": LANE_ID,
        "mode": cmd,
        "ts": now_iso(),
        "environment_fingerprint": benchmark_environment_fingerprint(root),
        "snapshot_path": snapshot_rel,
        "snapshot_version": snapshot.get("schema_version").cloned().unwrap_or(Value::Null),
        "snapshot_generated_from": snapshot.get("generated_from").cloned().unwrap_or(Value::Null),
        "reference_month": snapshot.get("reference_month").cloned().unwrap_or(Value::Null),
        "bar_width": bar_width,
        "openclaw_measured": Value::Object(openclaw_measured),
        "pure_workspace_measured": pure_workspace_measured.clone().map(Value::Object).unwrap_or(Value::Null),
        "pure_workspace_tiny_max_measured": pure_workspace_tiny_max_measured
            .clone()
            .map(Value::Object)
            .unwrap_or(Value::Null),
        "runtime_receipt": runtime_receipt,
        "benchmark_preflight": benchmark_preflight,
        "shared_throughput_sampling": {
            "tasks_per_sec": shared_throughput_sampling.tasks_per_sec,
            "rounds": SHARED_THROUGHPUT_ROUNDS,
            "warmup_rounds": SHARED_THROUGHPUT_WARMUP_ROUNDS,
            "sample_ms": SHARED_THROUGHPUT_SAMPLE_MS,
            "source": SHARED_THROUGHPUT_SOURCE,
            "uncached": shared_throughput_sampling.uncached,
            "workload_seed": clean(&shared_throughput_sampling.workload_seed, 160),
            "warmup_samples_ops_per_sec": shared_throughput_sampling
                .warmup_samples
                .iter()
                .map(|value| json!(((value * 100.0).round()) / 100.0))
                .collect::<Vec<Value>>(),
            "measured_samples_ops_per_sec": shared_throughput_sampling
                .measured_samples
                .iter()
                .map(|value| json!(((value * 100.0).round()) / 100.0))
                .collect::<Vec<Value>>(),
            "stddev_ops_per_sec": ((shared_throughput_sampling.stddev * 100.0).round()) / 100.0,
            "min_ops_per_sec": ((shared_throughput_sampling.min * 100.0).round()) / 100.0,
            "max_ops_per_sec": ((shared_throughput_sampling.max * 100.0).round()) / 100.0
        },
        "projects": Value::Object(projects),
        "categories": categories,
        "ascii_report": ascii_report,
        "claim_evidence": [
            {
                "id": "competitive_benchmark_matrix_live_openclaw",
                "claim": "openclaw_metrics_are_measured_from_runtime_and_policy_counters",
                "evidence": {
                    "runtime_source": "runtime_efficiency_floor",
                    "counter_sources": [
                        "client/runtime/config/guard_check_registry.json",
                        "client/runtime/config/platform_adaptation_channels.json",
                        "client/runtime/config/provider_onboarding_manifest.json",
                        "client/runtime/config/model_health_auto_recovery_policy.json"
                    ]
                }
            },
            {
                "id": "competitive_benchmark_matrix_snapshot_reference",
                "claim": "competitor_metrics_are_loaded_from_reference_snapshot",
                "evidence": {
                    "snapshot_path": snapshot_rel,
                    "reference_month": snapshot.get("reference_month").cloned().unwrap_or(Value::Null)
                }
            },
            {
                "id": "competitive_benchmark_matrix_pure_workspace_probe",
                "claim": "pure_workspace_metrics_are_measured_from_rust_only_client_binary_probes_when_artifacts_exist",
                "evidence": {
                    "binary_probe_present": pure_workspace_measured.is_some()
                }
            },
            {
                "id": "competitive_benchmark_matrix_pure_workspace_tiny_max_probe",
                "claim": "pure_workspace_tiny_max_profile_is_reported_when_tiny_max_daemon_artifact_is_available",
                "evidence": {
                    "tiny_max_probe_present": pure_workspace_tiny_max_measured.is_some()
                }
            },
            {
                "id": "competitive_benchmark_matrix_environment_fingerprint",
                "claim": "benchmark_reports_include_runtime_environment_fingerprint_for_reproducibility",
                "evidence": {
                    "environment_fingerprint_present": true
                }
            },
            {
                "id": "competitive_benchmark_matrix_uncached_throughput_sampling",
                "claim": "shared_throughput_baseline_runs optional uncached pre-sample cache-bust and exposes round-level spread",
                "evidence": {
                    "throughput_uncached": throughput_uncached,
                    "throughput_rounds": SHARED_THROUGHPUT_ROUNDS,
                    "throughput_warmup_rounds": SHARED_THROUGHPUT_WARMUP_ROUNDS
                }
            },
            {
                "id": "competitive_benchmark_matrix_preflight_gate",
                "claim": "benchmark publication fails closed when host load or throughput jitter exceed bounded preflight thresholds",
                "evidence": {
                    "preflight_enabled": preflight_config.enabled,
                    "max_load_per_core": preflight_config.max_load_per_core,
                    "max_noise_cv_pct": preflight_config.max_noise_cv_pct,
                    "noise_sample_ms": preflight_config.noise_sample_ms,
                    "noise_rounds": preflight_config.noise_rounds
                }
            }
        ]
    });

    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));

    let latest_path = root.join(STATE_LATEST_REL);
    let history_path = root.join(STATE_HISTORY_REL);
    write_json_atomic(&latest_path, &out)?;
    append_jsonl(&history_path, &out)?;

    Ok(out)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "help"))
    {
        usage();
        return 0;
    }

    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());

    let snapshot_rel = parsed
        .flags
        .get("snapshot")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_SNAPSHOT_REL.to_string());

    let refresh_default = false;
    let refresh_runtime = parse_bool_flag(
        parsed.flags.get("refresh-runtime").map(String::as_str),
        refresh_default,
    );
    let throughput_uncached = parse_bool_flag(
        parsed.flags.get("throughput-uncached").map(String::as_str),
        SHARED_THROUGHPUT_DEFAULT_UNCACHED,
    );
    let preflight_enabled = parse_bool_flag(
        parsed.flags.get("benchmark-preflight").map(String::as_str),
        BENCHMARK_PREFLIGHT_ENABLED_DEFAULT,
    );
    let preflight_config = BenchmarkPreflightConfig {
        enabled: preflight_enabled,
        max_load_per_core: parse_f64_flag(
            parsed
                .flags
                .get("preflight-max-load-per-core")
                .map(String::as_str),
            BENCHMARK_PREFLIGHT_MAX_LOAD_PER_CORE_DEFAULT,
            0.01,
            8.0,
        ),
        max_noise_cv_pct: parse_f64_flag(
            parsed
                .flags
                .get("preflight-max-noise-cv-pct")
                .map(String::as_str),
            BENCHMARK_PREFLIGHT_MAX_NOISE_CV_PCT_DEFAULT,
            0.01,
            100.0,
        ),
        noise_sample_ms: parse_u64_flag(
            parsed
                .flags
                .get("preflight-noise-sample-ms")
                .map(String::as_str),
            BENCHMARK_PREFLIGHT_NOISE_SAMPLE_MS_DEFAULT,
            100,
            5_000,
        ),
        noise_rounds: parse_usize_flag(
            parsed
                .flags
                .get("preflight-noise-rounds")
                .map(String::as_str),
            BENCHMARK_PREFLIGHT_NOISE_ROUNDS_DEFAULT,
            1,
            20,
        ),
    };
    let bar_width = parse_bar_width(parsed.flags.get("bar-width").map(String::as_str));

    match cmd.as_str() {
        "run" | "status" => match run_impl(
            root,
            &cmd,
            &snapshot_rel,
            refresh_runtime,
            bar_width,
            throughput_uncached,
            preflight_config,
        ) {
            Ok(out) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).unwrap_or_else(|_| {
                        "{\"ok\":false,\"error\":\"encode_failed\"}".to_string()
                    })
                );
                0
            }
            Err(err) => {
                let mut out = json!({
                    "ok": false,
                    "type": "competitive_benchmark_matrix",
                    "lane": LANE_ID,
                    "mode": cmd,
                    "ts": now_iso(),
                    "snapshot_path": snapshot_rel,
                    "error": err
                });
                out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).unwrap_or_else(|_| {
                        "{\"ok\":false,\"error\":\"encode_failed\"}".to_string()
                    })
                );
                1
            }
        },
        _ => {
            usage();
            let mut out = json!({
                "ok": false,
                "type": "competitive_benchmark_matrix_cli_error",
                "lane": LANE_ID,
                "ts": now_iso(),
                "error": "unknown_command",
                "command": cmd
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            println!(
                "{}",
                serde_json::to_string_pretty(&out)
                    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
            );
            2
        }
    }
}

#[cfg(test)]
#[path = "benchmark_matrix_tests.rs"]
mod tests;

