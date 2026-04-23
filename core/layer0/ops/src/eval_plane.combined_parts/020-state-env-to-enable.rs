
const STATE_ENV: &str = "EVAL_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "eval_plane";
const CONTRACT_PATH: &str = "planes/contracts/eval/eval_loop_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  infring-ops eval-plane status");
    println!("  infring-ops eval-plane enable-neuralavb [--enabled=1|0] [--strict=1|0]");
    println!("  infring-ops eval-plane experiment-loop [--iterations=<n>] [--baseline-cost-usd=<n>] [--run-cost-usd=<n>] [--baseline-accuracy=<0..1>] [--run-accuracy=<0..1>] [--fixture-json=<json>] [--strict=1|0]");
    println!("  infring-ops eval-plane benchmark [--strict=1|0]");
    println!("  infring-ops eval-plane dashboard [--strict=1|0]");
    println!("  infring-ops eval-plane run [--iterations=<n>] [--baseline-cost-usd=<n>] [--run-cost-usd=<n>] [--baseline-accuracy=<0..1>] [--run-accuracy=<0..1>] [--strict=1|0]");
    println!("  infring-ops eval-plane rl-upgrade [--profile=infring-v2] [--iterations=<n>] [--runtime-classes=terminal,gui,swe,tool-call] [--persona=<id>] [--strict=1|0]");
    println!("  infring-ops eval-plane rl-status [--strict=1|0]");
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn state_file(root: &Path, file_name: &str) -> PathBuf {
    state_root(root).join(file_name)
}

fn state_subdir_file(root: &Path, dir: &str, file_name: &str) -> PathBuf {
    state_root(root).join(dir).join(file_name)
}

fn latest_path(root: &Path) -> PathBuf {
    state_file(root, "latest.json")
}

fn config_path(root: &Path) -> PathBuf {
    state_file(root, "config.json")
}

fn fixture_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "fixtures", "ground_truth_latest.json")
}

fn loop_latest_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "loops", "latest.json")
}

fn trace_history_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "loops", "trace_history.jsonl")
}

fn benchmark_latest_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "benchmarks", "latest.json")
}

fn rl_latest_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "rl", "infring_v2_latest.json")
}

fn rl_history_path(root: &Path) -> PathBuf {
    state_subdir_file(root, "rl", "infring_v2_history.jsonl")
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "eval_plane_error", payload)
}

fn parse_json_flag(raw: Option<&String>) -> Option<Value> {
    raw.and_then(|text| serde_json::from_str::<Value>(text).ok())
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "enable-neuralavb" => vec!["V6-EVAL-001.1", "V6-EVAL-001.4"],
        "experiment-loop" => vec![
            "V6-EVAL-001.1",
            "V6-EVAL-001.2",
            "V6-EVAL-001.3",
            "V6-EVAL-001.4",
        ],
        "benchmark" => vec!["V6-EVAL-001.3", "V6-EVAL-001.4"],
        "dashboard" => vec!["V6-EVAL-001.3", "V6-EVAL-001.4", "V6-EVAL-001.5"],
        "run" => vec![
            "V6-EVAL-001.1",
            "V6-EVAL-001.2",
            "V6-EVAL-001.3",
            "V6-EVAL-001.4",
        ],
        "rl-upgrade" | "rl-status" => vec![
            "V6-COCKPIT-017.11",
            "V6-COCKPIT-017.12",
            "V6-COCKPIT-017.13",
            "V6-COCKPIT-017.14",
            "V6-COCKPIT-017.15",
        ],
        _ => vec!["V6-EVAL-001.4"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "eval_plane_conduit_enforcement",
        "core/layer0/ops/eval_plane",
        bypass_requested,
        "eval_runtime_routes_through_layer0_conduit_with_fail_closed_denials",
        &claim_ids,
    )
}

fn status(root: &Path) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "eval_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "config": read_json(&config_path(root)),
        "latest_loop": read_json(&loop_latest_path(root)),
        "latest_benchmark": read_json(&benchmark_latest_path(root)),
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.4",
                "claim": "eval_surface_is_core_authoritative_and_receipted",
                "evidence": {
                    "has_loop": read_json(&loop_latest_path(root)).is_some()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn upsert_fixture(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let default_fixture = json!({
        "version": "v1",
        "dataset": "neuralavb_ground_truth",
        "cases": [
            {"id":"latency_guard","expected":"pass"},
            {"id":"accuracy_guard","expected":"pass"},
            {"id":"cost_guard","expected":"pass"}
        ]
    });
    let fixture = parse_json_flag(parsed.flags.get("fixture-json")).unwrap_or(default_fixture);
    let _ = write_json(&fixture_path(root), &fixture);
    fixture
}

fn run_enable(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let enabled = parse_bool(parsed.flags.get("enabled"), true);
    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version":"v1",
            "kind":"eval_loop_contract",
            "max_iterations": 32
        }),
    );
    let config = json!({
        "version":"v1",
        "enabled_neuralavb": enabled,
        "updated_at": crate::now_iso(),
        "contract_digest": sha256_hex_str(&contract.to_string())
    });
    let _ = write_json(&config_path(root), &config);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "eval_plane_enable_neuralavb",
        "lane": "core/layer0/ops",
        "action": "enable-neuralavb",
        "enabled": enabled,
        "config_path": config_path(root).display().to_string(),
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.1",
                "claim": "eval_engine_enables_build_experiment_evaluate_loop_profile",
                "evidence": {"enabled": enabled}
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
