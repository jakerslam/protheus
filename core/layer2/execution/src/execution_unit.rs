use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const EXECUTION_UNIT_REGISTRATION_RECEIPT_TYPE: &str = "layer2_execution_unit_registration_receipt";
const EXECUTION_UNIT_STATE_TRANSITION_RECEIPT_TYPE: &str =
    "layer2_execution_unit_state_transition_receipt";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionUnitState {
    Initialized,
    Running,
    Degraded,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnitBudget {
    #[serde(default)]
    pub cpu_millis: u64,
    #[serde(default)]
    pub memory_bytes: u64,
    #[serde(default)]
    pub runtime_millis: u64,
}

impl Default for ExecutionUnitBudget {
    fn default() -> Self {
        Self {
            cpu_millis: 1_000,
            memory_bytes: 64 * 1024 * 1024,
            runtime_millis: 60_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnitReceiptRef {
    pub receipt_id: String,
    pub receipt_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnit {
    pub id: String,
    pub state: ExecutionUnitState,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub budget: ExecutionUnitBudget,
    #[serde(default)]
    pub receipts: Vec<ExecutionUnitReceiptRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnitTransitionReceipt {
    pub receipt_id: String,
    pub receipt_type: String,
    pub sequence: u64,
    pub unit_id: String,
    #[serde(default)]
    pub previous_state: Option<ExecutionUnitState>,
    pub next_state: ExecutionUnitState,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExecutionUnitTracker {
    #[serde(default)]
    pub units: BTreeMap<String, ExecutionUnit>,
    #[serde(default)]
    pub receipts: Vec<ExecutionUnitTransitionReceipt>,
    #[serde(default)]
    pub next_sequence: u64,
}

impl ExecutionUnitTracker {
    pub fn register_unit(
        &mut self,
        unit_id: &str,
        dependencies: Vec<String>,
        budget: ExecutionUnitBudget,
    ) -> Result<ExecutionUnitTransitionReceipt, String> {
        let normalized_id = normalize_unit_id(unit_id);
        if normalized_id.is_empty() {
            return Err("execution_unit_id_missing".to_string());
        }
        if self.units.contains_key(normalized_id.as_str()) {
            return Err(format!("execution_unit_duplicate:{normalized_id}"));
        }
        let unit = ExecutionUnit {
            id: normalized_id.clone(),
            state: ExecutionUnitState::Initialized,
            dependencies: normalize_dependencies(&dependencies),
            budget,
            receipts: Vec::new(),
        };
        self.units.insert(normalized_id.clone(), unit);
        self.emit_transition_receipt(
            normalized_id.as_str(),
            None,
            ExecutionUnitState::Initialized,
            "unit_registered",
            EXECUTION_UNIT_REGISTRATION_RECEIPT_TYPE,
        )
    }

    pub fn transition_unit_state(
        &mut self,
        unit_id: &str,
        next_state: ExecutionUnitState,
        reason: &str,
    ) -> Result<ExecutionUnitTransitionReceipt, String> {
        let normalized_id = normalize_unit_id(unit_id);
        let current = self
            .units
            .get(normalized_id.as_str())
            .ok_or_else(|| format!("execution_unit_missing:{normalized_id}"))?
            .state
            .clone();
        if !is_valid_transition(&current, &next_state) {
            return Err(format!(
                "execution_unit_state_transition_invalid:{normalized_id}:{:?}->{:?}",
                current, next_state
            ));
        }
        if let Some(unit) = self.units.get_mut(normalized_id.as_str()) {
            unit.state = next_state.clone();
        }
        self.emit_transition_receipt(
            normalized_id.as_str(),
            Some(current),
            next_state,
            reason,
            EXECUTION_UNIT_STATE_TRANSITION_RECEIPT_TYPE,
        )
    }

    fn emit_transition_receipt(
        &mut self,
        unit_id: &str,
        previous_state: Option<ExecutionUnitState>,
        next_state: ExecutionUnitState,
        reason: &str,
        receipt_type: &str,
    ) -> Result<ExecutionUnitTransitionReceipt, String> {
        let normalized_reason = normalize_reason(reason);
        if normalized_reason.is_empty() {
            return Err("execution_unit_transition_reason_missing".to_string());
        }
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| "execution_unit_receipt_sequence_overflow".to_string())?;
        let receipt = ExecutionUnitTransitionReceipt {
            receipt_id: format!("execution_unit_receipt_{sequence:06}"),
            receipt_type: receipt_type.to_string(),
            sequence,
            unit_id: unit_id.to_string(),
            previous_state,
            next_state,
            reason: normalized_reason,
        };
        self.receipts.push(receipt.clone());
        if let Some(unit) = self.units.get_mut(unit_id) {
            unit.receipts.push(ExecutionUnitReceiptRef {
                receipt_id: receipt.receipt_id.clone(),
                receipt_type: receipt.receipt_type.clone(),
            });
        }
        Ok(receipt)
    }
}

fn normalize_unit_id(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-' || *ch == ':')
        .collect::<String>()
}

fn normalize_dependencies(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|dep| normalize_unit_id(dep))
        .filter(|dep| !dep.is_empty())
        .collect::<Vec<_>>()
}

fn normalize_reason(raw: &str) -> String {
    raw.chars().take(160).collect::<String>()
}

fn is_valid_transition(current: &ExecutionUnitState, next: &ExecutionUnitState) -> bool {
    match (current, next) {
        (ExecutionUnitState::Initialized, ExecutionUnitState::Running)
        | (ExecutionUnitState::Initialized, ExecutionUnitState::Degraded)
        | (ExecutionUnitState::Initialized, ExecutionUnitState::Terminated)
        | (ExecutionUnitState::Running, ExecutionUnitState::Degraded)
        | (ExecutionUnitState::Running, ExecutionUnitState::Terminated)
        | (ExecutionUnitState::Degraded, ExecutionUnitState::Running)
        | (ExecutionUnitState::Degraded, ExecutionUnitState::Terminated) => true,
        (left, right) => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_unit_tracker_register_and_transition_emit_receipts() {
        let mut tracker = ExecutionUnitTracker::default();
        let registration = tracker
            .register_unit(
                "lane_worker_alpha",
                vec!["layer2_execution".to_string(), "layer2_receipts".to_string()],
                ExecutionUnitBudget {
                    cpu_millis: 2_000,
                    memory_bytes: 8 * 1024 * 1024,
                    runtime_millis: 90_000,
                },
            )
            .expect("register");
        assert_eq!(
            registration.receipt_type,
            EXECUTION_UNIT_REGISTRATION_RECEIPT_TYPE
        );
        assert_eq!(registration.previous_state, None);
        assert_eq!(registration.next_state, ExecutionUnitState::Initialized);

        let transition = tracker
            .transition_unit_state(
                "lane_worker_alpha",
                ExecutionUnitState::Running,
                "scheduler_admitted",
            )
            .expect("transition");
        assert_eq!(
            transition.receipt_type,
            EXECUTION_UNIT_STATE_TRANSITION_RECEIPT_TYPE
        );
        assert_eq!(
            transition.previous_state,
            Some(ExecutionUnitState::Initialized)
        );
        assert_eq!(transition.next_state, ExecutionUnitState::Running);
        assert_eq!(tracker.receipts.len(), 2);
        let unit = tracker.units.get("lane_worker_alpha").expect("unit");
        assert_eq!(unit.state, ExecutionUnitState::Running);
        assert_eq!(unit.receipts.len(), 2);
        assert_eq!(unit.receipts[0].receipt_id, registration.receipt_id);
        assert_eq!(unit.receipts[1].receipt_id, transition.receipt_id);
    }

    #[test]
    fn execution_unit_tracker_rejects_transition_after_termination() {
        let mut tracker = ExecutionUnitTracker::default();
        tracker
            .register_unit("lane_worker_beta", Vec::new(), ExecutionUnitBudget::default())
            .expect("register");
        tracker
            .transition_unit_state(
                "lane_worker_beta",
                ExecutionUnitState::Terminated,
                "retired",
            )
            .expect("terminate");
        let err = tracker
            .transition_unit_state(
                "lane_worker_beta",
                ExecutionUnitState::Running,
                "invalid_restart",
            )
            .expect_err("terminated unit should not restart");
        assert!(err.contains("execution_unit_state_transition_invalid"));
    }
}
