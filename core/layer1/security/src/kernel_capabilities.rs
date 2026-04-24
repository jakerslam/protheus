use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const DEFAULT_KERNEL_CAPABILITY_POLICY_PATH: &str =
    "core/layer1/security/config/kernel_capability_policy.json";
pub const DEFAULT_KERNEL_CAPABILITY_REPORT_PATH: &str =
    "core/local/artifacts/kernel_capability_guard_current.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDefinition {
    pub id: String,
    pub description: String,
    pub authority: String,
    #[serde(default)]
    pub protected_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectedActionPolicy {
    pub action: String,
    pub required_capability: String,
    pub receipt_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityReceiptRequirements {
    pub required: bool,
    pub include_granted_records: bool,
    pub include_denied_records: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelCapabilityPolicy {
    pub version: String,
    pub owner: String,
    pub receipt_requirements: CapabilityReceiptRequirements,
    pub capabilities: Vec<CapabilityDefinition>,
    pub protected_actions: Vec<ProtectedActionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub principal: String,
    pub action: String,
    #[serde(default)]
    pub granted_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityReceiptRecord {
    pub receipt_type: String,
    pub request_id: String,
    pub principal: String,
    pub action: String,
    pub capability: String,
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDecision {
    pub request_id: String,
    pub action: String,
    pub required_capability: String,
    pub granted: bool,
    pub reason: String,
    pub receipt: CapabilityReceiptRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCapabilityGuardReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub generated_at: String,
    pub policy_path: String,
    pub summary: Value,
    pub checks: Vec<Value>,
    pub decisions: Vec<CapabilityDecision>,
}

pub fn load_kernel_capability_policy(path: &str) -> Result<KernelCapabilityPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_policy_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_policy_failed:{err}"))
}

pub fn capability_ids(policy: &KernelCapabilityPolicy) -> BTreeSet<String> {
    policy
        .capabilities
        .iter()
        .map(|capability| capability.id.clone())
        .collect()
}

pub fn required_action_policy<'a>(
    policy: &'a KernelCapabilityPolicy,
    action: &str,
) -> Option<&'a ProtectedActionPolicy> {
    policy
        .protected_actions
        .iter()
        .find(|protected| protected.action == action)
}

pub fn check_capability(
    policy: &KernelCapabilityPolicy,
    request: &CapabilityRequest,
) -> CapabilityDecision {
    let Some(action_policy) = required_action_policy(policy, request.action.as_str()) else {
        return decision_for(
            request,
            "unknown".to_string(),
            "kernel_capability_check_receipt".to_string(),
            false,
            format!("unprotected_or_unknown_action:{}", request.action),
        );
    };
    let required = action_policy.required_capability.clone();
    let grant_set: BTreeSet<&str> = request
        .granted_capabilities
        .iter()
        .map(|capability| capability.as_str())
        .collect();
    let granted = grant_set.contains(required.as_str());
    let reason = if granted {
        format!("granted_capability:{required}")
    } else {
        format!("missing_capability:{required}")
    };
    decision_for(
        request,
        required,
        action_policy.receipt_type.clone(),
        granted,
        reason,
    )
}

pub fn evaluate_capability_requests(
    policy: &KernelCapabilityPolicy,
    requests: &[CapabilityRequest],
) -> Vec<CapabilityDecision> {
    requests
        .iter()
        .map(|request| check_capability(policy, request))
        .collect()
}

pub fn default_capability_requests() -> Vec<CapabilityRequest> {
    vec![
        request("workspace_read_granted", "operator", "read_file", &["read_file"]),
        request(
            "workspace_search_granted",
            "operator",
            "search_workspace",
            &["search_workspace"],
        ),
        request("web_call_granted", "operator", "call_web", &["call_web"]),
        request(
            "mutation_denied_without_grant",
            "operator",
            "mutate_state",
            &["read_file", "search_workspace"],
        ),
    ]
}

pub fn build_kernel_capability_guard_report(
    policy_path: &str,
    policy: &KernelCapabilityPolicy,
    requests: &[CapabilityRequest],
) -> KernelCapabilityGuardReport {
    let ids = capability_ids(policy);
    let decisions = evaluate_capability_requests(policy, requests);
    let required_capabilities = ["read_file", "search_workspace", "call_web", "mutate_state"];
    let required_actions = ["read_file", "search_workspace", "call_web", "mutate_state"];
    let model_ok = required_capabilities
        .iter()
        .all(|required| ids.contains(*required));
    let action_mapping_ok = required_actions.iter().all(|action| {
        required_action_policy(policy, action)
            .map(|action_policy| action_policy.required_capability == *action)
            .unwrap_or(false)
    });
    let grant_ok = decisions
        .iter()
        .filter(|decision| decision.request_id.ends_with("_granted"))
        .all(|decision| decision.granted);
    let denial_ok = decisions.iter().any(|decision| {
        !decision.granted && decision.reason == "missing_capability:mutate_state"
    });
    let receipt_ok = decisions.iter().all(|decision| {
        decision.receipt.receipt_type == "kernel_capability_check_receipt"
            && decision.receipt.capability == decision.required_capability
            && matches!(decision.receipt.decision.as_str(), "granted" | "denied")
            && !decision.receipt.reason.is_empty()
    });
    let receipt_policy_ok = policy.receipt_requirements.required
        && policy.receipt_requirements.include_granted_records
        && policy.receipt_requirements.include_denied_records;
    let checks = vec![
        check_row("kernel_capability_model_required_set_contract", model_ok),
        check_row("kernel_capability_action_mapping_contract", action_mapping_ok),
        check_row("kernel_capability_pre_execution_grant_contract", grant_ok),
        check_row("kernel_capability_pre_execution_denial_contract", denial_ok),
        check_row("kernel_capability_receipt_contract", receipt_ok),
        check_row("kernel_capability_receipt_policy_contract", receipt_policy_ok),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));
    KernelCapabilityGuardReport {
        ok,
        report_type: "kernel_capability_guard".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        policy_path: policy_path.to_string(),
        summary: json!({
            "capability_count": policy.capabilities.len(),
            "protected_action_count": policy.protected_actions.len(),
            "decision_count": decisions.len(),
            "granted_count": decisions.iter().filter(|decision| decision.granted).count(),
            "denied_count": decisions.iter().filter(|decision| !decision.granted).count(),
            "required_capabilities": required_capabilities,
            "pass": ok
        }),
        checks,
        decisions,
    }
}

pub fn run_kernel_capability_guard(
    policy_path: &str,
    out_json: &str,
    strict: bool,
) -> Result<KernelCapabilityGuardReport, String> {
    let policy = load_kernel_capability_policy(policy_path)?;
    let report = build_kernel_capability_guard_report(
        policy_path,
        &policy,
        default_capability_requests().as_slice(),
    );
    write_report(out_json, &report)?;
    if strict && !report.ok {
        return Err("kernel_capability_guard_failed".to_string());
    }
    Ok(report)
}

fn decision_for(
    request: &CapabilityRequest,
    required_capability: String,
    receipt_type: String,
    granted: bool,
    reason: String,
) -> CapabilityDecision {
    let decision = if granted { "granted" } else { "denied" }.to_string();
    CapabilityDecision {
        request_id: request.request_id.clone(),
        action: request.action.clone(),
        required_capability: required_capability.clone(),
        granted,
        reason: reason.clone(),
        receipt: CapabilityReceiptRecord {
            receipt_type,
            request_id: request.request_id.clone(),
            principal: request.principal.clone(),
            action: request.action.clone(),
            capability: required_capability,
            decision,
            reason,
        },
    }
}

fn request(
    request_id: &str,
    principal: &str,
    action: &str,
    granted_capabilities: &[&str],
) -> CapabilityRequest {
    CapabilityRequest {
        request_id: request_id.to_string(),
        principal: principal.to_string(),
        action: action.to_string(),
        granted_capabilities: granted_capabilities
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
    }
}

fn check_row(id: &str, ok: bool) -> Value {
    json!({ "id": id, "ok": ok })
}

fn write_report(out_json: &str, report: &KernelCapabilityGuardReport) -> Result<(), String> {
    let path = Path::new(out_json);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let raw = serde_json::to_string_pretty(report)
        .map_err(|err| format!("serialize_report_failed:{err}"))?;
    fs::write(path, format!("{raw}\n")).map_err(|err| format!("write_report_failed:{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> KernelCapabilityPolicy {
        serde_json::from_str(include_str!(
            "../config/kernel_capability_policy.json"
        ))
        .expect("policy")
    }

    #[test]
    fn capability_model_contains_required_kernel_actions() {
        let policy = policy();
        let ids = capability_ids(&policy);
        for required in ["read_file", "search_workspace", "call_web", "mutate_state"] {
            assert!(ids.contains(required), "missing {required}");
        }
    }

    #[test]
    fn missing_grant_denies_before_execution() {
        let policy = policy();
        let request = request("deny_mutation", "operator", "mutate_state", &["read_file"]);
        let decision = check_capability(&policy, &request);
        assert!(!decision.granted);
        assert_eq!(decision.reason, "missing_capability:mutate_state");
        assert_eq!(decision.receipt.decision, "denied");
    }

    #[test]
    fn capability_receipts_record_granted_and_denied_actions() {
        let policy = policy();
        let report = build_kernel_capability_guard_report(
            DEFAULT_KERNEL_CAPABILITY_POLICY_PATH,
            &policy,
            default_capability_requests().as_slice(),
        );
        assert!(report.ok);
        assert!(report.decisions.iter().any(|decision| decision.granted));
        assert!(report.decisions.iter().any(|decision| !decision.granted));
        assert!(report
            .decisions
            .iter()
            .all(|decision| decision.receipt.receipt_type == "kernel_capability_check_receipt"));
    }
}
