use crate::contracts::{
    DegradationState, ExecutionCorrelation, ExecutionState, OrchestrationPlan, PlanCandidate,
    PlanStatus, StepState, StepStatus, TypedOrchestrationRequest,
};

pub fn execution_state_for(
    request: &TypedOrchestrationRequest,
    plan: &PlanCandidate,
    needs_clarification: bool,
) -> ExecutionState {
    let correlation = correlation_for(request, plan);
    let step_statuses = observed_step_statuses(request);
    if !plan.degradation.is_empty() {
        let alternate_path = plan
            .steps
            .iter()
            .map(|row| row.target_contract.clone())
            .collect::<Vec<_>>();
        return ExecutionState {
            plan_status: if needs_clarification || plan.requires_clarification {
                PlanStatus::ClarificationRequired
            } else {
                PlanStatus::Degraded
            },
            steps: plan
                .steps
                .iter()
                .map(|row| StepState {
                    step_id: row.step_id.clone(),
                    status: step_statuses
                        .get(&row.step_id)
                        .cloned()
                        .or_else(|| observed_step_status(request))
                        .unwrap_or(StepStatus::Degraded),
                    blocked_on: row.blocked_on.clone(),
                })
                .collect(),
            recovery: None,
            degradation: Some(DegradationState {
                reasons: plan.degradation.clone(),
                alternate_path,
                note: "planner selected degraded alternate path".to_string(),
            }),
            correlation,
        };
    }

    let plan_status = if let Some(observed) = observed_plan_status(request) {
        observed
    } else if needs_clarification || plan.requires_clarification {
        PlanStatus::ClarificationRequired
    } else if !plan.blocked_on.is_empty() || plan.steps.is_empty() {
        PlanStatus::Blocked
    } else {
        PlanStatus::Ready
    };

    ExecutionState {
        plan_status,
        steps: plan
            .steps
            .iter()
            .map(|row| StepState {
                step_id: row.step_id.clone(),
                status: if let Some(observed) = step_statuses.get(&row.step_id).cloned() {
                    observed
                } else if let Some(observed) = observed_step_status(request) {
                    observed
                } else if row.blocked_on.is_empty() {
                    StepStatus::Ready
                } else {
                    StepStatus::Blocked
                },
                blocked_on: row.blocked_on.clone(),
            })
            .collect(),
        recovery: None,
        degradation: None,
        correlation,
    }
}

pub fn progress_message(plan: &OrchestrationPlan) -> String {
    let posture = format!("{:?}", plan.posture).to_lowercase();
    let status = format!("{:?}", plan.execution_state.plan_status).to_lowercase();
    format!(
        "orchestration posture={} status={} steps={} clarification={} confidence={:.2}",
        posture,
        status,
        plan.selected_plan.steps.len(),
        plan.needs_clarification,
        plan.selected_plan.confidence
    )
}

fn observed_plan_status(request: &TypedOrchestrationRequest) -> Option<PlanStatus> {
    match read_string_value(
        &request.payload,
        &[
            &["core_execution_status"],
            &["core_execution", "status"],
        ],
    )
        .and_then(|row| row.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "running" => Some(PlanStatus::Running),
        "completed" | "succeeded" => Some(PlanStatus::Completed),
        "failed" => Some(PlanStatus::Failed),
        _ => None,
    }
}

fn observed_step_status(request: &TypedOrchestrationRequest) -> Option<StepStatus> {
    match read_string_value(
        &request.payload,
        &[
            &["core_execution_status"],
            &["core_execution", "status"],
        ],
    )
        .and_then(|row| row.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "running" => Some(StepStatus::Running),
        "completed" | "succeeded" => Some(StepStatus::Succeeded),
        "failed" => Some(StepStatus::Failed),
        _ => None,
    }
}

fn observed_step_statuses(
    request: &TypedOrchestrationRequest,
) -> std::collections::BTreeMap<String, StepStatus> {
    let mut out = std::collections::BTreeMap::new();
    for path in [
        &["core_step_statuses"][..],
        &["core_execution", "step_statuses"][..],
    ] {
        if let Some(value) = read_path(&request.payload, path) {
            if let Some(map) = value.as_object() {
                for (key, raw_status) in map {
                    if let Some(status) = parse_step_status(raw_status.as_str().unwrap_or("")) {
                        out.insert(key.trim().to_string(), status);
                    }
                }
            }
        }
    }
    out
}

fn parse_step_status(raw: &str) -> Option<StepStatus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "pending" => Some(StepStatus::Pending),
        "ready" => Some(StepStatus::Ready),
        "blocked" => Some(StepStatus::Blocked),
        "degraded" => Some(StepStatus::Degraded),
        "skipped" => Some(StepStatus::Skipped),
        "running" => Some(StepStatus::Running),
        "completed" | "succeeded" => Some(StepStatus::Succeeded),
        "failed" => Some(StepStatus::Failed),
        _ => None,
    }
}

fn correlation_for(
    request: &TypedOrchestrationRequest,
    plan: &PlanCandidate,
) -> ExecutionCorrelation {
    ExecutionCorrelation {
        orchestration_trace_id: format!(
            "orch_{}_{}",
            request.session_id,
            plan.plan_id.replace(|ch: char| !ch.is_ascii_alphanumeric(), "")
        ),
        expected_core_contract_ids: plan
            .steps
            .iter()
            .map(|row| row.expected_contract_ref.clone())
            .collect(),
        observed_core_receipt_ids: read_string_list(
            &request.payload,
            &[
                &["observed_core_receipt_ids"][..],
                &["core_receipts"][..],
                &["core_execution", "receipt_ids"][..],
            ],
        ),
        observed_core_outcome_refs: read_string_list(
            &request.payload,
            &[
                &["observed_core_outcome_refs"][..],
                &["core_outcomes"][..],
                &["core_execution", "outcome_refs"][..],
            ],
        ),
    }
}

fn read_string_list(payload: &serde_json::Value, keys: &[&[&str]]) -> Vec<String> {
    let mut out = Vec::new();
    for key in keys {
        if let Some(rows) = read_path(payload, key).and_then(|row| row.as_array()) {
            for value in rows.iter().filter_map(|row| row.as_str()) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn read_string_value<'a>(
    payload: &'a serde_json::Value,
    paths: &[&[&str]],
) -> Option<&'a serde_json::Value> {
    paths.iter().find_map(|path| read_path(payload, path))
}

fn read_path<'a>(
    payload: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    let mut cursor = payload;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    Some(cursor)
}
