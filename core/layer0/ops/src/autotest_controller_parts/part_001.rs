fn runtime_paths(root: &Path) -> RuntimePaths {
    let state_dir = std::env::var("AUTOTEST_STATE_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("local/state/ops/autotest"));

    let default_pain_signals_path = root.join("local/state/autonomy/pain_signals.jsonl");

    RuntimePaths {
        policy_path: std::env::var("AUTOTEST_POLICY_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL)),
        state_dir: state_dir.clone(),
        registry_path: state_dir.join("registry.json"),
        status_path: state_dir.join("status.json"),
        events_path: state_dir.join("events.jsonl"),
        latest_path: state_dir.join("latest.json"),
        reports_dir: state_dir.join("reports"),
        runs_dir: state_dir.join("runs"),
        module_root: std::env::var("AUTOTEST_MODULE_ROOT")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join("systems")),
        test_root: std::env::var("AUTOTEST_TEST_ROOT")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join("tests/client-memory-tools")),
        spine_runs_dir: std::env::var("AUTOTEST_SPINE_RUNS_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join("local/state/spine/runs")),
        pain_signals_path: std::env::var("AUTOTEST_PAIN_SIGNALS_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or(default_pain_signals_path),
    }
}

fn load_policy(root: &Path, policy_path: &Path) -> Policy {
    let mut out = default_policy();
    let raw = read_json(policy_path);
    if !raw.is_object() {
        return out;
    }

    if let Some(v) = raw.get("version").and_then(Value::as_str) {
        let clean = normalize_token(v, 24);
        if !clean.is_empty() {
            out.version = clean;
        }
    }
    out.enabled = raw
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(out.enabled);
    out.strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(out.strict_default);

    if let Some(module_discovery) = raw.get("module_discovery") {
        out.module_include_ext = module_discovery
            .get("include_ext")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|rows| !rows.is_empty())
            .unwrap_or_else(|| out.module_include_ext.clone());
        out.module_ignore_prefixes = module_discovery
            .get("ignore_prefixes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| out.module_ignore_prefixes.clone());
    }

    if let Some(test_discovery) = raw.get("test_discovery") {
        if let Some(sfx) = test_discovery.get("include_suffix").and_then(Value::as_str) {
            let sfx = sfx.trim();
            if !sfx.is_empty() {
                out.test_include_suffix = sfx.to_string();
            }
        }
        out.test_ignore_prefixes = test_discovery
            .get("ignore_prefixes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| out.test_ignore_prefixes.clone());
    }

    if let Some(heuristics) = raw.get("heuristics") {
        out.min_match_score = heuristics
            .get("min_match_score")
            .and_then(Value::as_i64)
            .unwrap_or(out.min_match_score);
        out.min_token_len = heuristics
            .get("min_token_len")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(out.min_token_len);
        out.shared_token_score = heuristics
            .get("shared_token_score")
            .and_then(Value::as_i64)
            .unwrap_or(out.shared_token_score);
        out.basename_contains_score = heuristics
            .get("basename_contains_score")
            .and_then(Value::as_i64)
            .unwrap_or(out.basename_contains_score);
        out.layer_hint_score = heuristics
            .get("layer_hint_score")
            .and_then(Value::as_i64)
            .unwrap_or(out.layer_hint_score);
    }

    if let Some(explicit_maps) = raw
        .get("explicit_maps")
        .and_then(|v| v.get("by_prefix"))
        .and_then(Value::as_object)
    {
        let mut maps = BTreeMap::new();
        for (k, v) in explicit_maps {
            let rows = v
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            maps.insert(k.to_string(), rows);
        }
        out.explicit_prefix_maps = maps;
    }

    if let Some(commands) = raw.get("critical_commands").and_then(Value::as_array) {
        let rows = commands
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            out.critical_commands = rows;
        }
    }

    if let Some(execution) = raw.get("execution") {
        out.execution.default_scope = execution
            .get("default_scope")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .filter(|s| ["critical", "changed", "all"].contains(&s.as_str()))
            .unwrap_or_else(|| out.execution.default_scope.clone());
        out.execution.strict = execution
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or(out.execution.strict);
        out.execution.max_tests_per_run = execution
            .get("max_tests_per_run")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(out.execution.max_tests_per_run)
            .clamp(1, 500);
        out.execution.run_timeout_ms = execution
            .get("run_timeout_ms")
            .and_then(Value::as_i64)
            .unwrap_or(out.execution.run_timeout_ms)
            .clamp(1_000, 2 * 60 * 60 * 1_000);
        out.execution.timeout_ms_per_test = execution
            .get("timeout_ms_per_test")
            .and_then(Value::as_i64)
            .unwrap_or(out.execution.timeout_ms_per_test)
            .clamp(1_000, 2 * 60 * 60 * 1_000);
        out.execution.retry_flaky_once = execution
            .get("retry_flaky_once")
            .and_then(Value::as_bool)
            .unwrap_or(out.execution.retry_flaky_once);
        out.execution.flaky_quarantine_after = execution
            .get("flaky_quarantine_after")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.execution.flaky_quarantine_after);
        out.execution.flaky_quarantine_sec = execution
            .get("flaky_quarantine_sec")
            .and_then(Value::as_i64)
            .unwrap_or(out.execution.flaky_quarantine_sec)
            .clamp(0, 7 * 24 * 60 * 60);
        out.execution.midrun_resource_guard = execution
            .get("midrun_resource_guard")
            .and_then(Value::as_bool)
            .unwrap_or(out.execution.midrun_resource_guard);
        out.execution.resource_recheck_every_tests = execution
            .get("resource_recheck_every_tests")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(out.execution.resource_recheck_every_tests)
            .clamp(1, 256);
    }

    if let Some(alerts) = raw.get("alerts") {
        out.alerts.emit_untested = alerts
            .get("emit_untested")
            .and_then(Value::as_bool)
            .unwrap_or(out.alerts.emit_untested);
        out.alerts.emit_changed_without_tests = alerts
            .get("emit_changed_without_tests")
            .and_then(Value::as_bool)
            .unwrap_or(out.alerts.emit_changed_without_tests);
        out.alerts.max_untested_in_report = alerts
            .get("max_untested_in_report")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(out.alerts.max_untested_in_report)
            .clamp(1, 400);
        out.alerts.max_failed_in_report = alerts
            .get("max_failed_in_report")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(out.alerts.max_failed_in_report)
            .clamp(1, 400);
    }

    if let Some(runtime_guard) = raw.get("runtime_guard") {
        out.runtime_guard.spine_hot_window_sec = runtime_guard
            .get("spine_hot_window_sec")
            .and_then(Value::as_i64)
            .unwrap_or(out.runtime_guard.spine_hot_window_sec)
            .clamp(5, 24 * 60 * 60);
        out.runtime_guard.max_rss_mb = runtime_guard
            .get("max_rss_mb")
            .and_then(Value::as_f64)
            .unwrap_or(out.runtime_guard.max_rss_mb)
            .clamp(256.0, 256_000.0);
    }

    if let Some(sleep_cfg) = raw.get("sleep_window_local") {
        out.sleep_window_start_hour = sleep_cfg
            .get("start_hour")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.sleep_window_start_hour)
            .clamp(0, 23);
        out.sleep_window_end_hour = sleep_cfg
            .get("end_hour")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.sleep_window_end_hour)
            .clamp(0, 23);
    }

    if let Some(ext) = raw.get("external_health") {
        out.external_health_paths = ext
            .get("sources")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|v| {
                        if let Some(p) = v.get("path").and_then(Value::as_str) {
                            return Some(p.trim().to_string());
                        }
                        v.as_str().map(|s| s.trim().to_string())
                    })
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| out.external_health_paths.clone());
        out.external_health_window_hours = ext
            .get("window_hours")
            .and_then(Value::as_i64)
            .unwrap_or(out.external_health_window_hours)
            .clamp(1, 24 * 30);
    }

    let _ = root;
    out
}

fn ensure_state_dirs(paths: &RuntimePaths) -> Result<(), String> {
    ensure_dir(&paths.state_dir)?;
    ensure_dir(&paths.reports_dir)?;
    ensure_dir(&paths.runs_dir)?;
    if let Some(parent) = paths.events_path.parent() {
        ensure_dir(parent)?;
    }
    if let Some(parent) = paths.latest_path.parent() {
        ensure_dir(parent)?;
    }
    if let Some(parent) = paths.registry_path.parent() {
        ensure_dir(parent)?;
    }
    if let Some(parent) = paths.status_path.parent() {
        ensure_dir(parent)?;
    }
    Ok(())
}

fn load_status(paths: &RuntimePaths) -> StatusState {
    let raw = read_json(&paths.status_path);
    serde_json::from_value::<StatusState>(raw).unwrap_or_else(|_| StatusState {
        version: "1.0".to_string(),
        updated_at: None,
        modules: HashMap::new(),
        tests: HashMap::new(),
        alerts: AlertState::default(),
        last_sync: None,
        last_run: None,
        last_report: None,
    })
}

fn list_files_recursively(root_dir: &Path) -> Vec<PathBuf> {
    if !root_dir.exists() {
        return Vec::new();
    }
    WalkDir::new(root_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn should_ignore_rel(rel: &str, ignore_prefixes: &[String]) -> bool {
    ignore_prefixes.iter().any(|prefix| rel.starts_with(prefix))
}

fn sha256_file(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => stable_hash(&String::from_utf8_lossy(&bytes), 64),
        Err(_) => stable_hash("missing", 64),
    }
}

fn module_candidates(root: &Path, paths: &RuntimePaths, policy: &Policy) -> Vec<ModuleCandidate> {
    let mut out = Vec::new();
    for abs in list_files_recursively(&paths.module_root) {
        let rel = rel_path(root, &abs);
        if should_ignore_rel(&rel, &policy.module_ignore_prefixes) {
            continue;
        }
        if !policy
            .module_include_ext
            .iter()
            .any(|ext| rel.ends_with(ext.as_str()))
        {
            continue;
        }
        let path_name = rel.clone();
        let base = abs
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(ModuleCandidate {
            id: stable_id(&format!("mod|{path_name}"), "mod"),
            path: path_name,
            abs_path: abs,
            basename: base,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn test_candidates(root: &Path, paths: &RuntimePaths, policy: &Policy) -> Vec<TestCandidate> {
    let mut out = Vec::new();
    for abs in list_files_recursively(&paths.test_root) {
        let rel = rel_path(root, &abs);
        if should_ignore_rel(&rel, &policy.test_ignore_prefixes) {
            continue;
        }
        if !rel.ends_with(&policy.test_include_suffix) {
            continue;
        }
        let stem = abs
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(TestCandidate {
            id: stable_id(&format!("tst|{rel}"), "tst"),
            kind: "node_test".to_string(),
            path: rel.clone(),
            command: format!("node {rel}"),
            stem,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn tokenize_name(v: &str, min_len: usize) -> Vec<String> {
    v.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|s| normalize_token(s, 120))
        .filter(|s| !s.is_empty() && s.len() >= min_len)
        .collect()
}

fn layer_hint(rel: &str) -> String {
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.len() >= 3 && parts[0] == "systems" {
        normalize_token(parts[1], 64)
    } else if parts.len() >= 2 {
        normalize_token(parts[0], 64)
    } else {
        String::new()
    }
}
