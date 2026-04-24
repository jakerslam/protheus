#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TOKEN_LEN: usize = 128;
const MAX_SURFACE_ITEM_LEN: usize = 96;
const MAX_SURFACE_ITEMS: usize = 128;
const MAX_MANIFEST_HASH_LEN: usize = 128;
const MAX_TS_MS: i64 = 9_999_999_999_999;
const MAX_EXECUTION_UNITS: usize = 1024;

fn sanitize_token(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(max_len)
        .collect()
}

fn sanitize_identifier(input: &str, max_len: usize, fallback: &str) -> String {
    let filtered: String = sanitize_token(input, max_len)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .collect();
    if filtered.is_empty() {
        fallback.to_string()
    } else {
        filtered
    }
}

fn normalize_surface(entries: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for entry in entries {
        let token = sanitize_token(entry, MAX_SURFACE_ITEM_LEN);
        if token.is_empty() || !seen.insert(token.clone()) {
            continue;
        }
        out.push(token);
        if out.len() >= MAX_SURFACE_ITEMS {
            break;
        }
    }
    out
}

fn normalize_action(action: &str) -> String {
    let normalized = sanitize_token(action, MAX_TOKEN_LEN).to_lowercase();
    match normalized.as_str() {
        "start" => "activate".to_string(),
        "enable" => "activate".to_string(),
        "stop" => "deactivate".to_string(),
        "disable" => "deactivate".to_string(),
        "" => "status".to_string(),
        _ => normalized,
    }
}

fn normalize_manifest_hash(hash: &str) -> String {
    sanitize_token(hash, MAX_MANIFEST_HASH_LEN)
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(MAX_MANIFEST_HASH_LEN)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsExtensionDescriptor {
    pub extension_id: String,
    pub namespace: String,
    pub capability_manifest_hash: String,
    pub syscall_surface: Vec<String>,
    pub driver_surface: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsExtensionEnvelope {
    pub source_layer: String,
    pub extension_id: String,
    pub namespace: String,
    pub action: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionUnitState {
    Init,
    Running,
    Degraded,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnitBudget {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub queue_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnit {
    pub id: String,
    pub state: ExecutionUnitState,
    pub dependencies: Vec<String>,
    pub budget: ExecutionUnitBudget,
    pub receipts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionUnitError {
    InvalidId,
    InvalidDependency,
    EmptyBudget,
    EmptyReceipt,
    UnitAlreadyExists,
    UnitNotFound,
    InvalidTransition,
    CapacityExceeded,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionUnitTracker {
    units: BTreeMap<String, ExecutionUnit>,
}

impl ExecutionUnitBudget {
    pub fn bounded(
        cpu_millis: u64,
        memory_bytes: u64,
        storage_bytes: u64,
        queue_depth: u32,
    ) -> Self {
        Self {
            cpu_millis,
            memory_bytes,
            storage_bytes,
            queue_depth,
        }
    }

    pub fn is_bounded(&self) -> bool {
        self.cpu_millis > 0
            && self.memory_bytes > 0
            && self.storage_bytes > 0
            && self.queue_depth > 0
    }
}

impl ExecutionUnit {
    pub fn new(
        id: &str,
        dependencies: &[String],
        budget: ExecutionUnitBudget,
        registration_receipt: &str,
    ) -> Result<Self, ExecutionUnitError> {
        let id = sanitize_identifier(id, MAX_TOKEN_LEN, "");
        if id.is_empty() {
            return Err(ExecutionUnitError::InvalidId);
        }
        let dependencies = normalize_dependencies(dependencies)?;
        if !budget.is_bounded() {
            return Err(ExecutionUnitError::EmptyBudget);
        }
        let registration_receipt = normalize_receipt(registration_receipt)?;
        Ok(Self {
            id,
            state: ExecutionUnitState::Init,
            dependencies,
            budget,
            receipts: vec![registration_receipt],
        })
    }
}

impl ExecutionUnitTracker {
    pub fn register(&mut self, unit: ExecutionUnit) -> Result<(), ExecutionUnitError> {
        if self.units.len() >= MAX_EXECUTION_UNITS {
            return Err(ExecutionUnitError::CapacityExceeded);
        }
        if self.units.contains_key(unit.id.as_str()) {
            return Err(ExecutionUnitError::UnitAlreadyExists);
        }
        self.units.insert(unit.id.clone(), unit);
        Ok(())
    }

    pub fn transition(
        &mut self,
        id: &str,
        next_state: ExecutionUnitState,
        receipt: &str,
    ) -> Result<(), ExecutionUnitError> {
        let id = sanitize_identifier(id, MAX_TOKEN_LEN, "");
        let receipt = normalize_receipt(receipt)?;
        let unit = self
            .units
            .get_mut(id.as_str())
            .ok_or(ExecutionUnitError::UnitNotFound)?;
        if !is_allowed_transition(&unit.state, &next_state) {
            return Err(ExecutionUnitError::InvalidTransition);
        }
        unit.state = next_state;
        unit.receipts.push(receipt);
        Ok(())
    }

    pub fn unit(&self, id: &str) -> Option<&ExecutionUnit> {
        let id = sanitize_identifier(id, MAX_TOKEN_LEN, "");
        self.units.get(id.as_str())
    }

    pub fn units(&self) -> Vec<&ExecutionUnit> {
        self.units.values().collect()
    }
}

fn normalize_dependencies(dependencies: &[String]) -> Result<Vec<String>, ExecutionUnitError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for dependency in dependencies {
        let dependency = sanitize_token(dependency, MAX_TOKEN_LEN);
        if dependency.is_empty() {
            continue;
        }
        if !dependency.starts_with("core/layer2/") && !dependency.starts_with("core/layer3/") {
            return Err(ExecutionUnitError::InvalidDependency);
        }
        if seen.insert(dependency.clone()) {
            out.push(dependency);
        }
    }
    if out.is_empty() {
        return Err(ExecutionUnitError::InvalidDependency);
    }
    Ok(out)
}

fn normalize_receipt(receipt: &str) -> Result<String, ExecutionUnitError> {
    let receipt = sanitize_token(receipt, MAX_TOKEN_LEN);
    if receipt.is_empty() {
        return Err(ExecutionUnitError::EmptyReceipt);
    }
    Ok(receipt)
}

fn is_allowed_transition(current: &ExecutionUnitState, next: &ExecutionUnitState) -> bool {
    matches!(
        (current, next),
        (ExecutionUnitState::Init, ExecutionUnitState::Running)
            | (ExecutionUnitState::Init, ExecutionUnitState::Terminated)
            | (ExecutionUnitState::Running, ExecutionUnitState::Degraded)
            | (ExecutionUnitState::Running, ExecutionUnitState::Terminated)
            | (ExecutionUnitState::Degraded, ExecutionUnitState::Running)
            | (ExecutionUnitState::Degraded, ExecutionUnitState::Terminated)
    )
}

pub fn wrap_os_extension(
    descriptor: &OsExtensionDescriptor,
    action: &str,
    ts_ms: i64,
) -> OsExtensionEnvelope {
    let extension_id = sanitize_identifier(
        descriptor.extension_id.as_str(),
        MAX_TOKEN_LEN,
        "unknown_extension",
    );
    let namespace = sanitize_identifier(
        descriptor.namespace.as_str(),
        MAX_TOKEN_LEN,
        "infring.unknown",
    );
    let syscall_surface = normalize_surface(&descriptor.syscall_surface);
    let driver_surface = normalize_surface(&descriptor.driver_surface);
    let manifest_hash = normalize_manifest_hash(descriptor.capability_manifest_hash.as_str());
    let mut normalized_action = normalize_action(action);
    if manifest_hash.is_empty() || (syscall_surface.is_empty() && driver_surface.is_empty()) {
        normalized_action = "status".to_string();
    }
    OsExtensionEnvelope {
        source_layer: "layer3".to_string(),
        extension_id,
        namespace,
        action: normalized_action,
        ts_ms: ts_ms.clamp(0, MAX_TS_MS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_extension_action() {
        let d = OsExtensionDescriptor {
            extension_id: "os.netstack.v1".to_string(),
            namespace: "infring.net".to_string(),
            capability_manifest_hash: "abc123".to_string(),
            syscall_surface: vec!["net.open".to_string()],
            driver_surface: vec!["driver.nic".to_string()],
        };
        let env = wrap_os_extension(&d, "activate", 1_762_000_000_000);
        assert_eq!(env.source_layer, "layer3");
        assert_eq!(env.action, "activate");
    }

    #[test]
    fn normalizes_untrusted_envelope_inputs() {
        let d = OsExtensionDescriptor {
            extension_id: " \u{200B} \n".to_string(),
            namespace: " \u{200C} ".to_string(),
            capability_manifest_hash: "abc123".to_string(),
            syscall_surface: vec!["net.open".to_string(), "net.open".to_string()],
            driver_surface: vec!["driver.nic".to_string()],
        };
        let env = wrap_os_extension(&d, "start", -42);
        assert_eq!(env.extension_id, "unknown_extension");
        assert_eq!(env.namespace, "infring.unknown");
        assert_eq!(env.action, "activate");
        assert_eq!(env.ts_ms, 0);
    }

    #[test]
    fn fails_closed_to_status_when_manifest_or_surfaces_are_missing() {
        let d = OsExtensionDescriptor {
            extension_id: "os.netstack.v1".to_string(),
            namespace: "infring.net".to_string(),
            capability_manifest_hash: " \u{200B} ".to_string(),
            syscall_surface: vec![],
            driver_surface: vec![],
        };
        let env = wrap_os_extension(&d, "start", 100);
        assert_eq!(env.action, "status");
        assert_eq!(env.ts_ms, 100);
    }

    fn sample_budget() -> ExecutionUnitBudget {
        ExecutionUnitBudget::bounded(1_000, 64 * 1024 * 1024, 16 * 1024 * 1024, 8)
    }

    fn sample_dependencies() -> Vec<String> {
        vec![
            "core/layer2/execution".to_string(),
            "core/layer2/nexus".to_string(),
        ]
    }

    #[test]
    fn tracks_execution_unit_lifecycle_with_receipts() {
        let unit = ExecutionUnit::new(
            "service.chat-shell",
            &sample_dependencies(),
            sample_budget(),
            "receipt.registered",
        )
        .expect("execution unit should be valid");
        let mut tracker = ExecutionUnitTracker::default();
        tracker.register(unit).expect("register unit");
        tracker
            .transition(
                "service.chat-shell",
                ExecutionUnitState::Running,
                "receipt.running",
            )
            .expect("running transition");
        tracker
            .transition(
                "service.chat-shell",
                ExecutionUnitState::Degraded,
                "receipt.degraded",
            )
            .expect("degraded transition");
        tracker
            .transition(
                "service.chat-shell",
                ExecutionUnitState::Terminated,
                "receipt.terminated",
            )
            .expect("terminated transition");
        let unit = tracker.unit("service.chat-shell").expect("tracked unit");
        assert_eq!(unit.state, ExecutionUnitState::Terminated);
        assert_eq!(unit.receipts.len(), 4);
    }

    #[test]
    fn rejects_unbounded_or_cross_boundary_execution_units() {
        let bad_budget = ExecutionUnitBudget::bounded(0, 64, 64, 1);
        let err = ExecutionUnit::new(
            "service.bad",
            &sample_dependencies(),
            bad_budget,
            "receipt.registered",
        )
        .expect_err("zero budget is invalid");
        assert_eq!(err, ExecutionUnitError::EmptyBudget);

        let bad_deps = vec!["adapters/runtime/provider".to_string()];
        let err = ExecutionUnit::new("service.bad", &bad_deps, sample_budget(), "receipt")
            .expect_err("gateway dependency cannot be a layer3 execution dependency");
        assert_eq!(err, ExecutionUnitError::InvalidDependency);
    }

    #[test]
    fn rejects_invalid_execution_unit_state_transitions() {
        let unit = ExecutionUnit::new(
            "service.once",
            &sample_dependencies(),
            sample_budget(),
            "receipt.registered",
        )
        .expect("execution unit should be valid");
        let mut tracker = ExecutionUnitTracker::default();
        tracker.register(unit).expect("register unit");
        tracker
            .transition(
                "service.once",
                ExecutionUnitState::Terminated,
                "receipt.terminated",
            )
            .expect("init can terminate");
        let err = tracker
            .transition(
                "service.once",
                ExecutionUnitState::Running,
                "receipt.invalid_restart",
            )
            .expect_err("terminated units cannot restart inside layer3");
        assert_eq!(err, ExecutionUnitError::InvalidTransition);
    }
}
