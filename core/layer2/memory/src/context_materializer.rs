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

pub fn materialize_context(
    principal_id: &str,
    requested_scopes: &[MemoryScope],
    redaction_policy: OwnerExportRedactionPolicy,
    source_versions: &[MemoryVersion],
) -> ContextMaterialization {
    let mut entries = Vec::<MaterializedMemoryEntry>::new();
    let mut manifest_entries = Vec::<ContextManifestEntryRef>::new();

    for version in source_versions {
        if !requested_scopes.is_empty() && !requested_scopes.iter().any(|row| row == &version.scope)
        {
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
        context_manifest_id: format!(
            "context_{}",
            &deterministic_hash(&(principal_id.to_string(), &manifest_entries, now_ms()))[..24]
        ),
        principal_id: principal_id.to_string(),
        requested_scopes: requested_scopes.to_vec(),
        redaction_policy,
        entries: manifest_entries,
        lineage_refs: entries
            .iter()
            .flat_map(|row| row.lineage_refs.clone())
            .collect::<Vec<_>>(),
        timestamp_ms: now_ms(),
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
