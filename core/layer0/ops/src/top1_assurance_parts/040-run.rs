fn provider_contract_env_keys(provider: &str) -> &'static [&'static str] {
    match provider {
        "openai" => &["OPENAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        "xai" => &["XAI_API_KEY"],
        "tts" => &["ELEVENLABS_API_KEY", "OPENAI_API_KEY"],
        _ => &[],
    }
}

fn provider_contract_present(snapshot: &Value, provider: &str) -> bool {
    let needle = provider.to_ascii_lowercase();
    let compact = snapshot.to_string().to_ascii_lowercase();
    if compact.contains(&format!("\"{needle}\"")) {
        return true;
    }
    for pointer in [
        "/provider_family_contract_suite_contract/providers",
        "/provider_contract_suite_contract/providers",
        "/provider_runtime_core_contract/providers",
        "/providers",
        "/provider_inventory/providers",
    ] {
        let Some(value) = snapshot.pointer(pointer) else {
            continue;
        };
        match value {
            Value::Array(rows) => {
                let hit = rows.iter().any(|row| {
                    row.as_str()
                        .map(|v| v.eq_ignore_ascii_case(provider))
                        .or_else(|| {
                            row.as_object().map(|obj| {
                                ["id", "provider", "family", "name"]
                                    .into_iter()
                                    .filter_map(|key| obj.get(key))
                                    .filter_map(Value::as_str)
                                    .any(|v| v.eq_ignore_ascii_case(provider))
                            })
                        })
                        .unwrap_or(false)
                });
                if hit {
                    return true;
                }
            }
            Value::Object(obj) => {
                if obj.keys().any(|key| key.eq_ignore_ascii_case(provider)) {
                    return true;
                }
            }
            Value::String(raw) => {
                if raw.eq_ignore_ascii_case(provider) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn run_tooling_contracts(
    root: &Path,
    _policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let snapshot_rel = parsed
        .flags
        .get("runtime-snapshot-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("local/state/ops/web_tooling/runtime_snapshot/latest.json");
    let snapshot_path = root.join(snapshot_rel);
    let snapshot = read_json(&snapshot_path).unwrap_or(Value::Null);

    let required = ["openai", "openrouter", "xai", "tts"];
    let mut provider_checks = Vec::<Value>::new();
    let mut errors = Vec::<String>::new();
    if snapshot.is_null() {
        errors.push("tooling_contract_snapshot_missing".to_string());
    }

    for provider in required {
        let contract_present = !snapshot.is_null() && provider_contract_present(&snapshot, provider);
        let env_keys = provider_contract_env_keys(provider);
        let auth_present = env_keys.iter().any(|key| {
            std::env::var(key)
                .ok()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        });
        let ok = contract_present && auth_present;
        if !ok {
            errors.push(format!("tooling_contract_provider_{provider}_missing"));
        }
        provider_checks.push(json!({
            "provider": provider,
            "contract_present": contract_present,
            "auth_present": auth_present,
            "auth_env_keys": env_keys
        }));
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "snapshot_path": snapshot_rel,
        "provider_checks": provider_checks,
        "errors": errors
    })
}

fn run_comparison_matrix(
    root: &Path,
    policy: &Top1Policy,
    strict: bool,
    parsed: &crate::ParsedArgs,
) -> Value {
    let snapshot_rel = parsed
        .flags
        .get("snapshot-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.comparison.snapshot_path.as_str());
    let output_rel = parsed
        .flags
        .get("output-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.comparison.output_path.as_str());
    let benchmark_rel = parsed
        .flags
        .get("benchmark-path")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(policy.benchmark.benchmark_path.as_str());
    let apply = parse_bool(parsed.flags.get("apply"), true);

    let snapshot_path = root.join(snapshot_rel);
    let output_path = root.join(output_rel);
    let benchmark_path = root.join(benchmark_rel);

    let snapshot = read_json(&snapshot_path).unwrap_or(Value::Null);
    let benchmark = read_json(&benchmark_path).unwrap_or(Value::Null);

    let mut errors = Vec::<String>::new();
    if snapshot.is_null() {
        errors.push("comparison_snapshot_missing".to_string());
    }

    let mut projects = snapshot
        .get("projects")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if projects.is_empty() {
        errors.push("comparison_snapshot_projects_missing".to_string());
    }

    let metrics = benchmark.get("metrics").cloned().unwrap_or(Value::Null);
    let infring = json!({
        "cold_start_ms": metrics.get("cold_start_ms").and_then(Value::as_f64),
        "idle_memory_mb": metrics.get("idle_rss_mb").and_then(Value::as_f64),
        "install_size_mb": metrics.get("install_size_mb").and_then(Value::as_f64),
        "tasks_per_sec": metrics.get("tasks_per_sec").and_then(Value::as_f64)
    });
    projects.insert("Protheus".to_string(), infring);

    let generated_at = now_iso();
    let markdown = render_matrix_markdown(&generated_at, &projects, benchmark_rel, snapshot_rel);

    if apply {
        if let Some(parent) = output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&output_path, markdown.as_bytes()).is_err() {
            errors.push("comparison_output_write_failed".to_string());
        }
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "snapshot_path": snapshot_rel,
        "benchmark_path": benchmark_rel,
        "output_path": output_rel,
        "apply": apply,
        "project_count": projects.len(),
        "errors": errors
    })
}

fn run_status(root: &Path, policy: &Top1Policy) -> Value {
    let latest = read_json(&root.join(&policy.outputs.latest_path));
    json!({
        "ok": true,
        "type": "top1_assurance_status",
        "lane": LANE_ID,
        "ts": now_iso(),
        "latest_path": policy.outputs.latest_path,
        "history_path": policy.outputs.history_path,
        "has_latest": latest.is_some(),
        "latest": latest
    })
}

fn wrap_receipt(
    root: &Path,
    policy: &Top1Policy,
    command: &str,
    strict: bool,
    payload: Value,
    write_state: bool,
) -> Value {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let mut out = json!({
        "ok": ok,
        "type": "top1_assurance",
        "lane": LANE_ID,
        "command": command,
        "strict": strict,
        "ts": now_iso(),
        "payload": payload,
        "claim_evidence": [
            {
                "id": "top1_assurance_lane",
                "claim": "top1_assurance_contracts_emit_deterministic_receipts",
                "evidence": {
                    "command": command,
                    "strict": strict
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));

    if write_state {
        let latest_path = root.join(&policy.outputs.latest_path);
        let history_path = root.join(&policy.outputs.history_path);
        let _ = write_json(&latest_path, &out);
        let _ = append_jsonl(&history_path, &out);
    }

    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|arg| matches!(arg.as_str(), "help" | "--help" | "-h"))
    {
        usage();
        return 0;
    }

    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let policy_rel = parsed
        .flags
        .get("policy")
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(DEFAULT_POLICY_REL);
    let policy_path = root.join(policy_rel);
    let policy = load_policy(root, &policy_path);
    let strict = parse_bool(parsed.flags.get("strict"), policy.strict_default);

    let payload = match command.as_str() {
        "status" => run_status(root, &policy),
        "proof-coverage" => run_proof_coverage(root, &policy, strict, &parsed),
        "proof-vm" => run_proof_vm(root, &policy, strict, &parsed),
        "size-gate" => run_size_gate(root, &policy, strict, &parsed),
        "benchmark-thresholds" => run_benchmark_thresholds(root, &policy, strict, &parsed),
        "tooling-contracts" => run_tooling_contracts(root, &policy, strict, &parsed),
        "comparison-matrix" => run_comparison_matrix(root, &policy, strict, &parsed),
        "run-all" => {
            let proof = run_proof_coverage(root, &policy, strict, &parsed);
            let vm = run_proof_vm(root, &policy, strict, &parsed);
            let size = run_size_gate(root, &policy, strict, &parsed);
            let bench = run_benchmark_thresholds(root, &policy, strict, &parsed);
            let tooling = run_tooling_contracts(root, &policy, strict, &parsed);
            let compare = run_comparison_matrix(root, &policy, strict, &parsed);
            let ok = [
                proof.get("ok").and_then(Value::as_bool).unwrap_or(false),
                vm.get("ok").and_then(Value::as_bool).unwrap_or(false),
                size.get("ok").and_then(Value::as_bool).unwrap_or(false),
                bench.get("ok").and_then(Value::as_bool).unwrap_or(false),
                tooling.get("ok").and_then(Value::as_bool).unwrap_or(false),
                compare.get("ok").and_then(Value::as_bool).unwrap_or(false),
            ]
            .into_iter()
            .all(|v| v);
            json!({
                "ok": ok,
                "strict": strict,
                "steps": {
                    "proof_coverage": proof,
                    "proof_vm": vm,
                    "size_gate": size,
                    "benchmark_thresholds": bench,
                    "tooling_contracts": tooling,
                    "comparison_matrix": compare
                }
            })
        }
        _ => json!({
            "ok": false,
            "error": "unknown_command",
            "command": command,
            "usage": [
                "protheus-ops top1-assurance status",
                "protheus-ops top1-assurance proof-coverage --strict=1",
                "protheus-ops top1-assurance proof-vm --strict=1",
                "protheus-ops top1-assurance size-gate --strict=1",
                "protheus-ops top1-assurance benchmark-thresholds --strict=1",
                "protheus-ops top1-assurance tooling-contracts --strict=1",
                "protheus-ops top1-assurance comparison-matrix --strict=1",
                "protheus-ops top1-assurance run-all --strict=1"
            ]
        }),
    };

    let should_write = command != "status";
    let receipt = wrap_receipt(root, &policy, &command, strict, payload, should_write);

    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );

    if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
    }

    #[test]
    fn microbench_reports_positive_rate() {
        let rate = microbench_tasks_per_sec(120);
        assert!(rate > 0.0);
    }

    #[test]
    fn render_matrix_markdown_has_header_and_rows() {
        let mut projects = Map::<String, Value>::new();
        projects.insert(
            "Protheus".to_string(),
            json!({
                "cold_start_ms": 50.0,
                "idle_memory_mb": 20.0,
                "install_size_mb": 120.0,
                "tasks_per_sec": 9000.0
            }),
        );
        let md = render_matrix_markdown(
            "2026-03-13T00:00:00Z",
            &projects,
            "local/state/ops/top1_assurance/benchmark_latest.json",
            "client/runtime/config/competitive_benchmark_snapshot_2026_02.json",
        );
        assert!(md.contains("# Protheus vs X (CI Generated)"));
        assert!(md.contains("| Protheus |"));
    }

    #[test]
    fn toolchain_check_discovers_home_scoped_binaries() {
        let _guard = env_lock();
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path();
        let empty_path = home.join("path-empty");
        fs::create_dir_all(&empty_path).expect("mkdir path");

        let lean = home.join(".elan/bin/lean");
        let cargo_kani = home.join(".cargo/bin/cargo-kani");
        fs::create_dir_all(lean.parent().expect("lean parent")).expect("mkdir lean");
        fs::create_dir_all(cargo_kani.parent().expect("kani parent")).expect("mkdir kani");
        fs::write(&lean, "#!/bin/sh\necho 'Lean 4.0.0'\n").expect("write lean");
        fs::write(&cargo_kani, "#!/bin/sh\necho 'cargo-kani 0.56.0'\n").expect("write kani");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&lean, &cargo_kani] {
                let mut perms = fs::metadata(path).expect("metadata").permissions();
                perms.set_mode(0o755);
                fs::set_permissions(path, perms).expect("chmod");
            }
        }

        let old_home = std::env::var_os("HOME");
        let old_path = std::env::var_os("PATH");
        std::env::set_var("HOME", home);
        std::env::set_var("PATH", &empty_path);

        let lean_check = run_toolchain_check("lean_toolchain");
        let kani_check = run_toolchain_check("kani_toolchain");

        if let Some(value) = old_home {
            std::env::set_var("HOME", value);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(value) = old_path {
            std::env::set_var("PATH", value);
        } else {
            std::env::remove_var("PATH");
        }

        assert_eq!(lean_check.get("ok").and_then(Value::as_bool), Some(true));
        assert!(lean_check
            .get("resolved_bin")
            .and_then(Value::as_str)
            .map(|v| v.ends_with(".elan/bin/lean"))
            .unwrap_or(false));
        assert_eq!(kani_check.get("ok").and_then(Value::as_bool), Some(true));
        assert!(kani_check
            .get("resolved_bin")
            .and_then(Value::as_str)
            .map(|v| v.ends_with(".cargo/bin/cargo-kani"))
            .unwrap_or(false));
    }
}
