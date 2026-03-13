// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::binary_blob_runtime;
use crate::directive_kernel;
use crate::network_protocol;
use crate::v8_kernel::{
    parse_bool, parse_f64, print_json, read_json, scoped_state_root, sha256_hex_str, write_json,
    write_receipt,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "RSI_IGNITION_STATE_ROOT";
const STATE_SCOPE: &str = "rsi_ignition";

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn loop_state_path(root: &Path) -> PathBuf {
    state_root(root).join("loop_state.json")
}

fn default_loop_state() -> Value {
    json!({
        "version": "1.0",
        "active": false,
        "drift_score": 0.12,
        "exploration_drive": 0.62,
        "merge_count": 0,
        "rollback_count": 0,
        "proactive_evolution_count": 0,
        "last_merge": null,
        "last_rollback": null,
        "swarm": {
            "nodes": 0,
            "share_rate": 0.0,
            "convergence_score": 0.0
        },
        "created_at": now_iso()
    })
}

fn load_loop_state(root: &Path) -> Value {
    read_json(&loop_state_path(root)).unwrap_or_else(default_loop_state)
}

fn store_loop_state(root: &Path, state: &Value) -> Result<(), String> {
    write_json(&loop_state_path(root), state)
}

fn loop_obj_mut(state: &mut Value) -> &mut Map<String, Value> {
    if !state.is_object() {
        *state = default_loop_state();
    }
    state.as_object_mut().expect("loop_state_object")
}

fn mutation_history_path(root: &Path) -> PathBuf {
    crate::core_state_root(root)
        .join("ops")
        .join("binary_blob_runtime")
        .join("mutation_history.jsonl")
}

fn estimate_recent_failure_rate(root: &Path) -> f64 {
    let path = mutation_history_path(root);
    let Ok(raw) = fs::read_to_string(path) else {
        return 0.0;
    };
    let mut total = 0usize;
    let mut denied = 0usize;
    for line in raw.lines().rev().take(64) {
        let Ok(row) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if row.get("allow").is_some() {
            total += 1;
            if !row.get("allow").and_then(Value::as_bool).unwrap_or(false) {
                denied += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        (denied as f64) / (total as f64)
    }
}

fn simulate_regression(proposal: &str, module: &str) -> f64 {
    let h = sha256_hex_str(&format!("{proposal}:{module}"));
    let tail = &h[h.len().saturating_sub(4)..];
    let n = u64::from_str_radix(tail, 16).unwrap_or(0);
    ((n % 100) as f64) / 1000.0
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "rsi_ignition_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 220),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

fn command_status(root: &Path) -> i32 {
    let state = load_loop_state(root);
    emit(
        root,
        json!({
            "ok": true,
            "type": "rsi_ignition_status",
            "lane": "core/layer0/ops",
            "loop_state": state,
            "latest": read_json(&latest_path(root))
        }),
    )
}

fn command_ignite(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let proposal = clean(
        parsed
            .flags
            .get("proposal")
            .cloned()
            .unwrap_or_else(|| "optimize_runtime_loop".to_string()),
        280,
    );
    let module = clean(
        parsed
            .flags
            .get("module")
            .cloned()
            .unwrap_or_else(|| "conduit".to_string()),
        120,
    )
    .to_ascii_lowercase();
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let canary_pass = parse_bool(parsed.flags.get("canary-pass"), true);
    let sim_regression = parsed
        .flags
        .get("sim-regression")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(|| simulate_regression(&proposal, &module))
        .max(0.0);
    let threshold = parse_f64(parsed.flags.get("max-regression"), 0.05).max(0.0);
    let gate_action = format!("rsi:ignite:{module}");
    let gate_ok = directive_kernel::action_allowed(root, &gate_action);
    let mut allowed = gate_ok && canary_pass && sim_regression <= threshold;
    let mut mutation_exit = 0i32;

    let mut state = load_loop_state(root);
    let state_obj = loop_obj_mut(&mut state);
    state_obj.insert("active".to_string(), Value::Bool(apply && allowed));

    if apply && allowed {
        mutation_exit = binary_blob_runtime::run(
            root,
            &[
                "mutate".to_string(),
                format!("--module={module}"),
                format!("--proposal={proposal}"),
                "--apply=1".to_string(),
                format!("--canary-pass={}", if canary_pass { 1 } else { 0 }),
                format!("--sim-regression={sim_regression:.4}"),
            ],
        );
        if mutation_exit != 0 {
            allowed = false;
        }
    }

    if apply && allowed {
        let next = state_obj
            .get("merge_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        state_obj.insert("merge_count".to_string(), Value::from(next));
        state_obj.insert(
            "last_merge".to_string(),
            json!({
                "ts": now_iso(),
                "proposal": proposal,
                "module": module,
                "sim_regression": sim_regression
            }),
        );
        let _ = network_protocol::run(
            root,
            &[
                "reward".to_string(),
                "--agent=organism:global".to_string(),
                "--amount=1".to_string(),
                "--reason=tokenomics".to_string(),
            ],
        );
    } else if apply {
        let next = state_obj
            .get("rollback_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        state_obj.insert("rollback_count".to_string(), Value::from(next));
        state_obj.insert(
            "last_rollback".to_string(),
            json!({
                "ts": now_iso(),
                "proposal": proposal,
                "module": module,
                "gate_ok": gate_ok,
                "canary_pass": canary_pass,
                "sim_regression": sim_regression,
                "mutation_exit": mutation_exit
            }),
        );
    }

    if let Err(err) = store_loop_state(root, &state) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_activate",
                "lane": "core/layer0/ops",
                "error": clean(err, 220)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": allowed,
            "type": "rsi_ignition_activate",
            "lane": "core/layer0/ops",
            "proposal": proposal,
            "module": module,
            "apply": apply,
            "gate_ok": gate_ok,
            "canary_pass": canary_pass,
            "sim_regression": sim_regression,
            "max_regression": threshold,
            "mutation_exit": mutation_exit,
            "pipeline": ["propose", "simulate", "canary", "merge_or_rollback"]
        }),
    )
}

fn command_reflect(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let mut state = load_loop_state(root);
    let observed_failure_rate = estimate_recent_failure_rate(root);
    let drift = parsed
        .flags
        .get("drift")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or((0.1 + observed_failure_rate * 0.8).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);
    let exploration = parsed
        .flags
        .get("exploration")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(|| {
            let prior = state
                .get("exploration_drive")
                .and_then(Value::as_f64)
                .unwrap_or(0.6);
            if drift > 0.5 {
                (prior - 0.15).clamp(0.05, 1.0)
            } else {
                (prior + 0.05).clamp(0.05, 1.0)
            }
        })
        .clamp(0.0, 1.0);

    {
        let obj = loop_obj_mut(&mut state);
        obj.insert("drift_score".to_string(), Value::from(drift));
        obj.insert("exploration_drive".to_string(), Value::from(exploration));
        obj.insert(
            "last_reflection".to_string(),
            json!({
                "ts": now_iso(),
                "drift_score": drift,
                "exploration_drive": exploration,
                "observed_failure_rate": observed_failure_rate,
                "action": if drift > 0.5 { "self_correct" } else { "continue_explore" }
            }),
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_reflection",
                "lane": "core/layer0/ops",
                "error": clean(err, 220)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "rsi_ignition_reflection",
            "lane": "core/layer0/ops",
            "drift_score": drift,
            "exploration_drive": exploration,
            "observed_failure_rate": observed_failure_rate,
            "action": if drift > 0.5 { "self_correct" } else { "continue_explore" }
        }),
    )
}

fn command_swarm(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let nodes = parse_f64(parsed.flags.get("nodes"), 8.0).max(1.0) as u64;
    let share_rate = parse_f64(parsed.flags.get("share-rate"), 0.55).clamp(0.0, 1.0);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let gate_ok = directive_kernel::action_allowed(root, "rsi:swarm");
    let convergence = ((share_rate * 0.75) + ((nodes as f64).ln() / 10.0)).clamp(0.0, 1.0);
    let allowed = gate_ok && convergence > 0.1;

    let mut state = load_loop_state(root);
    if apply && allowed {
        let obj = loop_obj_mut(&mut state);
        obj.insert(
            "swarm".to_string(),
            json!({
                "nodes": nodes,
                "share_rate": share_rate,
                "convergence_score": convergence,
                "updated_at": now_iso()
            }),
        );
        let _ = network_protocol::run(
            root,
            &[
                "reward".to_string(),
                "--agent=organism:swarm".to_string(),
                format!("--amount={:.4}", (nodes as f64) * share_rate * 0.1),
                "--reason=tokenomics".to_string(),
            ],
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_swarm",
                "lane": "core/layer0/ops",
                "error": clean(err, 220)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": allowed,
            "type": "rsi_ignition_swarm",
            "lane": "core/layer0/ops",
            "nodes": nodes,
            "share_rate": share_rate,
            "convergence_score": convergence,
            "apply": apply,
            "gate_ok": gate_ok
        }),
    )
}

fn command_evolve(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let mut state = load_loop_state(root);
    let insight = clean(
        parsed
            .flags
            .get("insight")
            .cloned()
            .unwrap_or_else(|| "I found a lower-cost planning strategy with stable quality.".to_string()),
        360,
    );
    let module = clean(
        parsed
            .flags
            .get("module")
            .cloned()
            .unwrap_or_else(|| "conduit".to_string()),
        120,
    )
    .to_ascii_lowercase();
    let apply = parse_bool(parsed.flags.get("apply"), false);
    let ignite_apply = parse_bool(parsed.flags.get("ignite-apply"), false);
    let gate_ok = directive_kernel::action_allowed(root, &format!("rsi:evolve:{module}"));

    let mut ignite_exit = 0i32;
    if apply && gate_ok {
        ignite_exit = command_ignite(
            root,
            &parse_args(&[
                "ignite".to_string(),
                format!("--proposal={insight}"),
                format!("--module={module}"),
                format!("--apply={}", if ignite_apply { 1 } else { 0 }),
            ]),
        );
    }

    {
        let obj = loop_obj_mut(&mut state);
        let next = obj
            .get("proactive_evolution_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        obj.insert("proactive_evolution_count".to_string(), Value::from(next));
        obj.insert(
            "last_evolution".to_string(),
            json!({
                "ts": now_iso(),
                "insight": insight,
                "module": module,
                "apply": apply,
                "ignite_apply": ignite_apply,
                "ignite_exit": ignite_exit
            }),
        );
    }
    if let Err(err) = store_loop_state(root, &state) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_evolve",
                "lane": "core/layer0/ops",
                "error": clean(err, 220)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": gate_ok,
            "type": "rsi_ignition_evolve",
            "lane": "core/layer0/ops",
            "insight": insight,
            "module": module,
            "apply": apply,
            "ignite_apply": ignite_apply,
            "ignite_exit": ignite_exit,
            "directive_safe": gate_ok
        }),
    )
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops rsi-ignition status");
        println!("  protheus-ops rsi-ignition ignite [--proposal=<text>] [--module=<id>] [--apply=1|0] [--canary-pass=1|0] [--sim-regression=<0..1>]");
        println!("  protheus-ops rsi-ignition reflect [--drift=<0..1>] [--exploration=<0..1>]");
        println!("  protheus-ops rsi-ignition swarm [--nodes=<n>] [--share-rate=<0..1>] [--apply=1|0]");
        println!("  protheus-ops rsi-ignition evolve [--insight=<text>] [--module=<id>] [--apply=1|0] [--ignite-apply=1|0]");
        return 0;
    }

    match command.as_str() {
        "status" => command_status(root),
        "ignite" => command_ignite(root, &parsed),
        "reflect" => command_reflect(root, &parsed),
        "swarm" => command_swarm(root, &parsed),
        "evolve" => command_evolve(root, &parsed),
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("protheus_rsi_ignition_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn allow(root: &Path, directive: &str) {
        std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "test-sign-key");
        assert_eq!(
            crate::directive_kernel::run(
                root,
                &[
                    "prime-sign".to_string(),
                    format!("--directive={directive}"),
                    "--signer=tester".to_string(),
                ]
            ),
            0
        );
    }

    #[test]
    fn ignite_requires_directive_gate() {
        let root = temp_root("gate");
        let exit = run(
            &root,
            &[
                "ignite".to_string(),
                "--proposal=unsafe".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ignite_mutates_when_allowed() {
        let root = temp_root("allowed");
        allow(&root, "allow:rsi:ignite");
        allow(&root, "allow:blob_mutate");
        let exit = run(
            &root,
            &[
                "ignite".to_string(),
                "--proposal=safe".to_string(),
                "--module=conduit".to_string(),
                "--apply=1".to_string(),
                "--canary-pass=1".to_string(),
                "--sim-regression=0.001".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let state = read_json(&loop_state_path(&root)).expect("state");
        assert!(
            state
                .get("merge_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn evolve_apply_writes_proactive_state() {
        let root = temp_root("evolve");
        allow(&root, "allow:rsi:evolve");
        let exit = run(
            &root,
            &[
                "evolve".to_string(),
                "--insight=more stable route".to_string(),
                "--module=conduit".to_string(),
                "--apply=1".to_string(),
                "--ignite-apply=0".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let state = read_json(&loop_state_path(&root)).expect("state");
        assert!(
            state
                .get("proactive_evolution_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }
}

