use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterManifest {
    pub schema_version: String,
    pub adapter: String,
    pub exports: Vec<String>,
    pub fallback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestValidation {
    pub ok: bool,
    pub errors: Vec<String>,
    pub normalized: AdapterManifest,
}

const MAX_EXPORTS: usize = 64;
const MAX_TOKEN_CHARS: usize = 64;
const ALLOWED_FALLBACKS: [&str; 3] = ["ts_adapter_lane", "rust_adapter_lane", "native_bridge_lane"];

fn normalize_token(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(MAX_TOKEN_CHARS)
        .collect::<String>()
}

fn normalize_manifest(m: &AdapterManifest) -> AdapterManifest {
    let mut exports = BTreeSet::new();
    for export in m.exports.iter().take(MAX_EXPORTS) {
        let normalized = normalize_token(export);
        if !normalized.is_empty() {
            exports.insert(normalized);
        }
    }
    AdapterManifest {
        schema_version: normalize_token(&m.schema_version),
        adapter: normalize_token(&m.adapter),
        exports: exports.into_iter().collect::<Vec<String>>(),
        fallback: normalize_token(&m.fallback),
    }
}

pub fn validate_manifest_detailed(m: &AdapterManifest) -> ManifestValidation {
    let normalized = normalize_manifest(m);
    let mut errors = Vec::new();
    if normalized.schema_version.is_empty() {
        errors.push("schema_version_missing".to_string());
    }
    if !normalized
        .schema_version
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch == '.')
    {
        errors.push("schema_version_invalid".to_string());
    }
    if !normalized.adapter.starts_with("infring_") {
        errors.push("adapter_prefix_invalid".to_string());
    }
    if normalized.exports.is_empty() {
        errors.push("exports_missing".to_string());
    }
    if m.exports.len() > MAX_EXPORTS {
        errors.push("exports_count_exceeded".to_string());
    }
    if normalized.fallback.is_empty() {
        errors.push("fallback_missing".to_string());
    } else if !ALLOWED_FALLBACKS.contains(&normalized.fallback.as_str()) {
        errors.push("fallback_unsupported".to_string());
    }
    ManifestValidation {
        ok: errors.is_empty(),
        errors,
        normalized,
    }
}

pub fn validate_manifest(m: &AdapterManifest) -> bool {
    validate_manifest_detailed(m).ok
}

pub fn sample_report() -> serde_json::Value {
    let manifest = AdapterManifest {
        schema_version: " 1.0 ".into(),
        adapter: "infring_wasm_bridge ".into(),
        exports: vec![
            "query".into(),
            "merge".into(),
            "emit".into(),
            "memory_hotpath".into(),
            "execution_replay".into(),
            "security_vault".into(),
        ],
        fallback: "ts_adapter_lane".into(),
    };
    let validation = validate_manifest_detailed(&manifest);

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-009",
        "v6_lane": "V6-RUST50-006",
        "manifest": validation.normalized,
        "manifest_valid": validation.ok,
        "manifest_errors": validation.errors,
        "mobile_targets": ["ios", "android", "wasm32-unknown-unknown"],
        "background_safe": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_validation_works() {
        let m = AdapterManifest {
            schema_version: "1.0".into(),
            adapter: "infring_wasm_bridge".into(),
            exports: vec!["run".into()],
            fallback: "ts".into(),
        };
        assert!(validate_manifest(&m));
    }

    #[test]
    fn manifest_validation_normalizes_and_rejects_invalid_prefix() {
        let m = AdapterManifest {
            schema_version: " 1.1 ".into(),
            adapter: " wasm_bridge ".into(),
            exports: vec![" query ".into(), "query".into(), "merge".into()],
            fallback: " ts ".into(),
        };
        let out = validate_manifest_detailed(&m);
        assert_eq!(out.ok, false);
        assert!(out.errors.contains(&"adapter_prefix_invalid".to_string()));
        assert_eq!(out.normalized.exports, vec!["merge".to_string(), "query".to_string()]);
    }

    #[test]
    fn manifest_validation_rejects_unknown_fallback() {
        let m = AdapterManifest {
            schema_version: "1.0".into(),
            adapter: "infring_wasm_bridge".into(),
            exports: vec!["run".into()],
            fallback: "custom_lane".into(),
        };
        let out = validate_manifest_detailed(&m);
        assert!(!out.ok);
        assert!(out.errors.contains(&"fallback_unsupported".to_string()));
    }
}
