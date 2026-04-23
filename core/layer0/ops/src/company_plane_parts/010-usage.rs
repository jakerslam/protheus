// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::company_plane (authoritative)
// SRS coverage marker: V4-DUAL-GOV-003

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, conduit_bypass_requested,
    emit_plane_receipt, load_json_or, parse_bool, parse_u64, plane_status, print_json, read_json,
    scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "COMPANY_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "company_plane";

const ORG_CONTRACT_PATH: &str = "planes/contracts/company/org_hierarchy_contract_v1.json";
const BUDGET_CONTRACT_PATH: &str = "planes/contracts/company/per_agent_budget_contract_v1.json";
const TICKET_CONTRACT_PATH: &str = "planes/contracts/company/ticket_audit_contract_v1.json";
const HEARTBEAT_CONTRACT_PATH: &str = "planes/contracts/company/team_heartbeat_contract_v1.json";
const WEB_TOOLING_PROVIDER_TARGETS: [&str; 4] = ["brave", "duckduckgo", "moonshot", "xai"];

fn usage() {
    println!("Usage:");
    println!("  infring-ops company-plane status");
    println!(
        "  infring-ops company-plane orchestrate-agency --team=<id> [--org-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops company-plane budget-enforce --agent=<id> [--period=daily|weekly] [--tokens=<n>] [--cost-usd=<n>] [--compute-ms=<n>] [--privacy-units=<n>] [--web-requests=<n>] [--web-cost-usd=<n>] [--web-provider=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops company-plane ticket --op=<create|assign|transition|handoff|close|status> [--team=<id>] [--ticket-id=<id>] [--title=<text>] [--state=<id>] [--assignee=<id>] [--from=<id>] [--to=<id>] [--tool-call-id=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops company-plane heartbeat --op=<tick|status|remote-feed> [--team=<id>] [--status=<healthy|degraded|critical>] [--agents-online=<n>] [--queue-depth=<n>] [--strict=1|0]"
    );
    println!(
        "  company-plane web tooling provider targets (contract-aligned): {}",
        WEB_TOOLING_PROVIDER_TARGETS.as_slice().join(",")
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "company_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "company_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "orchestrate-agency" => vec!["V6-COMPANY-001.1", "V6-COMPANY-001.5"],
        "budget-enforce" => vec!["V6-COMPANY-001.2", "V6-COMPANY-001.5"],
        "ticket" => vec!["V6-COMPANY-001.3", "V6-COMPANY-001.5"],
        "heartbeat" => vec!["V6-COMPANY-001.4", "V6-COMPANY-001.5"],
        _ => vec!["V6-COMPANY-001.2"],
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
        "company_conduit_enforcement",
        "core/layer0/ops/company_plane",
        bypass_requested,
        "company_control_paths_are_conduit_routed_with_fail_closed_receipts",
        &claim_ids,
    )
}

fn team_slug(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= 80 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "default-team".to_string()
    } else {
        trimmed.to_string()
    }
}

fn run_orchestrate_agency(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        ORG_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "company_org_hierarchy_contract",
            "required_fields": ["org_chart", "reporting_edges", "titles", "team_goals"],
            "default_org_chart": ["head", "lead", "member"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("company_org_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "company_org_hierarchy_contract"
    {
        errors.push("company_org_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .or_else(|| parsed.positional.get(1).map(String::as_str))
            .unwrap_or("default-team"),
    );
    let hierarchy = parsed
        .flags
        .get("org-json")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .unwrap_or_else(|| {
            json!({
                "org_chart": contract
                    .get("default_org_chart")
                    .cloned()
                    .unwrap_or_else(|| json!(["head", "lead", "member"])),
                "reporting_edges": [
                    {"from": "head", "to": "lead"},
                    {"from": "lead", "to": "member"}
                ],
                "titles": {
                    "head": "Team Head",
                    "lead": "Team Lead",
                    "member": "Specialist"
                },
                "team_goals": [
                    "ship weekly quality improvements",
                    "keep safety gates green"
                ]
            })
        });
    for key in contract
        .get("required_fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
    {
        if strict && !hierarchy.get(key).is_some() {
            errors.push(format!("company_org_missing_field::{key}"));
        }
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_orchestrate_agency",
            "errors": errors
        });
    }

    let artifact = json!({
        "version": "v1",
        "team": team,
        "instantiated_at": crate::now_iso(),
        "hierarchy": hierarchy,
        "command_alias": format!("infring orchestrate agency {}", team)
    });
    let path = state_root(root).join("org").join(format!("{team}.json"));
    let _ = write_json(&path, &artifact);
    let _ = append_jsonl(
        &state_root(root).join("org").join("history.jsonl"),
        &artifact,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "company_plane_orchestrate_agency",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "hierarchy": artifact,
        "web_tooling_provider_targets": WEB_TOOLING_PROVIDER_TARGETS,
        "claim_evidence": [
            {
                "id": "V6-COMPANY-001.1",
                "claim": "company_layer_instantiates_org_chart_reporting_edges_titles_and_team_goals",
                "evidence": {
                    "team": team,
                    "reporting_edges": artifact
                        .get("hierarchy")
                        .and_then(|v| v.get("reporting_edges"))
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                    "web_tooling_provider_targets": WEB_TOOLING_PROVIDER_TARGETS
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn budget_bucket(period: &str) -> String {
    let now = crate::now_iso();
    let date = now.get(0..10).unwrap_or("1970-01-01");
    if period == "weekly" {
        let year = date.get(0..4).unwrap_or("1970");
        let month = date.get(5..7).unwrap_or("01");
        let day = date
            .get(8..10)
            .and_then(|d| d.parse::<u32>().ok())
            .unwrap_or(1);
        let week = ((day.saturating_sub(1)) / 7) + 1;
        format!("{year}-{month}-W{week}")
    } else {
        date.to_string()
    }
}

fn parse_f64(raw: Option<&String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn ticket_state_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("tickets")
        .join(format!("{team}.json"))
}

fn ticket_history_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("tickets")
        .join("history")
        .join(format!("{team}.jsonl"))
}

fn heartbeat_state_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("heartbeat")
        .join("teams")
        .join(format!("{team}.json"))
}

fn heartbeat_remote_feed_path(root: &Path) -> PathBuf {
    state_root(root).join("heartbeat").join("remote_feed.json")
}

fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn company_duality_clearance_tier(toll: &Value, harmony: f64) -> i64 {
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

fn load_duality_state_snapshot(root: &Path) -> Value {
    crate::duality_seed::invoke(root, "loadDualityState", None).unwrap_or_else(|_| json!({}))
}

fn company_heartbeat_duality_snapshot(
    root: &Path,
    team: &str,
    sequence: u64,
    status: &str,
    agents_online: u64,
    queue_depth: u64,
    persist: bool,
) -> Value {
    let run_id = format!("company-heartbeat-{team}-{sequence}");
    let context = json!({
        "lane": "weaver_arbitration",
        "source": "company_heartbeat",
        "run_id": run_id,
        "team": team,
        "status": status,
        "agents_online": agents_online,
        "queue_depth": queue_depth
    });

    let evaluation = match crate::duality_seed::invoke(
        root,
        "duality_evaluate",
        Some(&json!({
            "context": context,
            "opts": {
                "persist": persist,
                "source": "company_heartbeat",
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "company_heartbeat_duality",
                "error": format!("duality_evaluate_failed:{err}")
            });
        }
    };

    let toll_update = match crate::duality_seed::invoke(
        root,
        "duality_toll_update",
        Some(&json!({
            "context": {
                "lane": "weaver_arbitration",
                "source": "company_heartbeat",
                "run_id": run_id,
                "team": team,
                "status": status
            },
            "signal": evaluation.clone(),
            "opts": {
                "persist": persist,
                "source": "company_heartbeat",
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "company_heartbeat_duality",
                "evaluation": evaluation,
                "error": format!("duality_toll_update_failed:{err}")
            });
        }
    };

    let toll = toll_update.get("toll").cloned().unwrap_or_else(|| json!({}));
    let harmony = as_f64(evaluation.get("zero_point_harmony_potential"), 0.0).clamp(0.0, 1.0);
    let debt_after = as_f64(toll.get("debt_after"), 0.0).clamp(0.0, 100.0);
    let recommended_clearance_tier = company_duality_clearance_tier(&toll, harmony);
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    json!({
        "ok": true,
        "type": "company_heartbeat_duality",
        "run_id": run_id,
        "evaluation": evaluation,
        "toll": toll,
        "state": toll_update.get("state").cloned().unwrap_or(Value::Null),
        "hard_block": hard_block,
        "recommended_clearance_tier": recommended_clearance_tier,
        "fractal_balance_score": ((harmony * (1.0 - debt_after.min(1.0))) * 1_000_000.0).round() / 1_000_000.0
    })
}

fn ensure_ticket_ledger_shape(v: &mut Value) {
    if !v.is_object() {
        *v = json!({
            "version": "v1",
            "teams": {}
        });
    }
    if !v.get("teams").map(Value::is_object).unwrap_or(false) {
        v["teams"] = Value::Object(serde_json::Map::new());
    }
}

fn read_json_lines(path: &Path) -> Vec<Value> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn ticket_event_hash(row: &Value) -> Option<String> {
    let mut canonical = row.clone();
    let obj = canonical.as_object_mut()?;
    obj.remove("event_hash");
    Some(sha256_hex_str(&canonical.to_string()))
}

fn validate_ticket_history_rows(history_rows: &[Value]) -> (bool, Vec<String>) {
    let mut issues = Vec::<String>::new();
    let mut previous_event_hash = "genesis".to_string();
    for (idx, row) in history_rows.iter().enumerate() {
        let stored_hash = row.get("event_hash").and_then(Value::as_str).unwrap_or("");
        if stored_hash.is_empty() {
            issues.push(format!("missing_event_hash_row_{idx}"));
            continue;
        }
        let recomputed = ticket_event_hash(row).unwrap_or_default();
        if recomputed != stored_hash {
            issues.push(format!("event_hash_mismatch_row_{idx}"));
        }
        let claimed_prev = row
            .get("prev_event_hash")
            .and_then(Value::as_str)
            .unwrap_or("");
        if claimed_prev != previous_event_hash {
            issues.push(format!("prev_hash_mismatch_row_{idx}"));
        }
        previous_event_hash = stored_hash.to_string();
    }
    (issues.is_empty(), issues)
}
