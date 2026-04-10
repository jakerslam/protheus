use crate::schemas::{
    ContextManifest, ContextManifestEntryRef, MemoryScope, MemoryVersion,
    OwnerExportRedactionPolicy,
};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterializedMemoryEntry {
    pub object_id: String,
    pub version_id: String,
    pub scope: MemoryScope,
    pub payload: Value,
    pub redacted: bool,
    pub lineage_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextMaterialization {
    pub manifest: ContextManifest,
    pub entries: Vec<MaterializedMemoryEntry>,
}

fn allows_scope(requested_scopes: &[MemoryScope], scope: &MemoryScope) -> bool {
    requested_scopes.is_empty() || requested_scopes.iter().any(|row| row == scope)
}

fn should_materialize_version(
    requested_scopes: &[MemoryScope],
    include_ephemeral: bool,
    version: &MemoryVersion,
) -> bool {
    if !allows_scope(requested_scopes, &version.scope) {
        return false;
    }
    !matches!(version.scope, MemoryScope::Ephemeral) || include_ephemeral
}

fn context_manifest_id(
    principal_id: &str,
    manifest_entries: &[ContextManifestEntryRef],
    timestamp_ms: u64,
) -> String {
    format!(
        "context_{}",
        &deterministic_hash(&(principal_id.to_string(), manifest_entries, timestamp_ms))[..24]
    )
}

pub fn materialize_context(
    principal_id: &str,
    requested_scopes: &[MemoryScope],
    redaction_policy: OwnerExportRedactionPolicy,
    source_versions: &[MemoryVersion],
) -> ContextMaterialization {
    let mut entries = Vec::<MaterializedMemoryEntry>::new();
    let mut manifest_entries = Vec::<ContextManifestEntryRef>::new();
    let explicit_ephemeral = requested_scopes
        .iter()
        .any(|scope| matches!(scope, MemoryScope::Ephemeral));
    let timestamp_ms = now_ms();

    for version in source_versions {
        if !should_materialize_version(requested_scopes, explicit_ephemeral, version) {
            continue;
        }
        let (payload, redacted) =
            render_scope_payload(&version.scope, &version.payload, &redaction_policy);
        entries.push(MaterializedMemoryEntry {
            object_id: version.object_id.clone(),
            version_id: version.version_id.clone(),
            scope: version.scope.clone(),
            payload,
            redacted,
            lineage_refs: version.lineage_refs.clone(),
        });
        manifest_entries.push(ContextManifestEntryRef {
            object_id: version.object_id.clone(),
            version_id: version.version_id.clone(),
            scope: version.scope.clone(),
            trust_state: version.trust_state.clone(),
            redacted,
        });
    }

    let manifest = ContextManifest {
        context_manifest_id: context_manifest_id(principal_id, &manifest_entries, timestamp_ms),
        principal_id: principal_id.to_string(),
        requested_scopes: requested_scopes.to_vec(),
        redaction_policy,
        entries: manifest_entries,
        lineage_refs: entries
            .iter()
            .flat_map(|row| row.lineage_refs.clone())
            .collect::<Vec<_>>(),
        timestamp_ms,
    };
    ContextMaterialization { manifest, entries }
}

fn render_scope_payload(
    scope: &MemoryScope,
    payload: &Value,
    redaction_policy: &OwnerExportRedactionPolicy,
) -> (Value, bool) {
    if !matches!(scope, MemoryScope::Owner) {
        return (payload.clone(), false);
    }
    match redaction_policy {
        OwnerExportRedactionPolicy::AllowFull => (payload.clone(), false),
        OwnerExportRedactionPolicy::AllowRedacted => {
            let summary = summarize_payload(payload);
            (json!({ "redacted": true, "summary": summary }), true)
        }
        OwnerExportRedactionPolicy::SummarizeOnly => {
            let summary = summarize_payload(payload);
            (json!({ "summary_only": summary }), true)
        }
        OwnerExportRedactionPolicy::Deny => (json!({ "denied": true }), true),
    }
}

fn summarize_payload(payload: &Value) -> String {
    let raw = match payload {
        Value::String(row) => row.clone(),
        _ => serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()),
    };
    raw.split_whitespace()
        .take(24)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::TrustState;
    use serde_json::json;

    fn sample_version(scope: MemoryScope, object_id: &str, version_id: &str) -> MemoryVersion {
        MemoryVersion {
            version_id: version_id.to_string(),
            object_id: object_id.to_string(),
            scope,
            parent_version_id: None,
            lineage_refs: vec!["lineage:test".to_string()],
            receipt_id: "receipt_test".to_string(),
            trust_state: TrustState::Validated,
            payload: json!({"value":"ok"}),
            payload_hash: "hash".to_string(),
            timestamp_ms: 1,
            proposed_by: "agent:test".to_string(),
        }
    }

    #[test]
    fn default_materialization_excludes_ephemeral_scope() {
        let versions = vec![
            sample_version(MemoryScope::Ephemeral, "obj_e", "v1"),
            sample_version(MemoryScope::Agent("alpha".to_string()), "obj_a", "v2"),
        ];
        let materialized = materialize_context(
            "agent:alpha",
            &[],
            OwnerExportRedactionPolicy::AllowFull,
            versions.as_slice(),
        );
        assert_eq!(materialized.entries.len(), 1);
        assert_eq!(materialized.entries[0].object_id, "obj_a");
    }

    #[test]
    fn explicit_ephemeral_scope_request_includes_ephemeral() {
        let versions = vec![
            sample_version(MemoryScope::Ephemeral, "obj_e", "v1"),
            sample_version(MemoryScope::Agent("alpha".to_string()), "obj_a", "v2"),
        ];
        let materialized = materialize_context(
            "agent:alpha",
            &[
                MemoryScope::Ephemeral,
                MemoryScope::Agent("alpha".to_string()),
            ],
            OwnerExportRedactionPolicy::AllowFull,
            versions.as_slice(),
        );
        assert_eq!(materialized.entries.len(), 2);
        assert!(materialized
            .entries
            .iter()
            .any(|entry| entry.scope == MemoryScope::Ephemeral));
    }
}
