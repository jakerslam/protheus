#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_TOKEN_LEN: usize = 128;
const MAX_SURFACE_ITEM_LEN: usize = 96;
const MAX_SURFACE_ITEMS: usize = 128;
const MAX_MANIFEST_HASH_LEN: usize = 128;

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

pub fn wrap_os_extension(
    descriptor: &OsExtensionDescriptor,
    action: &str,
    ts_ms: i64,
) -> OsExtensionEnvelope {
    let extension_id = sanitize_token(descriptor.extension_id.as_str(), MAX_TOKEN_LEN);
    let namespace = sanitize_token(descriptor.namespace.as_str(), MAX_TOKEN_LEN);
    let syscall_surface = normalize_surface(&descriptor.syscall_surface);
    let driver_surface = normalize_surface(&descriptor.driver_surface);
    let manifest_hash = normalize_manifest_hash(descriptor.capability_manifest_hash.as_str());
    let mut normalized_action = normalize_action(action);
    if manifest_hash.is_empty() || (syscall_surface.is_empty() && driver_surface.is_empty()) {
        normalized_action = "status".to_string();
    }
    OsExtensionEnvelope {
        source_layer: "layer3".to_string(),
        extension_id: if extension_id.is_empty() {
            "unknown_extension".to_string()
        } else {
            extension_id
        },
        namespace: if namespace.is_empty() {
            "protheus.unknown".to_string()
        } else {
            namespace
        },
        action: normalized_action,
        ts_ms: ts_ms.max(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_extension_action() {
        let d = OsExtensionDescriptor {
            extension_id: "os.netstack.v1".to_string(),
            namespace: "protheus.net".to_string(),
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
        assert_eq!(env.namespace, "protheus.unknown");
        assert_eq!(env.action, "activate");
        assert_eq!(env.ts_ms, 0);
    }

    #[test]
    fn fails_closed_to_status_when_manifest_or_surfaces_are_missing() {
        let d = OsExtensionDescriptor {
            extension_id: "os.netstack.v1".to_string(),
            namespace: "protheus.net".to_string(),
            capability_manifest_hash: " \u{200B} ".to_string(),
            syscall_surface: vec![],
            driver_surface: vec![],
        };
        let env = wrap_os_extension(&d, "start", 100);
        assert_eq!(env.action, "status");
        assert_eq!(env.ts_ms, 100);
    }
}
