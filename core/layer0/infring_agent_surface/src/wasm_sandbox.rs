use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WasmSandboxPolicy {
    pub enabled: bool,
    pub max_fuel: u64,
    pub max_watchdog_ms: u64,
    pub allow_network: bool,
    pub allowed_modules: Vec<String>,
}

impl Default for WasmSandboxPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            max_fuel: 5_000_000,
            max_watchdog_ms: 3_000,
            allow_network: false,
            allowed_modules: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WasmPolicyDecision {
    Allowed,
    Blocked(String),
}

pub fn wasm_policy_from_value(raw: Option<&Value>) -> WasmSandboxPolicy {
    let mut out = WasmSandboxPolicy::default();
    let Some(value) = raw else {
        return out;
    };
    if let Some(flag) = value.get("enabled").and_then(Value::as_bool) {
        out.enabled = flag;
    }
    if let Some(flag) = value.get("allow_network").and_then(Value::as_bool) {
        out.allow_network = flag;
    }
    if let Some(limit) = value.get("max_fuel").and_then(Value::as_u64) {
        out.max_fuel = limit;
    }
    if let Some(limit) = value.get("max_watchdog_ms").and_then(Value::as_u64) {
        out.max_watchdog_ms = limit;
    }
    if let Some(items) = value.get("allowed_modules").and_then(Value::as_array) {
        out.allowed_modules = items
            .iter()
            .filter_map(Value::as_str)
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .collect();
    }
    out
}

pub fn evaluate_wasm_policy(
    policy: &WasmSandboxPolicy,
    requested_modules: &[String],
    requests_network: bool,
) -> WasmPolicyDecision {
    if !policy.enabled {
        return WasmPolicyDecision::Allowed;
    }
    if requests_network && !policy.allow_network {
        return WasmPolicyDecision::Blocked("wasm_network_denied".to_string());
    }
    if policy.max_fuel == 0 {
        return WasmPolicyDecision::Blocked("wasm_fuel_zero_denied".to_string());
    }
    if policy.max_watchdog_ms == 0 {
        return WasmPolicyDecision::Blocked("wasm_watchdog_zero_denied".to_string());
    }
    if policy.allowed_modules.is_empty() {
        return WasmPolicyDecision::Allowed;
    }
    for module in requested_modules {
        let normalized = module.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if !policy.allowed_modules.iter().any(|item| item == &normalized) {
            return WasmPolicyDecision::Blocked(format!(
                "wasm_module_denied:{normalized}"
            ));
        }
    }
    WasmPolicyDecision::Allowed
}

pub fn evaluate_wasm_execution_boundary(
    policy: &WasmSandboxPolicy,
    module_id: &str,
    fuel_used: u64,
    elapsed_ms: u64,
    requested_network: bool,
) -> WasmPolicyDecision {
    if !policy.enabled {
        return WasmPolicyDecision::Allowed;
    }
    let normalized = module_id.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return WasmPolicyDecision::Blocked("wasm_module_id_invalid".to_string());
    }
    if requested_network && !policy.allow_network {
        return WasmPolicyDecision::Blocked("wasm_network_denied".to_string());
    }
    if fuel_used > policy.max_fuel {
        return WasmPolicyDecision::Blocked("wasm_fuel_budget_exceeded".to_string());
    }
    if elapsed_ms > policy.max_watchdog_ms {
        return WasmPolicyDecision::Blocked("wasm_watchdog_budget_exceeded".to_string());
    }
    if !policy.allowed_modules.is_empty()
        && !policy.allowed_modules.iter().any(|allowed| allowed == &normalized)
    {
        return WasmPolicyDecision::Blocked(format!("wasm_module_denied:{normalized}"));
    }
    WasmPolicyDecision::Allowed
}

pub fn wasm_policy_snapshot(policy: &WasmSandboxPolicy) -> Value {
    json!({
        "enabled": policy.enabled,
        "max_fuel": policy.max_fuel,
        "max_watchdog_ms": policy.max_watchdog_ms,
        "allow_network": policy.allow_network,
        "allowed_modules": policy.allowed_modules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_policy_blocks_network_when_disabled() {
        let policy = WasmSandboxPolicy {
            enabled: true,
            max_fuel: 1_000,
            max_watchdog_ms: 500,
            allow_network: false,
            allowed_modules: vec![],
        };
        let decision = evaluate_wasm_policy(&policy, &[], true);
        assert_eq!(
            decision,
            WasmPolicyDecision::Blocked("wasm_network_denied".to_string())
        );
    }

    #[test]
    fn wasm_policy_blocks_unknown_module_when_allowlist_is_set() {
        let policy = WasmSandboxPolicy {
            enabled: true,
            max_fuel: 1_000,
            max_watchdog_ms: 500,
            allow_network: false,
            allowed_modules: vec!["safe.module".to_string()],
        };
        let decision = evaluate_wasm_policy(&policy, &["unsafe.module".to_string()], false);
        assert_eq!(
            decision,
            WasmPolicyDecision::Blocked("wasm_module_denied:unsafe.module".to_string())
        );
    }

    #[test]
    fn wasm_boundary_blocks_fuel_overrun() {
        let policy = WasmSandboxPolicy {
            enabled: true,
            max_fuel: 100,
            max_watchdog_ms: 500,
            allow_network: true,
            allowed_modules: vec!["safe.module".to_string()],
        };
        let decision =
            evaluate_wasm_execution_boundary(&policy, "safe.module", 101, 20, false);
        assert_eq!(
            decision,
            WasmPolicyDecision::Blocked("wasm_fuel_budget_exceeded".to_string())
        );
    }

    #[test]
    fn wasm_boundary_allows_valid_execution_sample() {
        let policy = WasmSandboxPolicy {
            enabled: true,
            max_fuel: 1000,
            max_watchdog_ms: 500,
            allow_network: false,
            allowed_modules: vec!["safe.module".to_string()],
        };
        let decision =
            evaluate_wasm_execution_boundary(&policy, "safe.module", 200, 30, false);
        assert_eq!(decision, WasmPolicyDecision::Allowed);
    }
}
