use crate::task_graph::{LifecycleStatus, Task};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    CreateTask,
    UpdateStatus,
    UpdateMetadata,
    AddDependency,
    AddBlocker,
    ResolveBlocker,
    ClaimLease,
    Heartbeat,
    Handoff,
    CrossScopeMove,
    DeleteTask,
    EscalateAutonomy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationRisk {
    Routine,
    HighRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyDecision {
    pub risk: MutationRisk,
    pub require_verity: bool,
    pub async_audit: bool,
    pub reason_code: String,
}

pub trait VerityGate {
    fn approve(
        &self,
        scope_id: &str,
        task: Option<&Task>,
        mutation_kind: MutationKind,
        payload: &Value,
    ) -> bool;
}

pub struct AllowAllVerityGate;

impl VerityGate for AllowAllVerityGate {
    fn approve(
        &self,
        _scope_id: &str,
        _task: Option<&Task>,
        _mutation_kind: MutationKind,
        _payload: &Value,
    ) -> bool {
        true
    }
}

pub fn evaluate_mutation(
    scope_id: &str,
    task: Option<&Task>,
    mutation_kind: MutationKind,
    payload: &Value,
) -> PolicyDecision {
    use MutationKind::*;
    let cross_scope = payload
        .get("scope_id")
        .and_then(Value::as_str)
        .map(|v| v != scope_id)
        .unwrap_or(false);
    let cancelling_status = payload
        .get("next_status")
        .and_then(Value::as_str)
        .map(|v| v.eq_ignore_ascii_case("cancelled"))
        .unwrap_or(false);
    let escalating = payload
        .get("autonomy_escalation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let risk = match mutation_kind {
        DeleteTask | CrossScopeMove | EscalateAutonomy => MutationRisk::HighRisk,
        UpdateStatus if cancelling_status => MutationRisk::HighRisk,
        _ if cross_scope || escalating => MutationRisk::HighRisk,
        _ => MutationRisk::Routine,
    };
    let reason_code = match risk {
        MutationRisk::HighRisk => "synchronous_verity_required",
        MutationRisk::Routine => "policy_allowed_async_audit",
    };
    let _ = task
        .map(|row| row.lifecycle_status)
        .filter(|v| *v == LifecycleStatus::Cancelled);
    PolicyDecision {
        risk,
        require_verity: matches!(risk, MutationRisk::HighRisk),
        async_audit: matches!(risk, MutationRisk::Routine),
        reason_code: reason_code.to_string(),
    }
}

pub fn enforce_mutation(
    scope_id: &str,
    task: Option<&Task>,
    mutation_kind: MutationKind,
    payload: &Value,
    verity: &dyn VerityGate,
) -> Result<PolicyDecision, String> {
    let decision = evaluate_mutation(scope_id, task, mutation_kind, payload);
    if decision.require_verity && !verity.approve(scope_id, task, mutation_kind, payload) {
        return Err("verity_denied_high_risk_mutation".to_string());
    }
    Ok(decision)
}
