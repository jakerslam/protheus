// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

#[path = "assimilate_kernel_support.rs"]
mod support;

use base64::Engine;
use serde_json::{json, Value};
use std::path::Path;
use std::thread;
use std::time::Duration;
use support::{
    build_receipt_hash, canonical_assimilation_plan, decode_injected_route, maybe_prewarm,
    parse_args, payload_scaffold_for, render_bar, run_core_assimilation, update_metrics, usage,
    Route, RunResult, TargetMetrics, DEFAULT_REALTIME_DURATION_MS, DEFAULT_SHOWCASE_DURATION_MS,
    STAGES,
};

fn emit_stage_snapshot(total_ms: u64, include_final: bool) {
    let end = if include_final {
        STAGES.len()
    } else {
        STAGES.len() - 1
    };
    for stage in STAGES.iter().take(end) {
        let elapsed_ms = ((total_ms as f64) * (stage.percent as f64 / 100.0)).round() as u64;
        println!("{}", stage.label);
        println!(
            "{} {:>3}%   ({:.1} seconds elapsed)\n",
            render_bar(stage.percent),
            stage.percent,
            elapsed_ms as f64 / 1000.0
        );
    }
}

fn emit_showcase_progress(total_ms: u64) {
    let mut elapsed_ms = 0u64;
    for stage in STAGES.iter().take(STAGES.len() - 1) {
        let stage_ms = (stage.weight * total_ms as f64).round() as u64;
        elapsed_ms += stage_ms;
        println!("{}", stage.label);
        println!(
            "{} {:>3}%   ({:.1} seconds elapsed)\n",
            render_bar(stage.percent),
            stage.percent,
            elapsed_ms as f64 / 1000.0
        );
        if stage_ms > 0 {
            thread::sleep(Duration::from_millis(stage_ms));
        }
    }
}

fn print_final_success(target: &str, receipt: &str, metrics: &TargetMetrics, elapsed_ms: u64) {
    let final_stage = STAGES[STAGES.len() - 1];
    println!("{}", final_stage.label);
    println!(
        "{} {:>3}%   ({:.1} seconds elapsed)\n",
        render_bar(final_stage.percent),
        final_stage.percent,
        elapsed_ms as f64 / 1000.0
    );
    println!("Receipt: {receipt}");
    println!("Target: {target} fully assimilated. Agents online.");
    println!(
        "Latency: {} ms (p50={} ms, p95={} ms)",
        metrics.last_latency_ms, metrics.p50_ms, metrics.p95_ms
    );
    println!("\nPower to The Users.");
}

fn print_failure(target: &str, run: &RunResult) {
    let detail = run
        .payload
        .clone()
        .unwrap_or_else(|| json!({"ok":false,"error":"assimilation_failed"}));
    let out = json!({
        "ok": false,
        "type": "assimilate_failure",
        "target": target,
        "latency_ms": run.latency_ms,
        "status": run.status,
        "detail": detail
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&out)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn route_or_fallback(route: Option<Route>) -> Route {
    route.unwrap_or(Route {
        domain: String::new(),
        args: Vec::new(),
    })
}

fn runtime_receipt_fallback(payload: Option<&Value>, target: &str, ts_iso: &str) -> String {
    payload
        .and_then(|row| row.get("receipt_hash"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| build_receipt_hash(target, ts_iso))
}

fn print_pretty_json(value: &Value, stderr: bool) {
    let out = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    if stderr {
        eprintln!("{out}");
    } else {
        println!("{out}");
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let options = parse_args(argv);
    if options.help {
        usage();
        return 0;
    }
    if options.target.is_empty() {
        usage();
        return 1;
    }

    let target = options.target.clone();
    let route = match decode_injected_route(&options) {
        Ok(row) => row,
        Err(err) => {
            eprintln!(
                "{}",
                json!({"ok":false,"type":"assimilate_cli_error","error":err})
            );
            return 1;
        }
    };

    if options.scaffold_payload {
        let payload = payload_scaffold_for(&target);
        let payload_base64 = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(&payload).unwrap_or_default());
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "type": "assimilate_payload_scaffold",
                "target": target,
                "route": route,
                "payload": payload,
                "payload_base64": payload_base64
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
        return 0;
    }

    maybe_prewarm(root, options.prewarm);
    let ts_iso = crate::now_iso();
    let display_ms = options.duration_ms.unwrap_or(if options.showcase {
        DEFAULT_SHOWCASE_DURATION_MS
    } else {
        DEFAULT_REALTIME_DURATION_MS
    });

    if route.is_none() {
        let plan = canonical_assimilation_plan(
            &target,
            None,
            &ts_iso,
            "unadmitted",
            &options.hard_selector,
            options.selector_bypass,
        );
        if !options.allow_local_simulation {
            let out = json!({
                "ok": false,
                "type": "assimilate_unadmitted_target",
                "target": target,
                "reason": "unknown_target_requires_admission",
                "admission_verdict": "unadmitted",
                "next_steps": [
                    "Use a known governed target or provide a routed core lane payload.",
                    "If you need local simulation for testing, rerun with --allow-local-simulation=1."
                ],
                "canonical_plan": plan,
                "ts": ts_iso
            });
            if options.json {
                print_pretty_json(&out, false);
            } else {
                print_pretty_json(&out, true);
            }
            return 1;
        }
        if options.plan_only {
            let out = json!({
                "ok": true,
                "type": "assimilate_plan",
                "mode": "simulation",
                "target": target,
                "canonical_plan": canonical_assimilation_plan(
                    &target,
                    None,
                    &ts_iso,
                    "simulated",
                    &options.hard_selector,
                    options.selector_bypass,
                ),
                "ts": ts_iso
            });
            print_pretty_json(&out, false);
            return 0;
        }
        let receipt = build_receipt_hash(&target, &ts_iso);
        if !options.json {
            if display_ms > 0 {
                emit_showcase_progress(display_ms);
            } else {
                emit_stage_snapshot(0, false);
            }
        }
        let metrics = update_metrics(root, &target, display_ms, true);
        if options.json {
            print_pretty_json(
                &json!({
                    "ok": true,
                    "type": "assimilate_progress",
                    "mode": "simulation",
                    "admission_verdict": "simulated",
                    "target": target,
                    "receipt": receipt,
                    "latency_ms": metrics.last_latency_ms,
                    "metrics": metrics,
                    "ts": ts_iso,
                    "motto": "Power to The Users."
                }),
                false,
            );
            return 0;
        }
        print_final_success(&target, &receipt, &metrics, display_ms);
        return 0;
    }

    let route = route_or_fallback(route);
    let plan = canonical_assimilation_plan(
        &target,
        Some(&route),
        &ts_iso,
        "admitted",
        &options.hard_selector,
        options.selector_bypass,
    );
    if options.plan_only {
        let out = json!({
            "ok": true,
            "type": "assimilate_plan",
            "mode": "runtime",
            "target": target,
            "route": route,
            "canonical_plan": plan,
            "ts": ts_iso
        });
        print_pretty_json(&out, false);
        return 0;
    }
    let run_result = if display_ms > 0 && !options.json {
        let root_clone = root.to_path_buf();
        let domain = route.domain.clone();
        let args = route.args.clone();
        let handle = thread::spawn(move || run_core_assimilation(&root_clone, &domain, &args));
        emit_showcase_progress(display_ms);
        handle.join().unwrap_or(RunResult {
            status: 1,
            latency_ms: display_ms,
            payload: None,
            stderr: "assimilation_worker_join_failed".to_string(),
        })
    } else {
        run_core_assimilation(root, &route.domain, &route.args)
    };
    let metrics = update_metrics(root, &target, run_result.latency_ms, run_result.status == 0);
    let receipt = runtime_receipt_fallback(run_result.payload.as_ref(), &target, &ts_iso);

    if options.json {
        print_pretty_json(
            &json!({
                "ok": run_result.status == 0,
                "type": "assimilate_execution",
                "mode": "runtime",
                "target": target,
                "route": route,
                "latency_ms": run_result.latency_ms,
                "receipt": receipt,
                "metrics": metrics,
                "canonical_plan": plan,
                "payload": run_result.payload,
                "stderr": if run_result.status == 0 { "" } else { run_result.stderr.trim() },
                "ts": ts_iso
            }),
            false,
        );
        return if run_result.status == 0 { 0 } else { 1 };
    }

    emit_stage_snapshot(run_result.latency_ms, false);
    if run_result.status != 0 {
        print_failure(&target, &run_result);
        return 1;
    }
    print_final_success(&target, &receipt, &metrics, run_result.latency_ms);
    0
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "infring_assimilate_kernel_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(&root);
        root
    }

    #[test]
    fn unknown_target_is_unadmitted_by_default() {
        let root = temp_root();
        let code = run(
            &root,
            &[
                "workflow://definitely-unknown".to_string(),
                "--json=1".to_string(),
            ],
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn unknown_target_can_opt_in_local_simulation() {
        let root = temp_root();
        let code = run(
            &root,
            &[
                "workflow://definitely-unknown".to_string(),
                "--json=1".to_string(),
                "--allow-local-simulation=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }
}
