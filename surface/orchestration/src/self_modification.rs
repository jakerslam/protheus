use crate::trust_zones::{
    all_apply_allowed, all_enforced, evaluate_target_paths, load_trust_zone_policy,
    trust_zone_summary, TrustZoneEvaluation, DEFAULT_TRUST_ZONES_PATH,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY_PATH: &str = "surface/orchestration/config/self_modification_policy.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/self_modification_guard_current.json";
const DEFAULT_REPORT_PATH: &str = "local/workspace/reports/SELF_MODIFICATION_GUARD_CURRENT.md";

#[derive(Debug, Clone, Deserialize)]
struct SelfModificationPolicy {
    schema_version: u32,
    required_stages: Vec<String>,
    required_gates: Vec<GatePolicy>,
    apply_requirements: Vec<String>,
    rollback_metadata_required: bool,
    bypass_tokens_denied: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatePolicy {
    id: String,
    path: String,
    required: bool,
    allow_blocked_eval: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProposalInput {
    proposal_id: String,
    target_paths: Vec<String>,
    summary: String,
    requested_stages: Vec<String>,
    requested_bypass: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GateObservation {
    id: String,
    path: String,
    required: bool,
    present: bool,
    ok: Option<bool>,
    accepted: bool,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct StageDecision {
    stage: String,
    status: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct RollbackMetadata {
    rollback_required: bool,
    rollback_receipt_id: String,
    rollback_trigger: String,
    rollback_plan: String,
}

#[derive(Debug, Clone, Serialize)]
struct CheckRow {
    id: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct SelfModificationReport {
    ok: bool,
    r#type: String,
    schema_version: u32,
    generated_unix_seconds: u64,
    proposal: ProposalInput,
    stages: Vec<StageDecision>,
    gates: Vec<GateObservation>,
    apply_allowed: bool,
    apply_decision: String,
    monitor_plan: Value,
    rollback: RollbackMetadata,
    trust_zones: Vec<TrustZoneEvaluation>,
    trust_zone_summary: Value,
    bypass_enforcement: Value,
    checks: Vec<CheckRow>,
}

pub fn run_self_modification_guard(args: &[String]) -> i32 {
    let strict = flag_value(args, "--strict").unwrap_or_else(|| "0".to_string()) == "1";
    let policy_path = flag_value(args, "--policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let trust_zones_path =
        flag_value(args, "--trust-zones").unwrap_or_else(|| DEFAULT_TRUST_ZONES_PATH.to_string());
    let proposal_path = flag_value(args, "--proposal");
    let out_path = flag_value(args, "--out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let report_path = flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let policy = match load_policy(&policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!("self_modification_guard: failed to load policy {policy_path}: {err}");
            return 1;
        }
    };
    let trust_zone_policy = match load_trust_zone_policy(&trust_zones_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!(
                "self_modification_guard: failed to load trust zones {trust_zones_path}: {err}"
            );
            return 1;
        }
    };
    let proposal = match proposal_path {
        Some(path) => match load_proposal(&path) {
            Ok(proposal) => proposal,
            Err(err) => {
                eprintln!("self_modification_guard: failed to load proposal {path}: {err}");
                return 1;
            }
        },
        None => default_proposal(),
    };

    let report = build_self_modification_report(&policy, &trust_zone_policy, proposal);
    let wrote = write_json(&out_path, &report) && write_markdown(&report_path, &report);
    if !wrote {
        eprintln!("self_modification_guard: failed to write outputs");
        return 1;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    if strict && !report.ok {
        return 1;
    }
    0
}

fn load_policy(path: &str) -> Result<SelfModificationPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn load_proposal(path: &str) -> Result<ProposalInput, String> {
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn default_proposal() -> ProposalInput {
    ProposalInput {
        proposal_id: "self_modification_guard_default_probe".to_string(),
        target_paths: vec!["surface/orchestration/src/self_modification.rs".to_string()],
        summary: "Default fail-closed self-modification probe".to_string(),
        requested_stages: vec![
            "propose".to_string(),
            "validate".to_string(),
            "apply".to_string(),
            "monitor".to_string(),
            "rollback".to_string(),
        ],
        requested_bypass: None,
    }
}

fn build_self_modification_report(
    policy: &SelfModificationPolicy,
    trust_zone_policy: &crate::trust_zones::TrustZonePolicy,
    proposal: ProposalInput,
) -> SelfModificationReport {
    let gates = observe_gates(&policy.required_gates);
    let trust_zones = evaluate_target_paths(trust_zone_policy, &proposal.target_paths);
    let trust_zones_enforced = all_enforced(&trust_zones);
    let trust_zones_apply_allowed = all_apply_allowed(&trust_zones);
    let stage_complete = stages_match(&policy.required_stages, &proposal.requested_stages);
    let required_gates_present = gates
        .iter()
        .filter(|gate| gate.required)
        .all(|gate| gate.present);
    let gates_accepted = gates
        .iter()
        .filter(|gate| gate.required)
        .all(|gate| gate.accepted);
    let bypass_denied = bypass_request_denied(policy, &proposal);
    let rollback = rollback_metadata(policy, &proposal);
    let rollback_ok = !policy.rollback_metadata_required || rollback.rollback_required;
    let apply_requirements_present = requirements_present(policy);
    let apply_allowed = stage_complete
        && gates_accepted
        && bypass_denied
        && rollback_ok
        && trust_zones_apply_allowed
        && apply_requirements_present;
    let stages = stage_decisions(
        policy,
        &proposal,
        gates_accepted,
        rollback_ok,
        bypass_denied,
        trust_zones_apply_allowed,
    );
    let monitor_plan = json!({
        "post_apply_monitoring_required": true,
        "monitored_artifacts": [
            "core/local/artifacts/live_eval_current.json",
            "core/local/artifacts/eval_regression_guard_current.json",
            "local/state/ops/orchestration/workflow_phase_trace_latest.json"
        ],
        "regression_action": "auto_rollback_and_kernel_block",
        "monitor_window": "next_eval_sampling_cycle"
    });
    let checks = vec![
        CheckRow {
            id: "self_modification_policy_schema_contract".to_string(),
            ok: policy.schema_version == 1,
            detail: format!("schema_version={}", policy.schema_version),
        },
        CheckRow {
            id: "self_modification_stage_sequence_contract".to_string(),
            ok: stage_complete,
            detail: policy.required_stages.join(","),
        },
        CheckRow {
            id: "self_modification_required_gates_contract".to_string(),
            ok: required_gates_present,
            detail: format!("accepted_required_gates={}", gates.iter().filter(|g| g.required && g.accepted).count()),
        },
        CheckRow {
            id: "self_modification_apply_requirements_contract".to_string(),
            ok: apply_requirements_present,
            detail: policy.apply_requirements.join(","),
        },
        CheckRow {
            id: "self_modification_rollback_metadata_contract".to_string(),
            ok: rollback_ok,
            detail: rollback.rollback_receipt_id.clone(),
        },
        CheckRow {
            id: "self_modification_bypass_denied_contract".to_string(),
            ok: bypass_denied,
            detail: proposal.requested_bypass.clone().unwrap_or_else(|| "none".to_string()),
        },
        CheckRow {
            id: "self_modification_trust_zone_classification_contract".to_string(),
            ok: trust_zones_enforced,
            detail: format!("classified_targets={}", trust_zones.len()),
        },
        CheckRow {
            id: "self_modification_trust_zone_apply_enforcement_contract".to_string(),
            ok: trust_zones_enforced && (trust_zones_apply_allowed || !apply_allowed),
            detail: format!("trust_zone_apply_allowed={trust_zones_apply_allowed}"),
        },
    ];
    let ok = checks.iter().all(|check| check.ok);

    SelfModificationReport {
        ok,
        r#type: "self_modification_guard".to_string(),
        schema_version: policy.schema_version,
        generated_unix_seconds: now_unix_seconds(),
        proposal,
        stages,
        gates,
        apply_allowed,
        apply_decision: if apply_allowed {
            "apply_permitted_after_policy_pipeline".to_string()
        } else {
            "apply_blocked_until_pipeline_complete".to_string()
        },
        monitor_plan,
        rollback,
        trust_zone_summary: trust_zone_summary(&trust_zones),
        trust_zones,
        bypass_enforcement: json!({
            "runtime_bypass_blocked": bypass_denied,
            "denied_tokens": policy.bypass_tokens_denied,
            "allowed_entrypoint": "self_modification_guard"
        }),
        checks,
    }
}

fn stages_match(required: &[String], requested: &[String]) -> bool {
    required == requested
}

fn requirements_present(policy: &SelfModificationPolicy) -> bool {
    let required = ["tests", "replay_fixtures", "eval_gates", "monitoring", "rollback"];
    required
        .iter()
        .all(|item| policy.apply_requirements.iter().any(|row| row == item))
}

fn observe_gates(gates: &[GatePolicy]) -> Vec<GateObservation> {
    gates
        .iter()
        .map(|gate| {
            let value = fs::read_to_string(&gate.path)
                .ok()
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
            let present = value.is_some();
            let ok = value
                .as_ref()
                .and_then(|payload| payload.get("ok"))
                .and_then(Value::as_bool);
            let blocked_eval = value
                .as_ref()
                .and_then(|payload| payload.get("summary"))
                .and_then(|summary| summary.get("eval_release_gate"))
                .and_then(Value::as_str)
                == Some("blocked");
            let accepted = present
                && ((ok == Some(true) && !blocked_eval) || (gate.allow_blocked_eval && blocked_eval));
            let reason = if accepted {
                "gate_accepted".to_string()
            } else if !present {
                "gate_artifact_missing".to_string()
            } else {
                "gate_not_passing".to_string()
            };
            GateObservation {
                id: gate.id.clone(),
                path: gate.path.clone(),
                required: gate.required,
                present,
                ok,
                accepted,
                reason,
            }
        })
        .collect()
}

fn bypass_request_denied(policy: &SelfModificationPolicy, proposal: &ProposalInput) -> bool {
    match proposal.requested_bypass.as_deref() {
        None | Some("") => true,
        Some(token) => !policy
            .bypass_tokens_denied
            .iter()
            .any(|denied| denied == token),
    }
}

fn rollback_metadata(policy: &SelfModificationPolicy, proposal: &ProposalInput) -> RollbackMetadata {
    RollbackMetadata {
        rollback_required: policy.rollback_metadata_required,
        rollback_receipt_id: format!("rollback::{}", proposal.proposal_id),
        rollback_trigger: "post_apply_eval_or_replay_regression".to_string(),
        rollback_plan: "restore previous artifact snapshot, block apply lane, and rerun eval/replay gates".to_string(),
    }
}

fn stage_decisions(
    policy: &SelfModificationPolicy,
    proposal: &ProposalInput,
    gates_accepted: bool,
    rollback_ok: bool,
    bypass_denied: bool,
    trust_zones_apply_allowed: bool,
) -> Vec<StageDecision> {
    policy
        .required_stages
        .iter()
        .map(|stage| {
            let requested = proposal.requested_stages.iter().any(|row| row == stage);
            let (status, reason) = match stage.as_str() {
                "propose" if requested => ("complete", "proposal_recorded"),
                "validate" if requested && gates_accepted => ("complete", "tests_replay_eval_gates_accepted"),
                "apply" if requested && !trust_zones_apply_allowed => {
                    ("blocked", "apply_blocked_by_trust_zone")
                }
                "apply" if requested && gates_accepted && rollback_ok && bypass_denied => {
                    ("eligible", "apply_allowed_only_after_required_gates")
                }
                "apply" if requested => ("blocked", "apply_blocked_by_policy_pipeline"),
                "monitor" if requested => ("planned", "post_apply_monitoring_required"),
                "rollback" if requested && rollback_ok => ("planned", "rollback_metadata_ready"),
                _ if requested => ("blocked", "stage_policy_not_satisfied"),
                _ => ("missing", "stage_not_requested"),
            };
            StageDecision {
                stage: stage.clone(),
                status: status.to_string(),
                reason: reason.to_string(),
            }
        })
        .collect()
}

fn write_json(path: &str, value: &SelfModificationReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    serde_json::to_string_pretty(value)
        .ok()
        .and_then(|raw| fs::write(path, format!("{raw}\n")).ok())
        .is_some()
}

fn write_markdown(path: &str, report: &SelfModificationReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let mut body = String::new();
    body.push_str("# Self Modification Guard Current\n\n");
    body.push_str(&format!("- ok: {}\n", report.ok));
    body.push_str(&format!("- proposal: `{}`\n", report.proposal.proposal_id));
    body.push_str(&format!("- apply_allowed: {}\n", report.apply_allowed));
    body.push_str(&format!("- apply_decision: `{}`\n", report.apply_decision));
    body.push_str("- gates:\n");
    for gate in &report.gates {
        body.push_str(&format!("  - `{}`: {} ({})\n", gate.id, gate.accepted, gate.reason));
    }
    fs::write(path, body).is_ok()
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .find_map(|arg| arg.strip_prefix(&format!("{flag}=")).map(|value| value.to_string()))
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SelfModificationPolicy {
        SelfModificationPolicy {
            schema_version: 1,
            required_stages: vec!["propose", "validate", "apply", "monitor", "rollback"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            required_gates: vec![],
            apply_requirements: vec!["tests", "replay_fixtures", "eval_gates", "monitoring", "rollback"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            rollback_metadata_required: true,
            bypass_tokens_denied: vec!["direct_apply".to_string()],
        }
    }

    fn trust_zone_policy() -> crate::trust_zones::TrustZonePolicy {
        crate::trust_zones::TrustZonePolicy {
            schema_version: 1,
            default_zone: "propose_only".to_string(),
            zones: vec![
                crate::trust_zones::TrustZoneRule {
                    id: "control_plane".to_string(),
                    zone: "propose_only".to_string(),
                    description: "Control plane".to_string(),
                    path_prefixes: vec!["surface/orchestration/".to_string()],
                    apply_allowed: false,
                    propose_allowed: true,
                },
                crate::trust_zones::TrustZoneRule {
                    id: "gateway_mutable".to_string(),
                    zone: "mutable".to_string(),
                    description: "Gateways".to_string(),
                    path_prefixes: vec!["adapters/".to_string()],
                    apply_allowed: true,
                    propose_allowed: true,
                },
            ],
        }
    }

    #[test]
    fn full_pipeline_allows_apply_when_required_controls_are_present() {
        let mut proposal = default_proposal();
        proposal.target_paths = vec!["adapters/runtime/dev_only/legacy_runner.rs".to_string()];
        let report = build_self_modification_report(&policy(), &trust_zone_policy(), proposal);
        assert!(report.ok);
        assert!(report.apply_allowed);
        assert!(report.stages.iter().any(|stage| stage.stage == "rollback"));
    }

    #[test]
    fn denied_bypass_blocks_apply() {
        let mut proposal = default_proposal();
        proposal.target_paths = vec!["adapters/runtime/dev_only/legacy_runner.rs".to_string()];
        proposal.requested_bypass = Some("direct_apply".to_string());
        let report = build_self_modification_report(&policy(), &trust_zone_policy(), proposal);
        assert!(!report.ok);
        assert!(!report.apply_allowed);
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "self_modification_bypass_denied_contract" && !check.ok));
    }

    #[test]
    fn trust_zone_blocks_control_plane_apply() {
        let report =
            build_self_modification_report(&policy(), &trust_zone_policy(), default_proposal());
        assert!(report.ok);
        assert!(!report.apply_allowed);
        assert!(report
            .stages
            .iter()
            .any(|stage| stage.stage == "apply" && stage.reason == "apply_blocked_by_trust_zone"));
    }
}
