// SPDX-License-Identifier: Apache-2.0
use guard_registry_contract_consumer::{
    effective_guard_ids, is_capability_allowed, normalize_capability_list,
    normalize_guard_registry, GuardRegistryContract, GuardRegistryEntry,
};

#[test]
fn capability_list_normalization_dedupes_and_strips_invisible_chars() {
    let normalized = normalize_capability_list(&[
        " NET.READ ".to_string(),
        "net.read".to_string(),
        "\u{200B}fs.write".to_string(),
        "".to_string(),
    ]);
    assert_eq!(
        normalized,
        vec!["net.read".to_string(), "fs.write".to_string()]
    );
}

#[test]
fn inactive_guards_do_not_authorize_capabilities() {
    let contract = normalize_guard_registry(vec![GuardRegistryEntry {
        guard_id: "guard.alpha".to_string(),
        active: false,
        capabilities: vec!["net.read".to_string()],
    }]);
    assert!(!is_capability_allowed(&contract, "guard.alpha", "net.read"));
    assert!(effective_guard_ids(&contract).is_empty());
}

#[test]
fn active_guard_authorizes_only_declared_capabilities() {
    let contract = normalize_guard_registry(vec![GuardRegistryEntry {
        guard_id: "guard.alpha".to_string(),
        active: true,
        capabilities: vec!["net.read".to_string()],
    }]);
    assert!(is_capability_allowed(&contract, "guard.alpha", "net.read"));
    assert!(!is_capability_allowed(&contract, "guard.alpha", "fs.write"));
    assert!(!is_capability_allowed(&contract, "unknown", "net.read"));
}

#[test]
fn effective_guard_ids_dedupe_even_for_non_normalized_contract_inputs() {
    let contract = GuardRegistryContract {
        entries: vec![
            GuardRegistryEntry {
                guard_id: " GUARD.ALPHA ".to_string(),
                active: true,
                capabilities: vec!["net.read".to_string()],
            },
            GuardRegistryEntry {
                guard_id: "guard.alpha".to_string(),
                active: true,
                capabilities: vec!["fs.write".to_string()],
            },
        ],
    };
    assert_eq!(
        effective_guard_ids(&contract),
        vec!["guard.alpha".to_string()]
    );
}

#[test]
fn empty_capability_requests_fail_closed() {
    let contract = normalize_guard_registry(vec![GuardRegistryEntry {
        guard_id: "guard.alpha".to_string(),
        active: true,
        capabilities: vec!["net.read".to_string()],
    }]);
    assert!(!is_capability_allowed(
        &contract,
        "guard.alpha",
        "\u{200B}\n"
    ));
}
