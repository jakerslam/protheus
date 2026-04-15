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
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("protheus-ops"));
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
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("protheus-ops"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_recon_root(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{prefix}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(root.join("src"));
        let _ = fs::create_dir_all(root.join("tests"));
        root
    }

    #[test]
    fn canonical_plan_blocks_when_hard_selector_does_not_match_route_or_target() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "workflow://other-target",
            false,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("unadmitted"));
        let closure_complete = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_complete"))
            .and_then(Value::as_bool);
        assert_eq!(closure_complete, Some(false));
    }

    #[test]
    fn canonical_plan_blocks_when_selector_bypass_requested() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "",
            true,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("unadmitted"));
    }

    #[test]
    fn canonical_plan_admits_when_route_present_and_controls_satisfied() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "runtime-systems",
            false,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("admitted"));
        let closure_complete = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_complete"))
            .and_then(Value::as_bool);
        assert_eq!(closure_complete, Some(true));
    }

    #[test]
    fn parse_args_accepts_selector_controls() {
        let parsed = parse_args(&[
            "workflow://langgraph".to_string(),
            "--strict=1".to_string(),
            "--hard-selector=runtime-systems".to_string(),
            "--selector-bypass=1".to_string(),
        ]);
        assert_eq!(parsed.target, "workflow://langgraph");
        assert!(parsed.strict);
        assert_eq!(parsed.hard_selector, "runtime-systems");
        assert!(parsed.selector_bypass);
    }

    #[test]
    fn canonical_plan_recon_scans_path_targets_into_surfaces() {
        let root = temp_recon_root("infring_assimilation_recon");
        let _ = fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"demo\"\nversion=\"0.1.0\"\n[dependencies]\nserde = \"1\"\n",
        );
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");
        let _ = fs::write(root.join("LICENSE"), "Apache-2.0\n");
        let _ = fs::write(root.join("tests/demo_test.rs"), "#[test] fn ok() {}\n");
        let _ = fs::write(root.join("openapi.yaml"), "openapi: 3.0.0\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );

        let manifest_count = plan
            .get("recon_index")
            .and_then(|row| row.get("manifest_inventory"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        assert!(manifest_count >= 1);

        let dependency_count = plan
            .get("candidate_closure")
            .and_then(|row| row.get("dependencies"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        assert!(dependency_count >= 1);
        let external_edges = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_stats"))
            .and_then(|row| row.get("dependency_graph_summary"))
            .and_then(|row| row.get("external_edge_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(external_edges >= 1);
        let has_openapi_candidate = plan
            .get("candidate_set")
            .and_then(|row| row.get("targets"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.as_str() == Some("workflow://openapi-service"))
            })
            .unwrap_or(false);
        assert!(has_openapi_candidate);

        let verdict = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(verdict, Some("admitted"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_plan_dependency_graph_detects_internal_manifest_edges() {
        let root = temp_recon_root("infring_assimilation_internal_graph");
        let _ = fs::create_dir_all(root.join("crates/child/src"));
        let _ = fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"root-demo\"\nversion=\"0.1.0\"\n[dependencies]\nchild = { path = \"crates/child\" }\n",
        );
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");
        let _ = fs::write(
            root.join("crates/child/Cargo.toml"),
            "[package]\nname=\"child\"\nversion=\"0.1.0\"\n",
        );
        let _ = fs::write(root.join("crates/child/src/lib.rs"), "pub fn child() {}\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );

        let internal_edges = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_stats"))
            .and_then(|row| row.get("dependency_graph_summary"))
            .and_then(|row| row.get("internal_edge_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(internal_edges >= 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_plan_emits_manifest_blocker_for_path_target_without_manifests() {
        let root = temp_recon_root("infring_assimilation_recon_blocker");
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );
        let verdict = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(verdict, Some("unadmitted"));

        let has_manifest_blocker = plan
            .get("provisional_gap_report")
            .and_then(|row| row.get("gaps"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("gap_id").and_then(Value::as_str)
                        == Some("assimilation_manifest_surface_missing")
                        && row.get("severity").and_then(Value::as_str) == Some("blocker")
                })
            })
            .unwrap_or(false);
        assert!(has_manifest_blocker);

        let _ = fs::remove_dir_all(root);
    }
}


