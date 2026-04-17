// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, BTreeSet};

const MAX_GUARD_ID_LEN: usize = 96;
const MAX_CAPABILITY_LEN: usize = 96;
const MAX_CAPABILITY_COUNT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRegistryEntry {
    pub guard_id: String,
    pub active: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRegistryContract {
    pub entries: Vec<GuardRegistryEntry>,
}

fn sanitize_token(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'))
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_lowercase()
        .chars()
        .take(max_len)
        .collect()
}

pub fn normalize_capability_list(items: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        let token = sanitize_token(item, MAX_CAPABILITY_LEN);
        if token.is_empty() || !seen.insert(token.clone()) {
            continue;
        }
        out.push(token);
        if out.len() >= MAX_CAPABILITY_COUNT {
            break;
        }
    }
    out
}

pub fn normalize_guard_registry(entries: Vec<GuardRegistryEntry>) -> GuardRegistryContract {
    let mut merged: BTreeMap<String, GuardRegistryEntry> = BTreeMap::new();
    for entry in entries {
        let guard_id = sanitize_token(entry.guard_id.as_str(), MAX_GUARD_ID_LEN);
        if guard_id.is_empty() {
            continue;
        }
        let normalized_caps = normalize_capability_list(&entry.capabilities);
        let slot = merged.entry(guard_id.clone()).or_insert_with(|| GuardRegistryEntry {
            guard_id: guard_id.clone(),
            active: false,
            capabilities: Vec::new(),
        });
        slot.active = slot.active || entry.active;
        let combined = slot
            .capabilities
            .iter()
            .cloned()
            .chain(normalized_caps.into_iter())
            .collect::<Vec<_>>();
        slot.capabilities = normalize_capability_list(&combined);
    }
    GuardRegistryContract {
        entries: merged.into_values().collect(),
    }
}

pub fn effective_guard_ids(contract: &GuardRegistryContract) -> Vec<String> {
    contract
        .entries
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| sanitize_token(entry.guard_id.as_str(), MAX_GUARD_ID_LEN))
        .filter(|entry| !entry.is_empty())
        .collect()
}

pub fn is_capability_allowed(
    contract: &GuardRegistryContract,
    guard_id: &str,
    capability: &str,
) -> bool {
    let normalized_guard = sanitize_token(guard_id, MAX_GUARD_ID_LEN);
    let normalized_capability = sanitize_token(capability, MAX_CAPABILITY_LEN);
    if normalized_guard.is_empty() || normalized_capability.is_empty() {
        return false;
    }
    contract.entries.iter().any(|entry| {
        entry.active
            && sanitize_token(entry.guard_id.as_str(), MAX_GUARD_ID_LEN) == normalized_guard
            && entry
                .capabilities
                .iter()
                .any(|cap| sanitize_token(cap, MAX_CAPABILITY_LEN) == normalized_capability)
    })
}

pub fn add(left: u64, right: u64) -> u64 {
    left.saturating_add(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn guard_registry_normalization_merges_duplicates() {
        let contract = normalize_guard_registry(vec![
            GuardRegistryEntry {
                guard_id: "guard.alpha".to_string(),
                active: false,
                capabilities: vec!["net.read".to_string()],
            },
            GuardRegistryEntry {
                guard_id: " guard.alpha ".to_string(),
                active: true,
                capabilities: vec!["NET.READ".to_string(), "fs.write".to_string()],
            },
        ]);
        assert_eq!(contract.entries.len(), 1);
        assert_eq!(effective_guard_ids(&contract), vec!["guard.alpha".to_string()]);
        assert!(is_capability_allowed(&contract, "guard.alpha", "net.read"));
        assert!(is_capability_allowed(&contract, "guard.alpha", "fs.write"));
    }
}
