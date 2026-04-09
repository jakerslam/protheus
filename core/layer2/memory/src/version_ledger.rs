use crate::schemas::{MemoryMutationReplayRow, MemoryPurgeRecord, MemoryVersion};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct VersionLedger {
    versions: BTreeMap<String, MemoryVersion>,
    object_index: BTreeMap<String, Vec<String>>,
    purge_records: Vec<MemoryPurgeRecord>,
    purged_versions: BTreeMap<String, MemoryPurgeRecord>,
}

impl VersionLedger {
    pub fn append(&mut self, version: MemoryVersion) -> Result<(), String> {
        if self.versions.contains_key(&version.version_id) {
            return Err("version_already_exists".to_string());
        }
        self.object_index
            .entry(version.object_id.clone())
            .or_default()
            .push(version.version_id.clone());
        self.versions.insert(version.version_id.clone(), version);
        Ok(())
    }

    pub fn get(&self, version_id: &str) -> Option<&MemoryVersion> {
        self.versions.get(version_id)
    }

    pub fn versions_for_object(&self, object_id: &str) -> Vec<MemoryVersion> {
        self.object_index
            .get(object_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|version_id| self.versions.get(&version_id).cloned())
            .collect::<Vec<_>>()
    }

    pub fn latest_for_object(&self, object_id: &str) -> Option<MemoryVersion> {
        self.object_index
            .get(object_id)
            .and_then(|ids| ids.last())
            .and_then(|version_id| self.versions.get(version_id))
            .cloned()
    }

    pub fn all_versions(&self) -> Vec<MemoryVersion> {
        self.versions.values().cloned().collect::<Vec<_>>()
    }

    pub fn append_purge_record(&mut self, record: MemoryPurgeRecord) -> Result<(), String> {
        if !self
            .versions
            .contains_key(record.target_version_id.as_str())
        {
            return Err("purge_target_version_not_found".to_string());
        }
        if self
            .purged_versions
            .contains_key(record.target_version_id.as_str())
        {
            return Err("version_already_purged".to_string());
        }
        self.purged_versions
            .insert(record.target_version_id.clone(), record.clone());
        self.purge_records.push(record);
        Ok(())
    }

    pub fn purge_records(&self) -> &[MemoryPurgeRecord] {
        self.purge_records.as_slice()
    }

    pub fn is_purged(&self, version_id: &str) -> bool {
        self.purged_versions.contains_key(version_id)
    }

    pub fn active_versions_for_object(&self, object_id: &str) -> Vec<MemoryVersion> {
        self.versions_for_object(object_id)
            .into_iter()
            .filter(|row| !self.is_purged(row.version_id.as_str()))
            .collect::<Vec<_>>()
    }

    pub fn replay_rows(&self) -> Vec<MemoryMutationReplayRow> {
        let mut rows = self
            .versions
            .values()
            .map(|row| MemoryMutationReplayRow {
                object_id: row.object_id.clone(),
                version_id: row.version_id.clone(),
                parent_version_id: row.parent_version_id.clone(),
                scope: row.scope.clone(),
                trust_state: row.trust_state.clone(),
                receipt_id: row.receipt_id.clone(),
                timestamp_ms: row.timestamp_ms,
                payload_hash: row.payload_hash.clone(),
                lineage_refs: row.lineage_refs.clone(),
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            a.timestamp_ms
                .cmp(&b.timestamp_ms)
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.version_id.cmp(&b.version_id))
        });
        rows
    }
}
