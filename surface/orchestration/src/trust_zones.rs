use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_TRUST_ZONES_PATH: &str = "surface/orchestration/config/trust_zones.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/trust_zone_guard_current.json";
const DEFAULT_REPORT_PATH: &str = "local/workspace/reports/TRUST_ZONE_GUARD_CURRENT.md";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrustZonePolicy {
    pub schema_version: u32,
    pub default_zone: String,
    pub zones: Vec<TrustZoneRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrustZoneRule {
    pub id: String,
    pub zone: String,
    pub description: String,
    pub path_prefixes: Vec<String>,
    pub apply_allowed: bool,
    pub propose_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrustZoneEvaluation {
    pub target_path: String,
    pub zone_id: String,
    pub zone: String,
    pub matched_prefix: String,
    pub propose_allowed: bool,
    pub apply_allowed: bool,
    pub enforced: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CheckRow {
    id: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct TrustZoneGuardReport {
    ok: bool,
    r#type: String,
    schema_version: u32,
    generated_unix_seconds: u64,
    policy_path: String,
    evaluations: Vec<TrustZoneEvaluation>,
    checks: Vec<CheckRow>,
}

pub fn run_trust_zone_guard(args: &[String]) -> i32 {
    let strict = flag_value(args, "--strict").unwrap_or_else(|| "0".to_string()) == "1";
    let policy_path = flag_value(args, "--policy").unwrap_or_else(|| DEFAULT_TRUST_ZONES_PATH.to_string());
    let out_path = flag_value(args, "--out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let report_path = flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());
    let targets = flag_value(args, "--targets")
        .map(|raw| split_targets(&raw))
        .unwrap_or_else(default_probe_targets);

    let policy = match load_trust_zone_policy(&policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!("trust_zone_guard: failed to load policy {policy_path}: {err}");
            return 1;
        }
    };
    let report = build_guard_report(&policy_path, &policy, &targets);
    let wrote = write_json(&out_path, &report) && write_markdown(&report_path, &report);
    if !wrote {
        eprintln!("trust_zone_guard: failed to write outputs");
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

pub fn load_trust_zone_policy(path: &str) -> Result<TrustZonePolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

pub fn evaluate_target_paths(
    policy: &TrustZonePolicy,
    target_paths: &[String],
) -> Vec<TrustZoneEvaluation> {
    target_paths
        .iter()
        .map(|target| evaluate_target_path(policy, target))
        .collect()
}

pub fn all_apply_allowed(evaluations: &[TrustZoneEvaluation]) -> bool {
    evaluations.iter().all(|row| row.apply_allowed)
}

pub fn all_enforced(evaluations: &[TrustZoneEvaluation]) -> bool {
    !evaluations.is_empty() && evaluations.iter().all(|row| row.enforced)
}

fn build_guard_report(
    policy_path: &str,
    policy: &TrustZonePolicy,
    targets: &[String],
) -> TrustZoneGuardReport {
    let evaluations = evaluate_target_paths(policy, targets);
    let labels = policy
        .zones
        .iter()
        .map(|zone| zone.zone.as_str())
        .collect::<Vec<_>>();
    let has_immutable = labels.iter().any(|label| *label == "immutable");
    let has_propose_only = labels.iter().any(|label| *label == "propose_only");
    let has_mutable = labels.iter().any(|label| *label == "mutable");
    let kernel_denied = evaluations
        .iter()
        .any(|row| row.target_path.starts_with("core/") && !row.apply_allowed);
    let policy_denied = evaluations
        .iter()
        .any(|row| row.target_path.contains("policy") && !row.apply_allowed);
    let control_plane_denied = evaluations
        .iter()
        .any(|row| row.target_path.starts_with("surface/orchestration/") && !row.apply_allowed);
    let shell_mutable = evaluations
        .iter()
        .any(|row| row.target_path.starts_with("client/") && row.apply_allowed);
    let gateway_mutable = evaluations
        .iter()
        .any(|row| row.target_path.starts_with("adapters/") && row.apply_allowed);
    let checks = vec![
        CheckRow {
            id: "trust_zone_schema_contract".to_string(),
            ok: policy.schema_version == 1,
            detail: format!("schema_version={}", policy.schema_version),
        },
        CheckRow {
            id: "trust_zone_required_labels_contract".to_string(),
            ok: has_immutable && has_propose_only && has_mutable,
            detail: format!("labels={}", labels.join(",")),
        },
        CheckRow {
            id: "trust_zone_all_targets_classified_contract".to_string(),
            ok: all_enforced(&evaluations),
            detail: format!("classified={}", evaluations.len()),
        },
        CheckRow {
            id: "trust_zone_kernel_apply_denied_contract".to_string(),
            ok: kernel_denied,
            detail: "core/** must not be directly self-modified".to_string(),
        },
        CheckRow {
            id: "trust_zone_policy_apply_denied_contract".to_string(),
            ok: policy_denied,
            detail: "policy/receipt paths must not be directly self-modified".to_string(),
        },
        CheckRow {
            id: "trust_zone_control_plane_propose_only_contract".to_string(),
            ok: control_plane_denied,
            detail: "surface/orchestration/** is propose-only for self-modification".to_string(),
        },
        CheckRow {
            id: "trust_zone_shell_gateway_mutable_contract".to_string(),
            ok: shell_mutable && gateway_mutable,
            detail: format!("shell_mutable={shell_mutable};gateway_mutable={gateway_mutable}"),
        },
    ];
    let ok = checks.iter().all(|check| check.ok);
    TrustZoneGuardReport {
        ok,
        r#type: "trust_zone_guard".to_string(),
        schema_version: policy.schema_version,
        generated_unix_seconds: now_unix_seconds(),
        policy_path: policy_path.to_string(),
        evaluations,
        checks,
    }
}

fn evaluate_target_path(policy: &TrustZonePolicy, target_path: &str) -> TrustZoneEvaluation {
    let normalized = normalize_path(target_path);
    let matched = policy
        .zones
        .iter()
        .flat_map(|zone| {
            zone.path_prefixes
                .iter()
                .map(move |prefix| (zone, normalize_path(prefix)))
        })
        .filter(|(_, prefix)| normalized == *prefix || normalized.starts_with(prefix))
        .max_by_key(|(_, prefix)| prefix.len());

    match matched {
        Some((zone, prefix)) => TrustZoneEvaluation {
            target_path: normalized,
            zone_id: zone.id.clone(),
            zone: zone.zone.clone(),
            matched_prefix: prefix,
            propose_allowed: zone.propose_allowed,
            apply_allowed: zone.apply_allowed,
            enforced: true,
            reason: reason_for_zone(zone),
        },
        None => TrustZoneEvaluation {
            target_path: normalized,
            zone_id: "default".to_string(),
            zone: policy.default_zone.clone(),
            matched_prefix: "<default>".to_string(),
            propose_allowed: true,
            apply_allowed: false,
            enforced: true,
            reason: "default_zone_requires_proposal_review".to_string(),
        },
    }
}

fn reason_for_zone(zone: &TrustZoneRule) -> String {
    match zone.zone.as_str() {
        "immutable" => "immutable_zone_blocks_apply".to_string(),
        "propose_only" => "propose_only_zone_blocks_apply_until_operator_pipeline".to_string(),
        "mutable" => "mutable_zone_allows_apply_after_pipeline_gates".to_string(),
        other => format!("unknown_zone_{other}_defaults_to_policy"),
    }
}

fn split_targets(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_probe_targets() -> Vec<String> {
    vec![
        "core/layer0/ops/src/lib.rs".to_string(),
        "planes/contracts/orchestration/workflow_phase_trace_v1.json".to_string(),
        "surface/orchestration/config/self_modification_policy.json".to_string(),
        "surface/orchestration/src/self_modification.rs".to_string(),
        "client/runtime/systems/ui/infring_static/js/app.ts".to_string(),
        "adapters/runtime/dev_only/legacy_runner.rs".to_string(),
    ]
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn write_json(path: &str, value: &TrustZoneGuardReport) -> bool {
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

fn write_markdown(path: &str, report: &TrustZoneGuardReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let mut body = String::new();
    body.push_str("# Trust Zone Guard Current\n\n");
    body.push_str(&format!("- ok: {}\n", report.ok));
    body.push_str(&format!("- policy: `{}`\n", report.policy_path));
    body.push_str("- evaluations:\n");
    for row in &report.evaluations {
        body.push_str(&format!(
            "  - `{}` => `{}` / apply_allowed={} ({})\n",
            row.target_path, row.zone, row.apply_allowed, row.reason
        ));
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

pub fn trust_zone_summary(evaluations: &[TrustZoneEvaluation]) -> serde_json::Value {
    json!({
        "target_count": evaluations.len(),
        "all_enforced": all_enforced(evaluations),
        "all_apply_allowed": all_apply_allowed(evaluations),
        "zones": evaluations.iter().map(|row| {
            json!({
                "target_path": row.target_path,
                "zone": row.zone,
                "apply_allowed": row.apply_allowed,
                "reason": row.reason,
            })
        }).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> TrustZonePolicy {
        TrustZonePolicy {
            schema_version: 1,
            default_zone: "propose_only".to_string(),
            zones: vec![
                TrustZoneRule {
                    id: "kernel_immutable".to_string(),
                    zone: "immutable".to_string(),
                    description: "Kernel".to_string(),
                    path_prefixes: vec!["core/".to_string()],
                    apply_allowed: false,
                    propose_allowed: true,
                },
                TrustZoneRule {
                    id: "control_plane".to_string(),
                    zone: "propose_only".to_string(),
                    description: "Control plane".to_string(),
                    path_prefixes: vec!["surface/orchestration/".to_string()],
                    apply_allowed: false,
                    propose_allowed: true,
                },
                TrustZoneRule {
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
    fn immutable_and_propose_only_zones_block_apply() {
        let rows = evaluate_target_paths(
            &policy(),
            &[
                "core/layer0/ops/src/lib.rs".to_string(),
                "surface/orchestration/src/self_modification.rs".to_string(),
            ],
        );
        assert!(rows.iter().all(|row| row.enforced));
        assert!(rows.iter().all(|row| !row.apply_allowed));
    }

    #[test]
    fn mutable_gateway_zone_allows_apply_after_pipeline() {
        let rows = evaluate_target_paths(
            &policy(),
            &["adapters/runtime/dev_only/legacy_runner.rs".to_string()],
        );
        assert!(all_apply_allowed(&rows));
        assert_eq!(rows[0].zone, "mutable");
    }
}
