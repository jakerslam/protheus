use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i8)]
pub enum PermissionTrit {
    Deny = -1,
    Ask = 0,
    Allow = 1,
}

impl PermissionTrit {
    fn from_value(raw: &Value) -> Option<Self> {
        match raw {
            Value::Bool(true) => Some(Self::Allow),
            Value::Bool(false) => Some(Self::Deny),
            Value::Number(number) => match number.as_i64() {
                Some(1) => Some(Self::Allow),
                Some(0) => Some(Self::Ask),
                Some(-1) => Some(Self::Deny),
                _ => None,
            },
            Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
                "allow" | "true" | "1" => Some(Self::Allow),
                "ask" | "prompt" | "0" => Some(Self::Ask),
                "deny" | "false" | "-1" => Some(Self::Deny),
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionManifest {
    pub project_scope: Option<String>,
    pub grants: BTreeMap<String, PermissionTrit>,
}

impl Default for PermissionManifest {
    fn default() -> Self {
        Self {
            project_scope: None,
            grants: BTreeMap::new(),
        }
    }
}

pub fn permission_manifest_from_value(raw: Option<&Value>) -> PermissionManifest {
    let mut out = PermissionManifest::default();
    let Some(value) = raw else {
        return out;
    };
    out.project_scope = value
        .get("project_scope")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(grants) = value.get("grants").and_then(Value::as_object) else {
        return out;
    };
    for (key, value) in grants {
        let normalized = key.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if let Some(trit) = PermissionTrit::from_value(value) {
            out.grants.insert(normalized, trit);
        }
    }
    out
}

pub fn permission_for(manifest: &PermissionManifest, key: &str) -> PermissionTrit {
    let normalized = key.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return PermissionTrit::Ask;
    }
    if let Some(value) = manifest.grants.get(&normalized) {
        return *value;
    }
    if let Some(value) = manifest.grants.get("*") {
        return *value;
    }
    PermissionTrit::Ask
}

pub fn memory_read_allowed(manifest: &PermissionManifest) -> bool {
    permission_for(manifest, "memory.read") == PermissionTrit::Allow
}

pub fn memory_write_allowed(manifest: &PermissionManifest) -> bool {
    permission_for(manifest, "memory.write") == PermissionTrit::Allow
}

pub fn permission_manifest_snapshot(manifest: &PermissionManifest) -> Value {
    let grants = manifest
        .grants
        .iter()
        .map(|(key, value)| {
            let raw = match value {
                PermissionTrit::Deny => -1,
                PermissionTrit::Ask => 0,
                PermissionTrit::Allow => 1,
            };
            (key.clone(), json!(raw))
        })
        .collect::<serde_json::Map<String, Value>>();
    json!({
        "project_scope": manifest.project_scope,
        "grants": Value::Object(grants),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_manifest_supports_trit_values() {
        let manifest = permission_manifest_from_value(Some(&json!({
            "project_scope": "main",
            "grants": {
                "memory.read": 1,
                "memory.write": 0,
                "web.search": -1
            }
        })));
        assert!(memory_read_allowed(&manifest));
        assert!(!memory_write_allowed(&manifest));
        assert_eq!(
            permission_for(&manifest, "web.search"),
            PermissionTrit::Deny
        );
    }

    #[test]
    fn missing_grants_default_to_ask() {
        let manifest = PermissionManifest::default();
        assert_eq!(
            permission_for(&manifest, "memory.read"),
            PermissionTrit::Ask
        );
    }
}
