
fn parse_last_json_object(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    for line in trimmed.lines().rev() {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(row) {
            return Some(value);
        }
    }
    None
}

fn ensure_state_dir(root: &Path) {
    let _ = fs::create_dir_all(root.join(STATE_DIR_REL));
}

fn read_metrics(path: &Path) -> MetricsState {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<MetricsState>(&raw).ok())
        .unwrap_or_default()
}

fn write_metrics(path: &Path, metrics: &MetricsState) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(metrics) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as isize - 1;
    let bounded = idx.clamp(0, sorted.len() as isize - 1) as usize;
    sorted[bounded]
}

pub fn update_metrics(root: &Path, target: &str, latency_ms: u64, ok: bool) -> TargetMetrics {
    let metrics_path = root.join(METRICS_STATE_REL);
    let mut metrics = read_metrics(&metrics_path);
    let row = metrics.targets.entry(target.to_string()).or_default();
    row.count += 1;
    if ok {
        row.ok_count += 1;
    } else {
        row.fail_count += 1;
    }
    row.last_latency_ms = latency_ms;
    row.updated_at = now_iso();
    if ok {
        row.latencies_ms.push(latency_ms);
        if row.latencies_ms.len() > 200 {
            let keep_from = row.latencies_ms.len() - 200;
            row.latencies_ms = row.latencies_ms.split_off(keep_from);
        }
        let mut sorted = row.latencies_ms.clone();
        sorted.sort_unstable();
        row.p50_ms = percentile(&sorted, 50);
        row.p95_ms = percentile(&sorted, 95);
    }
    let out = row.clone();
    write_metrics(&metrics_path, &metrics);
    out
}

pub fn maybe_prewarm(root: &Path, enabled: bool) {
    if !enabled {
        return;
    }
    let path = root.join(PREWARM_STATE_REL);
    let state = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<PrewarmState>(&raw).ok())
        .unwrap_or_default();
    let now_ms = chrono::Utc::now().timestamp_millis();
    if now_ms - state.ts_ms < DEFAULT_PREWARM_TTL_MS {
        return;
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("infring-ops"));
    let _ = Command::new(exe)
        .current_dir(root)
        .arg("health-status")
        .arg("status")
        .arg("--fast=1")
        .output();
    ensure_state_dir(root);
    let next = PrewarmState {
        ts_ms: now_ms,
        ts: now_iso(),
    };
    if let Ok(raw) = serde_json::to_string_pretty(&next) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

pub fn run_core_assimilation(root: &Path, domain: &str, args: &[String]) -> RunResult {
    let start = Instant::now();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("infring-ops"));
    match Command::new(exe)
        .current_dir(root)
        .arg(domain)
        .args(args)
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            RunResult {
                status: out.status.code().unwrap_or(1),
                latency_ms: start.elapsed().as_millis() as u64,
                payload: parse_last_json_object(&stdout),
                stderr,
            }
        }
        Err(err) => RunResult {
            status: 1,
            latency_ms: start.elapsed().as_millis() as u64,
            payload: None,
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

pub fn render_bar(percent: u32) -> String {
    let bounded = percent.clamp(0, 100) as f64;
    let filled = ((bounded / 100.0) * BAR_WIDTH as f64).round() as usize;
    format!(
        "[{}{}]",
        FILLED_CHAR.to_string().repeat(filled),
        EMPTY_CHAR
            .to_string()
            .repeat(BAR_WIDTH.saturating_sub(filled))
    )
}
