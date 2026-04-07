use crate::schemas::MemoryObject;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct RecordStore {
    objects: BTreeMap<String, MemoryObject>,
    head_by_object: BTreeMap<String, String>,
    version_ids_by_object: BTreeMap<String, Vec<String>>,
}

impl RecordStore {
    pub fn upsert_object(&mut self, object: MemoryObject) {
        self.objects.insert(object.object_id.clone(), object);
    }

    pub fn get_object(&self, object_id: &str) -> Option<&MemoryObject> {
        self.objects.get(object_id)
    }

    pub fn get_object_mut(&mut self, object_id: &str) -> Option<&mut MemoryObject> {
        self.objects.get_mut(object_id)
    }

    pub fn all_objects(&self) -> Vec<MemoryObject> {
        self.objects.values().cloned().collect::<Vec<_>>()
    }

    pub fn register_version(&mut self, object_id: &str, version_id: &str) {
        let bucket = self
            .version_ids_by_object
            .entry(object_id.to_string())
            .or_default();
        if !bucket.iter().any(|existing| existing == version_id) {
            bucket.push(version_id.to_string());
        }
    }

    pub fn version_ids_for_object(&self, object_id: &str) -> Vec<String> {
        self.version_ids_by_object
            .get(object_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_head_version(&mut self, object_id: &str, version_id: &str) {
        self.head_by_object
            .insert(object_id.to_string(), version_id.to_string());
    }

    pub fn head_version_id(&self, object_id: &str) -> Option<String> {
        self.head_by_object.get(object_id).cloned()
    }
}
