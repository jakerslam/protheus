// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::v8_kernel::{deterministic_merkle_root, write_receipt};
use crate::{deterministic_receipt_hash, now_iso};
use base64::Engine as _;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "autonomy_controller";
const REPLACEMENT: &str = "protheus-ops autonomy-controller";
const STATE_DIR: &str = "local/state/ops/autonomy_controller";
const STATE_ENV: &str = "AUTONOMY_CONTROLLER_STATE_ROOT";
const STATE_SCOPE: &str = "autonomy_controller";

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops autonomy-controller status");
    println!("  protheus-ops autonomy-controller run [--max-actions=<n>] [--objective=<id>]");
    println!("  protheus-ops autonomy-controller hand-new [--hand-id=<id>] [--template=<id>] [--schedule=<cron>] [--provider=<id>] [--fallback=<id>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller hand-cycle --hand-id=<id> [--goal=<text>] [--provider=<id>] [--fallback=<id>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller hand-status [--hand-id=<id>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller hand-memory-page --hand-id=<id> [--op=page-in|page-out|status] [--tier=core|archival|external] [--key=<id>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller hand-wasm-task --hand-id=<id> [--task=<id>] [--fuel=<n>] [--epoch-ms=<n>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller compact [<snip|micro|full|reactive>] [--hand-id=<id>] [--auto-compact-pct=<0..100>] [--pressure-ratio=<0..1>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller dream [--hand-id=<id>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller proactive_daemon [status|cycle|pause|resume] [--auto=1|0] [--force=1|0] [--tick-ms=<n>] [--jitter-ms=<n>] [--window-sec=<n>] [--max-proactive=<n>] [--block-budget-ms=<n>] [--brief=1|0] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller speculate [run|status|merge|reject] [--spec-id=<id>] [--verify=1|0] [--input-json=<json>|--input-base64=<base64_json>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller autoreason [run|status] [--task=<text>] [--run-id=<id>] [--convergence=<n>] [--max-iters=<n>] [--judges=<n>] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller ephemeral-run [--goal=<text>] [--domain=<id>] [--ui-leaf=1|0] [--strict=1|0]");
    println!("  protheus-ops autonomy-controller trunk-status [--strict=1|0]");
    println!(
        "  protheus-ops autonomy-controller pain-signal [--action=<status|emit|focus-start|focus-stop|focus-status>] [--source=<id>] [--code=<id>] [--severity=<low|medium|high|critical>] [--risk=<low|medium|high>]"
    );
    println!(
        "  protheus-ops autonomy-controller multi-agent-debate <run|status> [--input-base64=<base64_json>|--input-json=<json>] [--policy=<path>] [--date=<YYYY-MM-DD>] [--persist=1|0]"
    );
    println!(
        "  protheus-ops autonomy-controller ethical-reasoning <run|status> [--input-base64=<base64_json>|--policy=<path>] [--state-dir=<path>] [--persist=1|0]"
    );
    println!(
        "  protheus-ops autonomy-controller autonomy-simulation-harness <run|status> [YYYY-MM-DD] [--days=N] [--write=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops autonomy-controller runtime-stability-soak [--action=<start|check-now|status|report>] [flags]"
    );
    println!(
        "  protheus-ops autonomy-controller self-documentation-closeout [--action=<run|status>] [flags]"
    );
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, false)
}

fn parse_positional(argv: &[String], idx: usize) -> Option<String> {
    argv.iter()
        .filter(|arg| !arg.trim().starts_with("--"))
        .nth(idx)
        .map(|v| v.trim().to_string())
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn parse_i64(raw: Option<&str>, fallback: i64, lo: i64, hi: i64) -> i64 {
    lane_utils::parse_i64_clamped(raw, fallback, lo, hi)
}

fn parse_u64(raw: Option<&str>, fallback: u64, lo: u64, hi: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn parse_payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = parse_flag(argv, "input-json") {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("input_json_parse_failed:{e}"));
    }
    if let Some(raw) = parse_flag(argv, "input-base64") {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|e| format!("input_base64_decode_failed:{e}"))?;
        let text =
            String::from_utf8(decoded).map_err(|e| format!("input_base64_utf8_failed:{e}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|e| format!("input_base64_json_parse_failed:{e}"));
    }
    Ok(json!({}))
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let max_actions = parse_flag(argv, "max-actions")
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 100))
        .unwrap_or(1);
    let objective = parse_flag(argv, "objective").unwrap_or_else(|| "default".to_string());

    let mut out = protheus_autonomy_core_v1::autonomy_receipt(cmd, Some(&objective));
    out["lane"] = Value::String(LANE_ID.to_string());
    out["ts"] = Value::String(now_iso());
    out["argv"] = json!(argv);
    out["max_actions"] = json!(max_actions);
    out["replacement"] = Value::String(REPLACEMENT.to_string());
    out["root"] = Value::String(root.to_string_lossy().to_string());
    out["claim_evidence"] = json!([
        {
            "id": "native_autonomy_controller_lane",
            "claim": "autonomy_controller_executes_natively_in_rust",
            "evidence": {
                "command": cmd,
                "max_actions": max_actions
            }
        }
    ]);
    if let Some(map) = out.as_object_mut() {
        map.remove("receipt_hash");
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn native_pain_signal_receipt(root: &Path, argv: &[String]) -> Value {
    let action = parse_flag(argv, "action")
        .or_else(|| parse_positional(argv, 1))
        .unwrap_or_else(|| "status".to_string());
    let source = parse_flag(argv, "source");
    let code = parse_flag(argv, "code");
    let severity = parse_flag(argv, "severity");
    let risk = parse_flag(argv, "risk");

    let mut out = protheus_autonomy_core_v1::pain_signal_receipt(
        action.as_str(),
        source.as_deref(),
        code.as_deref(),
        severity.as_deref(),
        risk.as_deref(),
    );
    out["lane"] = Value::String(LANE_ID.to_string());
    out["ts"] = Value::String(now_iso());
    out["argv"] = json!(argv);
    out["replacement"] = Value::String(REPLACEMENT.to_string());
    out["root"] = Value::String(root.to_string_lossy().to_string());
    out["claim_evidence"] = json!([
        {
            "id": "native_autonomy_pain_signal_lane",
            "claim": "pain_signal_contract_executes_natively_in_rust",
            "evidence": {
                "action": action,
                "source": source,
                "code": code
            }
        }
    ]);
    if let Some(map) = out.as_object_mut() {
        map.remove("receipt_hash");
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "autonomy_controller_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn state_root(root: &Path) -> PathBuf {
    root.join(STATE_DIR)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn clean_id(raw: Option<String>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn hand_path(root: &Path, hand_id: &str) -> PathBuf {
    state_root(root)
        .join("hands")
        .join(format!("{hand_id}.json"))
}

fn hand_events_path(root: &Path, hand_id: &str) -> PathBuf {
    state_root(root)
        .join("hands")
        .join(format!("{hand_id}.events.jsonl"))
}

fn trunk_state_path(root: &Path) -> PathBuf {
    state_root(root).join("trunk").join("state.json")
}

fn trunk_events_path(root: &Path) -> PathBuf {
    state_root(root).join("trunk").join("events.jsonl")
}

fn autonomy_runs_dir(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("local")
        .join("state")
        .join("autonomy")
        .join("runs")
}

fn autonomy_runs_path(root: &Path, day: &str) -> PathBuf {
    autonomy_runs_dir(root).join(format!("{day}.jsonl"))
}

fn today_ymd(ts: &str) -> String {
    let day = ts.split('T').next().unwrap_or("").trim();
    if day.len() == 10 && day.chars().nth(4) == Some('-') && day.chars().nth(7) == Some('-') {
        day.to_string()
    } else {
        now_iso().chars().take(10).collect()
    }
}

fn persist_autonomy_run_row(
    root: &Path,
    argv: &[String],
    receipt: &Value,
) -> Result<Value, String> {
    let ts = receipt
        .get("ts")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(now_iso);
    let day = today_ymd(&ts);
    let max_actions = parse_flag(argv, "max-actions")
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 100))
        .unwrap_or(1);
    let objective_id = parse_flag(argv, "objective").unwrap_or_else(|| "default".to_string());
    let result = parse_flag(argv, "result").unwrap_or_else(|| "executed".to_string());
    let outcome = parse_flag(argv, "outcome").unwrap_or_else(|| {
        if result.eq_ignore_ascii_case("executed") {
            "no_change".to_string()
        } else {
            "blocked".to_string()
        }
    });
    let policy_hold_reason = parse_flag(argv, "policy-hold-reason");
    let route_block_reason = parse_flag(argv, "route-block-reason");
    let policy_hold = parse_bool(parse_flag(argv, "policy-hold").as_deref(), false)
        || result
            .to_ascii_lowercase()
            .starts_with("no_candidates_policy_");
    let duality = receipt
        .get("duality")
        .and_then(Value::as_object)
        .map(|bundle| {
            json!({
                "toll": bundle.get("toll").cloned().unwrap_or(Value::Null),
                "dual_voice": bundle.get("dual_voice").cloned().unwrap_or(Value::Null),
                "fractal_balance_score": bundle
                    .get("fractal_balance_score")
                    .cloned()
                    .unwrap_or(Value::Null)
            })
        })
        .unwrap_or_else(|| {
            json!({
                "toll": Value::Null,
                "dual_voice": Value::Null,
                "fractal_balance_score": Value::Null
            })
        });
    let row = json!({
        "ts": ts,
        "type": "autonomy_run",
        "lane": LANE_ID,
        "command": "run",
        "objective_id": objective_id,
        "max_actions": max_actions,
        "result": result,
        "outcome": outcome,
        "policy_hold": policy_hold,
        "policy_hold_reason": policy_hold_reason,
        "route_block_reason": route_block_reason,
        "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "duality": duality
    });
    append_jsonl(&autonomy_runs_path(root, &day), &row)?;
    Ok(row)
}

fn load_domain_constraints(root: &Path) -> Value {
    read_json(
        &root
            .join("client")
            .join("runtime")
            .join("config")
            .join("agent_domain_constraints.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "allowed_domains": ["general", "finance", "healthcare", "enterprise", "research"],
            "deny_without_policy": true
        })
    })
}

fn load_provider_policy(root: &Path) -> Value {
    read_json(
        &root
            .join("client")
            .join("runtime")
            .join("config")
            .join("hand_provider_policy.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "allowed_providers": ["bitnet", "openai", "frontier_provider", "local-moe"],
            "default_provider": "bitnet",
            "max_cost_per_cycle_usd": 0.50
        })
    })
}

fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn autonomy_duality_clearance_tier(toll: &Value, harmony: f64) -> i64 {
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if hard_block {
        return 1;
    }
    let debt_after = as_f64(toll.get("debt_after"), 0.0).clamp(0.0, 100.0);
    if debt_after >= 0.75 {
        2
    } else if debt_after <= 0.2 && harmony >= 0.85 {
        4
    } else {
        3
    }
}

fn autonomy_duality_bundle(
    root: &Path,
    lane: &str,
    source: &str,
    run_id: &str,
    context: &Value,
    persist: bool,
) -> Value {
    let mut base_context = serde_json::Map::new();
    base_context.insert("lane".to_string(), Value::String(lane.to_string()));
    base_context.insert("source".to_string(), Value::String(source.to_string()));
    base_context.insert("run_id".to_string(), Value::String(run_id.to_string()));
    if let Some(obj) = context.as_object() {
        for (k, v) in obj {
            base_context.insert(k.clone(), v.clone());
        }
    }

    let evaluation = match crate::duality_seed::invoke(
        root,
        "duality_evaluate",
        Some(&json!({
            "context": Value::Object(base_context.clone()),
            "opts": {
                "persist": persist,
                "lane": lane,
                "source": source,
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "autonomy_duality_bundle",
                "error": format!("duality_evaluate_failed:{err}")
            });
        }
    };

    let dual_voice = crate::duality_seed::invoke(
        root,
        "dual_voice_evaluate",
        Some(&json!({
            "context": Value::Object(base_context.clone()),
            "left": {
                "policy_lens": "guardian",
                "focus": "structured_reasoning"
            },
            "right": {
                "policy_lens": "strategist",
                "focus": "creative_inversion"
            },
            "opts": {
                "persist": persist,
                "source": source,
                "run_id": run_id
            }
        })),
    )
    .unwrap_or_else(|_| json!({"ok": false, "type": "duality_dual_voice_evaluation"}));

    let toll_update = match crate::duality_seed::invoke(
        root,
        "duality_toll_update",
        Some(&json!({
            "context": Value::Object(base_context),
            "signal": evaluation.clone(),
            "opts": {
                "persist": persist,
                "source": source,
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "autonomy_duality_bundle",
                "evaluation": evaluation,
                "dual_voice": dual_voice,
                "error": format!("duality_toll_update_failed:{err}")
            });
        }
    };

    let toll = toll_update
        .get("toll")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let harmony = as_f64(
        dual_voice.get("harmony"),
        as_f64(evaluation.get("zero_point_harmony_potential"), 0.0),
    )
    .clamp(0.0, 1.0);
    let debt_after = as_f64(toll.get("debt_after"), 0.0).clamp(0.0, 100.0);
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_clearance_tier = autonomy_duality_clearance_tier(&toll, harmony);

    json!({
        "ok": true,
        "type": "autonomy_duality_bundle",
        "lane": lane,
        "source": source,
        "run_id": run_id,
        "evaluation": evaluation,
        "dual_voice": dual_voice,
        "toll": toll,
        "state": toll_update.get("state").cloned().unwrap_or(Value::Null),
        "hard_block": hard_block,
        "recommended_clearance_tier": recommended_clearance_tier,
        "fractal_balance_score": ((harmony * (1.0 - debt_after.min(1.0))) * 1_000_000.0).round() / 1_000_000.0
    })
}

fn autonomy_duality_hard_block(duality: &Value) -> bool {
    duality
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn conduit_guard(argv: &[String], strict: bool) -> Option<Value> {
    if strict && parse_bool(parse_flag(argv, "bypass").as_deref(), false) {
        Some(json!({
            "ok": false,
            "type": "autonomy_controller_conduit_gate",
            "lane": LANE_ID,
            "strict": strict,
            "error": "conduit_bypass_rejected",
            "claim_evidence": [
                {
                    "id": "V8-AGENT-ERA-001.5",
                    "claim": "all_ephemeral_and_hand_operations_route_through_conduit_with_fail_closed_boundary",
                    "evidence": {"bypass_requested": true}
                }
            ]
        }))
    } else {
        None
    }
}

fn emit_receipt(root: &Path, value: &mut Value) -> i32 {
    if let Some(map) = value.as_object_mut() {
        map.remove("receipt_hash");
    }
    value["receipt_hash"] = Value::String(receipt_hash(value));
    match write_receipt(root, STATE_ENV, STATE_SCOPE, value.clone()) {
        Ok(out) => {
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "autonomy_controller_error",
                "lane": LANE_ID,
                "error": err
            });
            out["receipt_hash"] = Value::String(receipt_hash(&out));
            print_json_line(&out);
            1
        }
    }
}
