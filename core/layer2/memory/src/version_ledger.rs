use crate::schemas::MemoryVersion;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct VersionLedger {
    versions: BTreeMap<String, MemoryVersion>,
    object_index: BTreeMap<String, Vec<String>>,
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
}
