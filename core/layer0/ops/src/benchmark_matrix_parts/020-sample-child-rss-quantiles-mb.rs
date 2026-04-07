fn sample_child_rss_quantiles_mb(
    program: &str,
    args: &[&str],
    warmup_runs: usize,
    samples: usize,
) -> Result<(f64, f64, f64), String> {
    for _ in 0..warmup_runs {
        let _ = sample_child_rss_mb(program, args)?;
    }
    let mut rows = Vec::new();
    for _ in 0..samples.max(1) {
        rows.push(sample_child_rss_mb(program, args)?);
    }
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok((
        percentile(&rows, 0.50),
        percentile(&rows, 0.95),
        percentile(&rows, 0.99),
    ))
}

fn command_stdout(program: &str, args: &[&str], cwd: Option<&Path>) -> Option<String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(clean(text, 120))
    }
}

fn benchmark_environment_fingerprint(root: &Path) -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpu_parallelism": std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        "rustc_version": command_stdout("rustc", &["--version"], None),
        "git_revision": command_stdout("git", &["rev-parse", "HEAD"], Some(root)),
        "workload_id": "live_hash_workload_v1"
    })
}

fn measure_pure_workspace_profile(
    root: &Path,
    mode: &str,
    probe_bin: &str,
    size_bin: &str,
    daemon_bin: Option<&str>,
    cold_start_quantiles: Option<(f64, f64, f64)>,
    cold_start_args: &[&str],
    idle_probe_args: &[&str],
    tasks_per_sec: f64,
    security_systems: f64,
) -> Result<Map<String, Value>, String> {
    let (cold_start_p50_ms, cold_start_p95_ms, cold_start_p99_ms) = cold_start_quantiles.unwrap_or(
        sample_command_quantiles_ms(probe_bin, cold_start_args, 2, 9)?,
    );
    let (idle_rss_p50_mb, idle_rss_p95_mb, idle_rss_p99_mb) =
        sample_child_rss_quantiles_mb(probe_bin, idle_probe_args, 1, 5)?;
    let mut install_size_mb = path_size_mb(root, size_bin);
    if let Some(daemon) = daemon_bin {
        install_size_mb += path_size_mb(root, daemon);
    }
    install_size_mb = (install_size_mb * 1000.0).round() / 1000.0;

    let mut measured = Map::<String, Value>::new();
    measured.insert("mode".to_string(), Value::String(mode.to_string()));
    measured.insert("cold_start_ms".to_string(), json!(cold_start_p50_ms));
    measured.insert("cold_start_p50_ms".to_string(), json!(cold_start_p50_ms));
    measured.insert("cold_start_p95_ms".to_string(), json!(cold_start_p95_ms));
    measured.insert("cold_start_p99_ms".to_string(), json!(cold_start_p99_ms));
    measured.insert("idle_memory_mb".to_string(), json!(idle_rss_p50_mb));
    measured.insert("idle_rss_p50_mb".to_string(), json!(idle_rss_p50_mb));
    measured.insert("idle_rss_p95_mb".to_string(), json!(idle_rss_p95_mb));
    measured.insert("idle_rss_p99_mb".to_string(), json!(idle_rss_p99_mb));
    measured.insert("install_size_mb".to_string(), json!(install_size_mb));
    attach_shared_throughput(&mut measured, tasks_per_sec);
    measured.insert("security_systems".to_string(), json!(security_systems));
    measured.insert("channel_adapters".to_string(), json!(0.0));
    measured.insert("llm_providers".to_string(), json!(0.0));
    measured.insert("measured".to_string(), Value::Bool(true));
    measured.insert(
        "data_source".to_string(),
        Value::String("pure_workspace_binary_probe".to_string()),
    );
    measured.insert(
        "probe_binary_path".to_string(),
        Value::String(clean(probe_bin, 320)),
    );
    measured.insert(
        "size_binary_path".to_string(),
        Value::String(clean(size_bin, 320)),
    );
    if let Some(daemon) = daemon_bin {
        measured.insert(
            "daemon_binary_path".to_string(),
            Value::String(clean(daemon, 320)),
        );
    }
    Ok(measured)
}

fn measure_pure_workspace(
    root: &Path,
    tasks_per_sec: f64,
) -> Result<(Option<Map<String, Value>>, Option<Map<String, Value>>), String> {
    let pure_probe_bin = locate_binary(
        root,
        &[
            "target/release/protheus-pure-workspace",
            "target/debug/protheus-pure-workspace",
            "target/x86_64-unknown-linux-musl/release/protheus-pure-workspace",
        ],
    );
    let Some(pure_probe_bin) = pure_probe_bin else {
        return Ok((None, None));
    };
    let pure_size_bin = locate_binary(
        root,
        &[
            "target/x86_64-unknown-linux-musl/release/protheus-pure-workspace",
            "target/release/protheus-pure-workspace",
            "target/debug/protheus-pure-workspace",
        ],
    )
    .unwrap_or_else(|| pure_probe_bin.clone());
    let daemon_bin_default = locate_binary(
        root,
        &[
            "target/x86_64-unknown-linux-musl/release/protheusd",
            "target/release/protheusd",
            "target/debug/protheusd",
        ],
    );

    let (pure_cold, tiny_cold) = sample_dual_command_quantiles_ms(
        pure_probe_bin.as_str(),
        &["benchmark-ping"],
        pure_probe_bin.as_str(),
        &["benchmark-ping", "--tiny-max=1"],
        2,
        9,
    )?;

    let security_systems = count_guard_checks(root)?;

    let default_profile = measure_pure_workspace_profile(
        root,
        "pure",
        pure_probe_bin.as_str(),
        pure_size_bin.as_str(),
        daemon_bin_default.as_deref(),
        Some(pure_cold),
        &["benchmark-ping"],
        &["probe", "--sleep-ms=450"],
        tasks_per_sec,
        security_systems,
    )?;

    let daemon_bin_tiny_max = locate_binary(
        root,
        &[
            "target/x86_64-unknown-linux-musl/release/protheusd_tiny_max",
            "target/x86_64-unknown-linux-musl/release/protheusd-tiny-max",
            "target/release/protheusd_tiny_max",
            "target/release/protheusd-tiny-max",
            "local/tmp/daemon-sizes/protheusd.pure",
        ],
    )
    .or_else(|| daemon_bin_default.clone());

    let tiny_max_profile = measure_pure_workspace_profile(
        root,
        "pure-tiny-max",
        pure_probe_bin.as_str(),
        pure_size_bin.as_str(),
        daemon_bin_tiny_max.as_deref(),
        Some(tiny_cold),
        &["benchmark-ping", "--tiny-max=1"],
        &["probe", "--sleep-ms=120", "--tiny-max=1"],
        tasks_per_sec,
        security_systems,
    )?;

    Ok((Some(default_profile), Some(tiny_max_profile)))
}

fn live_tasks_per_sec(sample_ms: u64) -> f64 {
    let target = Duration::from_millis(sample_ms.max(100));
    let started = Instant::now();
    let mut tasks = 0u64;
    while started.elapsed() < target {
        for idx in 0..SHARED_THROUGHPUT_WORK_FACTOR {
            let payload = format!("task-{tasks}-work-{idx}");
            let digest = Sha256::digest(payload.as_bytes());
            black_box(digest);
        }
        tasks = tasks.saturating_add(1);
    }
    let secs = started.elapsed().as_secs_f64();
    if secs <= 0.0 {
        0.0
    } else {
        tasks as f64 / secs
    }
}

fn effective_hash_ops_per_sec(tasks_per_sec: f64) -> f64 {
    ((tasks_per_sec * SHARED_THROUGHPUT_WORK_FACTOR as f64) * 100.0).round() / 100.0
}

fn pre_sample_cache_bust(workload_seed: &str, round_idx: usize) {
    let mut digest = Sha256::digest(format!("{workload_seed}:{round_idx}").as_bytes());
    for _ in 0..64 {
        let next = Sha256::digest(digest.as_slice());
        black_box(next.as_slice());
        digest = next;
    }
}

fn stable_stddev(samples: &[f64]) -> f64 {
    if samples.len() <= 1 {
        return 0.0;
    }
    let mean = samples.iter().copied().sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / samples.len() as f64;
    variance.sqrt()
}

fn benchmark_preflight_report(
    config: BenchmarkPreflightConfig,
    throughput_uncached: bool,
    cpu_parallelism: usize,
    load_one: f64,
    load_five: f64,
    load_fifteen: f64,
    noise_samples: &[f64],
) -> Value {
    let cpu_parallelism = cpu_parallelism.max(1);
    let load_per_core_one = load_one / cpu_parallelism as f64;
    let load_per_core_five = load_five / cpu_parallelism as f64;
    let load_per_core_peak = load_per_core_one.max(load_per_core_five);
    let mean_noise = if noise_samples.is_empty() {
        0.0
    } else {
        noise_samples.iter().copied().sum::<f64>() / noise_samples.len() as f64
    };
    let stddev_noise = stable_stddev(noise_samples);
    let noise_cv_pct = if mean_noise <= f64::EPSILON {
        100.0
    } else {
        (stddev_noise / mean_noise) * 100.0
    };

    let mut blockers = Vec::<String>::new();
    if load_per_core_peak > config.max_load_per_core {
        blockers.push(format!(
            "host_load_per_core_exceeded:{}>{}",
            ((load_per_core_peak * 1000.0).round()) / 1000.0,
            config.max_load_per_core
        ));
    }
    if noise_cv_pct > config.max_noise_cv_pct {
        blockers.push(format!(
            "throughput_noise_cv_exceeded:{}>{}",
            ((noise_cv_pct * 100.0).round()) / 100.0,
            config.max_noise_cv_pct
        ));
    }

    json!({
        "ok": blockers.is_empty(),
        "enabled": config.enabled,
        "throughput_uncached": throughput_uncached,
        "cpu_parallelism": cpu_parallelism,
        "load_average_one": ((load_one * 1000.0).round()) / 1000.0,
        "load_average_five": ((load_five * 1000.0).round()) / 1000.0,
        "load_average_fifteen": ((load_fifteen * 1000.0).round()) / 1000.0,
        "load_per_core_one": ((load_per_core_one * 1000.0).round()) / 1000.0,
        "load_per_core_five": ((load_per_core_five * 1000.0).round()) / 1000.0,
        "load_per_core_peak": ((load_per_core_peak * 1000.0).round()) / 1000.0,
        "max_load_per_core": config.max_load_per_core,
        "noise_sample_ms": config.noise_sample_ms,
        "noise_rounds": config.noise_rounds,
        "noise_samples_ops_per_sec": noise_samples
            .iter()
            .map(|value| json!(((value * 100.0).round()) / 100.0))
            .collect::<Vec<Value>>(),
        "noise_mean_ops_per_sec": ((mean_noise * 100.0).round()) / 100.0,
        "noise_stddev_ops_per_sec": ((stddev_noise * 100.0).round()) / 100.0,
        "noise_cv_pct": ((noise_cv_pct * 100.0).round()) / 100.0,
        "max_noise_cv_pct": config.max_noise_cv_pct,
        "blockers": blockers
    })
}

fn benchmark_preflight(config: BenchmarkPreflightConfig, throughput_uncached: bool) -> Value {
    if !config.enabled {
        return json!({
            "ok": true,
            "enabled": false,
            "skipped": true,
            "reason": "benchmark_preflight_disabled"
        });
    }

    let cpu_parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1);
    let load = System::load_average();

    let mut sample_idx = 0usize;
    let workload_seed = if throughput_uncached {
        format!("benchmark_preflight_uncached_seed_{}", now_iso())
    } else {
        "benchmark_preflight_cached".to_string()
    };
    let mut noise_samples = Vec::<f64>::new();
    for _ in 0..config.noise_rounds.max(1) {
        if throughput_uncached {
            pre_sample_cache_bust(workload_seed.as_str(), sample_idx);
        }
        sample_idx = sample_idx.saturating_add(1);
        noise_samples.push(live_tasks_per_sec(config.noise_sample_ms.max(100)));
    }

    benchmark_preflight_report(
        config,
        throughput_uncached,
        cpu_parallelism,
        load.one,
        load.five,
        load.fifteen,
        &noise_samples,
    )
}

fn stabilized_tasks_per_sec_samples_with<F>(
    rounds: usize,
    warmup_rounds: usize,
    mut sample: F,
) -> ThroughputSampling
where
    F: FnMut() -> f64,
{
    let mut warmup_samples = Vec::<f64>::new();
    for _ in 0..warmup_rounds {
        warmup_samples.push(sample());
    }
    let mut rows = Vec::<f64>::new();
    for _ in 0..rounds.max(1) {
        rows.push(sample());
    }
    let mut sorted = rows.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = percentile(&sorted, 0.50);
    let min = sorted.first().copied().unwrap_or(0.0);
    let max = sorted.last().copied().unwrap_or(0.0);
    ThroughputSampling {
        tasks_per_sec: ((median * 100.0).round()) / 100.0,
        warmup_samples,
        measured_samples: rows,
        stddev: stable_stddev(&sorted),
        min,
        max,
        uncached: SHARED_THROUGHPUT_DEFAULT_UNCACHED,
        workload_seed: String::new(),
    }
}

#[cfg(test)]
fn stabilized_tasks_per_sec_with<F>(rounds: usize, warmup_rounds: usize, sample: F) -> f64
where
    F: FnMut() -> f64,
{
    stabilized_tasks_per_sec_samples_with(rounds, warmup_rounds, sample).tasks_per_sec
}

fn stabilized_tasks_per_sec(rounds: usize, sample_ms: u64, uncached: bool) -> ThroughputSampling {
    let workload_seed = if uncached {
        format!("live_hash_workload_v1_uncached_seed_{}", now_iso())
    } else {
        "live_hash_workload_v1_cached".to_string()
    };
    let mut sample_idx = 0usize;
    let mut sampling =
        stabilized_tasks_per_sec_samples_with(rounds, SHARED_THROUGHPUT_WARMUP_ROUNDS, || {
            if uncached {
                pre_sample_cache_bust(workload_seed.as_str(), sample_idx);
            }
            sample_idx = sample_idx.saturating_add(1);
            live_tasks_per_sec(sample_ms)
        });
    sampling.uncached = uncached;
    sampling.workload_seed = workload_seed;
    sampling
}

fn attach_shared_throughput(measured: &mut Map<String, Value>, tasks_per_sec: f64) {
    measured.insert("tasks_per_sec".to_string(), json!(tasks_per_sec));
    measured.insert(
        "effective_hash_ops_per_sec".to_string(),
        json!(effective_hash_ops_per_sec(tasks_per_sec)),
    );
    measured.insert(
        "throughput_work_factor".to_string(),
        json!(SHARED_THROUGHPUT_WORK_FACTOR),
    );
    measured.insert(
        "throughput_source".to_string(),
        Value::String(SHARED_THROUGHPUT_SOURCE.to_string()),
    );
}
