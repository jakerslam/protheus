use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct BlobStore {
    blobs: BTreeMap<String, Vec<u8>>,
}

impl BlobStore {
    pub fn put(&mut self, blob_id: impl Into<String>, bytes: Vec<u8>) {
        self.blobs.insert(blob_id.into(), bytes);
    }

    pub fn get(&self, blob_id: &str) -> Option<&[u8]> {
        self.blobs.get(blob_id).map(|row| row.as_slice())
    }

    pub fn contains(&self, blob_id: &str) -> bool {
        self.blobs.contains_key(blob_id)
    }
}
