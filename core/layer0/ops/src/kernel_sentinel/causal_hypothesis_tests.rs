use super::*;
use crate::kernel_sentinel::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};

fn finding(fingerprint: &str, summary: &str, evidence: Vec<&str>) -> KernelSentinelFinding {
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "finding-1".to_string(),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::GatewayIsolation,
        fingerprint: fingerprint.to_string(),
        evidence: evidence.into_iter().map(str::to_string).collect(),
        summary: summary.to_string(),
        recommended_action: "inspect deterministic kernel evidence".to_string(),
        status: "open".to_string(),
    }
}

#[test]
fn codesigning_failure_becomes_testable_root_cause() {
    let row = finding(
        "gateway_start_failed",
        "launchd killed runtime with OS_REASON_CODESIGNING and exit 137",
        vec!["launchd://ai.infring.gateway/OS_REASON_CODESIGNING"],
    );
    let report = build_kernel_sentinel_causal_hypotheses(&[row], &json!({}), &[]);
    let first = &report["top_hypotheses"][0];
    assert_eq!(first["pattern"], "installed_runtime_identity_invalid");
    assert!(first["confidence_percent"].as_u64().unwrap() >= 70);
    assert!(first["falsification_probe"]["probe"]
        .as_str()
        .unwrap()
        .contains("codesign"));
}

#[test]
fn gateway_health_conflict_names_durable_listener_invariant() {
    let row = finding(
        "dashboard_healthz_not_ready",
        "healthz not ready while alternate_healthz_ready on 5173",
        vec!["gateway://healthz/4173", "health://alternate_healthz_ready/5173"],
    );
    let report = build_kernel_sentinel_causal_hypotheses(&[row], &json!({}), &[]);
    let first = &report["top_hypotheses"][0];
    assert_eq!(first["pattern"], "gateway_lifecycle_truth_contradiction");
    assert_eq!(
        first["causal_ladder"]["violated_invariant"],
        "gateway_success_requires_durable_listener"
    );
    assert!(first["counter_evidence"][0]
        .as_str()
        .unwrap()
        .contains("alternate route is healthy"));
}

#[test]
fn issue_text_uses_causal_pattern_not_generic_frame() {
    let row = finding(
        "authority_ghost_projection_cache",
        "authority ghost survived syntax removal through fallback route shape",
        vec!["shape://authority_ghost/fallback_route"],
    );
    let text = root_cause_hypothesis_text(
        &row,
        &row.evidence,
        "shell_is_projection_only",
        &json!({"root_frame": "authority_policy_contradiction"}),
    );
    assert!(text.contains("authority_shape_residue"));
    assert!(text.contains("shell_is_projection_only"));
}
