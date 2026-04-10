// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

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

fn flag_value(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let mut i = 0usize;
    while i < argv.len() {
        let tok = argv[i].trim();
        if let Some(v) = tok.strip_prefix(&pref) {
            return Some(v.to_string());
        }
        if tok == format!("--{key}") {
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    return Some(next.clone());
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_bool(value: Option<String>, fallback: bool) -> bool {
    let Some(raw) = value else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_u32(value: Option<String>, fallback: u32, min: u32, max: u32) -> u32 {
    value
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v.clamp(min, max))
        .unwrap_or(fallback)
}

fn parse_bool_flag(argv: &[String], key: &str, fallback: bool) -> bool {
    parse_bool(flag_value(argv, key), fallback)
}

fn parse_u32_flag(argv: &[String], key: &str, fallback: u32, min: u32, max: u32) -> u32 {
    parse_u32(flag_value(argv, key), fallback, min, max)
}

fn parse_csv_list(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

fn parse_csv_set(value: Option<String>, fallback: &str) -> HashSet<String> {
    value
        .or_else(|| Some(fallback.to_string()))
        .map(|row| {
            parse_csv_list(Some(row))
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn run(root: &Path, args: &[String]) -> i32 {
    let cmd = args
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops strategy-mode-governor status");
        println!("  protheus-ops strategy-mode-governor evaluate [--mode=score_only|canary_execute|execute] [--ready=1|0] [--failed-checks=a,b] [--canary-preview-ready=1|0] [--canary-ready=1|0] [--quality-lock=1|0]");
        return 0;
    }

    if !matches!(cmd.as_str(), "status" | "evaluate" | "run") {
        let mut out = json!({
            "ok": false,
            "type": "strategy_mode_governor_cli_error",
            "ts": now_iso(),
            "command": cmd,
            "argv": args,
            "error": "unknown_command",
            "exit_code": 2
        });
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        print_json_line(&out);
        return 2;
    }

    let current_mode = flag_value(args, "mode").unwrap_or_else(|| "score_only".to_string());
    let strict_ready = parse_bool_flag(args, "ready", false);
    let failed_checks = parse_csv_list(flag_value(args, "failed-checks"));
    let relax_checks = parse_csv_set(
        flag_value(args, "canary-relax-checks"),
        "success_criteria_pass_rate",
    );
    let readiness = readiness_state(
        &current_mode,
        strict_ready,
        &failed_checks,
        true,
        &relax_checks,
    );

    let canary = CanaryState {
        preview_ready_for_canary: parse_bool_flag(
            args,
            "canary-preview-ready",
            readiness.ready_for_canary,
        ),
        ready_for_execute: parse_bool_flag(args, "canary-ready", readiness.ready_for_execute),
        quality_lock_active: parse_bool_flag(args, "quality-lock", true),
    };

    let policy = GovernorPolicy {
        promote_canary: parse_bool_flag(args, "promote-canary", true),
        promote_execute: parse_bool_flag(args, "promote-execute", true),
        demote_not_ready: parse_bool_flag(args, "demote-not-ready", true),
        min_escalate_streak: parse_u32_flag(args, "min-escalate-streak", 1, 1, 100),
        min_demote_streak: parse_u32_flag(args, "min-demote-streak", 1, 1, 100),
        canary_require_quality_lock_for_execute: parse_bool_flag(
            args,
            "require-quality-lock",
            true,
        ),
        require_spc: parse_bool_flag(args, "require-spc", true),
    };

    let streak = StreakState {
        escalate_ready_streak: parse_u32_flag(args, "escalate-streak", 1, 0, 1000),
        demote_not_ready_streak: parse_u32_flag(args, "demote-streak", 0, 0, 1000),
    };

    let spc_pass = parse_bool_flag(args, "spc-pass", true);
    let spc_hold_escalation = parse_bool_flag(args, "spc-hold", false);
    let transition = decide_transition(
        &current_mode,
        &readiness,
        &canary,
        &policy,
        spc_pass,
        spc_hold_escalation,
        &streak,
    );

    let mut out = json!({
        "ok": true,
        "type": "strategy_mode_governor",
        "ts": now_iso(),
        "command": cmd,
        "argv": args,
        "root": root.to_string_lossy(),
        "current_mode": current_mode,
        "readiness": readiness,
        "canary": canary,
        "policy": policy,
        "streak": streak,
        "transition": transition,
        "claim_evidence": [
            {
                "id": "native_strategy_mode_governor_lane",
                "claim": "strategy mode transition logic executes natively in rust",
                "evidence": {
                    "transition_present": transition.is_some()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    print_json_line(&out);
    0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessState {
    pub strict_ready: bool,
    pub canary_relaxed: bool,
    pub ready_for_canary: bool,
    pub ready_for_execute: bool,
    pub effective_ready: bool,
    pub failed_checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanaryState {
    pub preview_ready_for_canary: bool,
    pub ready_for_execute: bool,
    pub quality_lock_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GovernorPolicy {
    pub promote_canary: bool,
    pub promote_execute: bool,
    pub demote_not_ready: bool,
    pub min_escalate_streak: u32,
    pub min_demote_streak: u32,
    pub canary_require_quality_lock_for_execute: bool,
    pub require_spc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreakState {
    pub escalate_ready_streak: u32,
    pub demote_not_ready_streak: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Transition {
    pub to_mode: String,
    pub reason: String,
    pub cooldown_exempt: bool,
}

pub fn canary_failed_checks_allowed(
    failed_checks: &[String],
    allowed_checks: &HashSet<String>,
) -> bool {
    if failed_checks.is_empty() || allowed_checks.is_empty() {
        return false;
    }
    failed_checks.iter().all(|check| {
        let normalized = check.trim();
        !normalized.is_empty() && allowed_checks.contains(normalized)
    })
}

pub fn readiness_state(
    mode: &str,
    ready_for_execute: bool,
    failed_checks: &[String],
    canary_relax_enabled: bool,
    canary_relax_checks: &HashSet<String>,
) -> ReadinessState {
    let failed = failed_checks
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let canary_relaxed =
        canary_relax_enabled && canary_failed_checks_allowed(&failed, canary_relax_checks);
    let ready_for_canary = ready_for_execute || canary_relaxed;
    let effective_ready = if mode.trim() == "execute" {
        ready_for_execute
    } else {
        ready_for_canary
    };

    ReadinessState {
        strict_ready: ready_for_execute,
        canary_relaxed,
        ready_for_canary,
        ready_for_execute,
        effective_ready,
        failed_checks: failed,
    }
}

fn spc_allows_escalation(
    spc_pass: bool,
    spc_hold_escalation: bool,
    policy: &GovernorPolicy,
) -> bool {
    if !policy.require_spc {
        return true;
    }
    spc_pass && !spc_hold_escalation
}

pub fn decide_transition(
    current_mode: &str,
    readiness: &ReadinessState,
    canary: &CanaryState,
    policy: &GovernorPolicy,
    spc_pass: bool,
    spc_hold_escalation: bool,
    streak: &StreakState,
) -> Option<Transition> {
    let mode = current_mode.trim();
    let escalate_ready = streak.escalate_ready_streak >= policy.min_escalate_streak.max(1);
    let demote_ready = streak.demote_not_ready_streak >= policy.min_demote_streak.max(1);

    if mode == "score_only" {
        if !policy.promote_canary {
            return None;
        }
        if readiness.ready_for_canary
            && canary.preview_ready_for_canary
            && spc_allows_escalation(spc_pass, spc_hold_escalation, policy)
            && escalate_ready
        {
            return Some(Transition {
                to_mode: "canary_execute".to_string(),
                reason: "readiness_pass_promote_canary".to_string(),
                cooldown_exempt: false,
            });
        }
        return None;
    }

    if mode == "canary_execute" {
        if policy.demote_not_ready && !readiness.ready_for_canary && demote_ready {
            return Some(Transition {
                to_mode: "score_only".to_string(),
                reason: "readiness_fail_demote_score_only".to_string(),
                cooldown_exempt: true,
            });
        }

        if policy.promote_execute
            && readiness.ready_for_execute
            && canary.ready_for_execute
            && spc_allows_escalation(spc_pass, spc_hold_escalation, policy)
            && escalate_ready
        {
            return Some(Transition {
                to_mode: "execute".to_string(),
                reason: "canary_metrics_pass_promote_execute".to_string(),
                cooldown_exempt: false,
            });
        }

        return None;
    }

    if mode == "execute" {
        let quality_lock_required = policy.canary_require_quality_lock_for_execute;
        let needs_demotion =
            !readiness.ready_for_execute || (quality_lock_required && !canary.quality_lock_active);

        if policy.demote_not_ready && needs_demotion && demote_ready {
            return Some(Transition {
                to_mode: "canary_execute".to_string(),
                reason: if !readiness.ready_for_execute {
                    "readiness_fail_demote_canary".to_string()
                } else {
                    "quality_lock_inactive_demote_canary".to_string()
                },
                cooldown_exempt: true,
            });
        }
    }

    None
}
