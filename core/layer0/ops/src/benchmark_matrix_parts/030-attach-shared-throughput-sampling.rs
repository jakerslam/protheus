fn attach_shared_throughput_sampling(
    measured: &mut Map<String, Value>,
    sampling: &ThroughputSampling,
) {
    measured.insert(
        "throughput_sampling_warmup_ops_per_sec".to_string(),
        Value::Array(
            sampling
                .warmup_samples
                .iter()
                .map(|value| json!(((value * 100.0).round()) / 100.0))
                .collect(),
        ),
    );
    measured.insert(
        "throughput_sampling_measured_ops_per_sec".to_string(),
        Value::Array(
            sampling
                .measured_samples
                .iter()
                .map(|value| json!(((value * 100.0).round()) / 100.0))
                .collect(),
        ),
    );
    measured.insert(
        "throughput_sampling_stddev_ops_per_sec".to_string(),
        json!(((sampling.stddev * 100.0).round()) / 100.0),
    );
    measured.insert(
        "throughput_sampling_min_ops_per_sec".to_string(),
        json!(((sampling.min * 100.0).round()) / 100.0),
    );
    measured.insert(
        "throughput_sampling_max_ops_per_sec".to_string(),
        json!(((sampling.max * 100.0).round()) / 100.0),
    );
    measured.insert(
        "throughput_uncached".to_string(),
        Value::Bool(sampling.uncached),
    );
    measured.insert(
        "throughput_workload_seed".to_string(),
        Value::String(clean(&sampling.workload_seed, 160)),
    );
}

fn runtime_metrics(
    root: &Path,
    refresh_runtime: bool,
) -> Result<(f64, f64, f64, Value, Value), String> {
    let mut source = "status".to_string();
    let mut fallback_reason = Value::Null;
    let mut runtime_json = Value::Null;

    if refresh_runtime {
        let args = vec!["run".to_string(), "--strict=0".to_string()];
        let parsed = parse_args(&args);
        match run_runtime_efficiency_floor(root, &parsed) {
            Ok(out) => {
                if extract_runtime_metrics(&out.json).is_some() {
                    source = "run".to_string();
                    runtime_json = out.json;
                } else {
                    fallback_reason =
                        Value::String("runtime_efficiency_run_missing_metrics".to_string());
                }
            }
            Err(err) => {
                fallback_reason = Value::String(format!("runtime_efficiency_run_failed:{err}"));
            }
        }
    }

    if runtime_json.is_null() {
        let args = vec!["status".to_string()];
        let parsed = parse_args(&args);
        runtime_json = status_runtime_efficiency_floor(root, &parsed).json;
    }

    if let Some((cold_start_ms, idle_memory_mb, install_size_mb)) =
        extract_runtime_metrics(&runtime_json)
    {
        let source_meta = json!({
            "mode": source,
            "refresh_requested": refresh_runtime,
            "fallback_reason": fallback_reason
        });
        return Ok((
            cold_start_ms,
            idle_memory_mb,
            install_size_mb,
            runtime_json,
            source_meta,
        ));
    }

    let local_install_size_mb = local_full_install_probe_mb(root);

    let top1_snapshot_path = root.join(TOP1_BENCHMARK_SNAPSHOT_REL);
    if let Ok(top1_snapshot) = read_json(&top1_snapshot_path) {
        if let Some((cold_start_ms, idle_memory_mb, snapshot_install_size_mb)) =
            extract_top1_snapshot_metrics(&top1_snapshot)
        {
            let install_size_mb = local_install_size_mb.unwrap_or(snapshot_install_size_mb);
            let source_meta = json!({
                "mode": if local_install_size_mb.is_some() {
                    "top1_benchmark_snapshot_with_local_install_probe"
                } else {
                    "top1_benchmark_snapshot"
                },
                "refresh_requested": refresh_runtime,
                "fallback_reason": if fallback_reason.is_null() {
                    Value::String("runtime_efficiency_missing_metrics".to_string())
                } else {
                    fallback_reason
                },
                "snapshot_path": TOP1_BENCHMARK_SNAPSHOT_REL,
                "install_source": if local_install_size_mb.is_some() {
                    Value::String("local_full_install_probe".to_string())
                } else {
                    Value::String("top1_snapshot".to_string())
                }
            });
            return Ok((
                cold_start_ms,
                idle_memory_mb,
                install_size_mb,
                top1_snapshot,
                source_meta,
            ));
        }
    }

    Err("runtime_efficiency_missing_metrics".to_string())
}

fn measure_infring(
    root: &Path,
    refresh_runtime: bool,
    throughput_sampling: &ThroughputSampling,
) -> Result<(Map<String, Value>, Value), String> {
    let (cold_start_ms, idle_memory_mb, install_size_mb, mut runtime_json, mut runtime_source) =
        runtime_metrics(root, refresh_runtime)?;
    let security_systems = count_guard_checks(root)?;
    let channel_adapters = count_channel_adapters(root)?;
    let llm_providers = count_llm_providers(root)?;
    let data_channels = count_data_channels(root)?;
    let plugin_marketplace_checks = count_plugin_marketplace_checks(root)?;
    let security_policy_checks_total = count_policy_checks_total(root)?;
    let mut measured = Map::<String, Value>::new();
    measured.insert("cold_start_ms".to_string(), json!(cold_start_ms));
    measured.insert("idle_memory_mb".to_string(), json!(idle_memory_mb));
    measured.insert("install_size_mb".to_string(), json!(install_size_mb));
    attach_shared_throughput(&mut measured, throughput_sampling.tasks_per_sec);
    attach_shared_throughput_sampling(&mut measured, throughput_sampling);
    measured.insert("security_systems".to_string(), json!(security_systems));
    measured.insert("channel_adapters".to_string(), json!(channel_adapters));
    measured.insert("llm_providers".to_string(), json!(llm_providers));
    measured.insert(
        "security_merge_guard_checks".to_string(),
        json!(security_systems),
    );
    measured.insert("platform_adapters".to_string(), json!(channel_adapters));
    measured.insert("data_channels".to_string(), json!(data_channels));
    measured.insert(
        "plugin_marketplace_checks".to_string(),
        json!(plugin_marketplace_checks),
    );
    measured.insert(
        "security_policy_checks_total".to_string(),
        json!(security_policy_checks_total),
    );
    measured.insert("measured".to_string(), Value::Bool(true));
    measured.insert(
        "data_source".to_string(),
        Value::String("runtime_efficiency_floor + policy counters".to_string()),
    );
    measured.insert(
        "counter_definitions".to_string(),
        benchmark_counter_definitions(),
    );
    if let Some(metrics) = runtime_json
        .get_mut("metrics")
        .and_then(Value::as_object_mut)
    {
        metrics.insert(
            "tasks_per_sec".to_string(),
            json!(throughput_sampling.tasks_per_sec),
        );
        metrics.insert(
            "effective_hash_ops_per_sec".to_string(),
            json!(effective_hash_ops_per_sec(
                throughput_sampling.tasks_per_sec
            )),
        );
    }
    if let Some(meta) = runtime_source.as_object_mut() {
        meta.insert(
            "tasks_source".to_string(),
            Value::String(SHARED_THROUGHPUT_SOURCE.to_string()),
        );
        meta.insert(
            "tasks_uncached".to_string(),
            Value::Bool(throughput_sampling.uncached),
        );
        meta.insert(
            "tasks_workload_seed".to_string(),
            Value::String(clean(&throughput_sampling.workload_seed, 160)),
        );
        meta.insert(
            "tasks_sample_ms".to_string(),
            json!(SHARED_THROUGHPUT_SAMPLE_MS),
        );
        meta.insert(
            "tasks_work_factor".to_string(),
            json!(SHARED_THROUGHPUT_WORK_FACTOR),
        );
        meta.insert(
            "effective_hash_ops_per_sec".to_string(),
            json!(effective_hash_ops_per_sec(
                throughput_sampling.tasks_per_sec
            )),
        );
        meta.insert(
            "tasks_phase".to_string(),
            Value::String("pre_profile_sampling_shared".to_string()),
        );
        meta.insert("tasks_rounds".to_string(), json!(SHARED_THROUGHPUT_ROUNDS));
        meta.insert(
            "tasks_warmup_rounds".to_string(),
            json!(SHARED_THROUGHPUT_WARMUP_ROUNDS),
        );
    }
    measured.insert("runtime_metric_source".to_string(), runtime_source);

    Ok((measured, runtime_json))
}

fn merge_projects(
    snapshot: &Value,
    infring_measured: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let base_projects = snapshot
        .get("projects")
        .and_then(Value::as_object)
        .ok_or_else(|| "benchmark_snapshot_missing_projects".to_string())?;

    let mut projects = base_projects.clone();
    projects.insert(
        "Infring".to_string(),
        Value::Object(infring_measured.clone()),
    );
    Ok(projects)
}

fn metric_value(project: &Map<String, Value>, category_key: &str) -> Option<f64> {
    project.get(category_key).and_then(Value::as_f64)
}

fn bar_fill(value: f64, min: f64, max: f64, width: usize, lower_is_better: bool) -> usize {
    if width == 0 {
        return 0;
    }
    if (max - min).abs() < f64::EPSILON {
        return width;
    }
    let mut norm = (value - min) / (max - min);
    if lower_is_better {
        norm = 1.0 - norm;
    }
    let clamped = norm.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    filled.clamp(1, width)
}

fn render_bar(width: usize, fill: usize) -> String {
    format!(
        "{}{}",
        "#".repeat(fill),
        "-".repeat(width.saturating_sub(fill))
    )
}

fn format_metric_value(category: Category, value: f64) -> String {
    match category.key {
        "cold_start_ms" => {
            if value >= 1000.0 {
                format!("{:.2} sec", value / 1000.0)
            } else {
                format!("{value:.0} {}", category.unit)
            }
        }
        "idle_memory_mb" | "install_size_mb" => format!("{value:.1} {}", category.unit),
        "tasks_per_sec" => format!("{value:.1} {}", category.unit),
        _ => format!("{value:.0}"),
    }
}

fn category_report(
    category: Category,
    projects: &Map<String, Value>,
    bar_width: usize,
) -> Result<Value, String> {
    let mut rows = Vec::<(String, f64, bool)>::new();
    for (name, entry) in projects {
        let Some(project) = entry.as_object() else {
            continue;
        };
        let Some(value) = metric_value(project, category.key) else {
            continue;
        };
        let highlight = project
            .get("highlight")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        rows.push((name.clone(), value, highlight));
    }
    if rows.is_empty() {
        return Err(format!(
            "benchmark_category_missing_values:{}",
            category.key
        ));
    }

    let min = rows
        .iter()
        .map(|(_, value, _)| *value)
        .fold(f64::INFINITY, f64::min);
    let max = rows
        .iter()
        .map(|(_, value, _)| *value)
        .fold(f64::NEG_INFINITY, f64::max);

    if category.lower_is_better {
        rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    let mut report_rows = Vec::<Value>::new();
    let mut lines = Vec::<String>::new();
    lines.push(category.label.to_string());

    for (idx, (name, value, highlight)) in rows.iter().enumerate() {
        let fill = bar_fill(*value, min, max, bar_width, category.lower_is_better);
        let bar = render_bar(bar_width, fill);
        let score = format_metric_value(category, *value);
        let marker = if *highlight { " *" } else { "" };
        lines.push(format!("{:<10} {}  {}{}", name, bar, score, marker));

        report_rows.push(json!({
            "rank": idx + 1,
            "project": name,
            "value": value,
            "bar": bar,
            "highlight": highlight,
            "score": score
        }));
    }

    Ok(json!({
        "key": category.key,
        "label": category.label,
        "lower_is_better": category.lower_is_better,
        "unit": category.unit,
        "bar_width": bar_width,
        "rows": report_rows,
        "ascii_lines": lines
    }))
}
