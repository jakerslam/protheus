// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

const MAX_STORAGE_KEY_LEN: usize = 240;
const MAX_STORAGE_VALUE_LEN: usize = 16 * 1024;
const MAX_STORAGE_TIMESTAMP_MS: u64 = 9_999_999_999_999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRecord {
    pub key: String,
    pub value: String,
    pub version: u64,
    pub updated_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    InvalidKey,
    InvalidValue,
    InvalidTimestamp,
    NotFound,
    VersionConflict,
}

#[derive(Debug, Default)]
pub struct StorageEngine {
    records: BTreeMap<String, StorageRecord>,
}

impl StorageEngine {
    pub fn get(&self, key: &str) -> Result<StorageRecord, StorageError> {
        let normalized_key = sanitize_key(key).ok_or(StorageError::InvalidKey)?;
        self.records
            .get(&normalized_key)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    pub fn put(
        &mut self,
        key: &str,
        value: &str,
        expected_version: Option<u64>,
        now_ms: u64,
    ) -> Result<StorageRecord, StorageError> {
        let normalized_key = sanitize_key(key).ok_or(StorageError::InvalidKey)?;
        let normalized_value = sanitize_value(value).ok_or(StorageError::InvalidValue)?;
        if !valid_timestamp(now_ms) {
            return Err(StorageError::InvalidTimestamp);
        }

        let previous = self.records.get(&normalized_key).cloned();
        if let Some(expected) = expected_version {
            let current = previous.as_ref().map(|row| row.version).unwrap_or(0);
            if expected != current {
                return Err(StorageError::VersionConflict);
            }
        }

        let next_version = previous.map(|row| row.version.saturating_add(1)).unwrap_or(1);
        let record = StorageRecord {
            key: normalized_key.clone(),
            value: normalized_value,
            version: next_version,
            updated_ms: now_ms,
        };
        self.records.insert(normalized_key, record.clone());
        Ok(record)
    }

    pub fn delete(&mut self, key: &str) -> Result<StorageRecord, StorageError> {
        let normalized_key = sanitize_key(key).ok_or(StorageError::InvalidKey)?;
        self.records
            .remove(&normalized_key)
            .ok_or(StorageError::NotFound)
    }
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_key(raw_key: &str) -> Option<String> {
    let mut normalized: String = strip_invisible_unicode(raw_key)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    normalized = normalized.trim().to_string();
    if normalized.is_empty() || normalized.len() > MAX_STORAGE_KEY_LEN {
        return None;
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        return None;
    }
    Some(normalized)
}

fn sanitize_value(raw_value: &str) -> Option<String> {
    let mut normalized: String = strip_invisible_unicode(raw_value)
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .collect();
    if normalized.len() > MAX_STORAGE_VALUE_LEN {
        normalized.truncate(MAX_STORAGE_VALUE_LEN);
    }
    if normalized.trim().is_empty() {
        return None;
    }
    Some(normalized)
}

fn valid_timestamp(now_ms: u64) -> bool {
    now_ms > 0 && now_ms <= MAX_STORAGE_TIMESTAMP_MS
}

#[cfg(test)]
mod tests {
    use super::{StorageEngine, StorageError};

    #[test]
    fn put_and_get_are_versioned_and_deterministic() {
        let mut store = StorageEngine::default();
        let first = store
            .put(
                "local/state/kernel",
                "{\"ok\":true}",
                None,
                1_762_100_000_000,
            )
            .expect("first put");
        assert_eq!(first.version, 1);

        let second = store
            .put(
                "local/state/kernel",
                "{\"ok\":false}",
                Some(first.version),
                1_762_100_000_001,
            )
            .expect("second put");
        assert_eq!(second.version, 2);

        let loaded = store.get("local/state/kernel").expect("get latest");
        assert_eq!(loaded.value, "{\"ok\":false}");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn put_rejects_invalid_keys_and_version_conflicts() {
        let mut store = StorageEngine::default();
        assert_eq!(
            store.put("bad key with spaces", "x", None, 1),
            Err(StorageError::InvalidKey)
        );

        let inserted = store.put("local/state/a", "x", None, 1).expect("insert");
        assert_eq!(
            store.put("local/state/a", "y", Some(inserted.version + 2), 2),
            Err(StorageError::VersionConflict)
        );
    }

    #[test]
    fn delete_requires_existing_record() {
        let mut store = StorageEngine::default();
        assert_eq!(store.delete("missing"), Err(StorageError::NotFound));
        store
            .put("local/state/b", "payload", None, 1_762_100_000_010)
            .expect("insert");
        let deleted = store.delete("local/state/b").expect("delete");
        assert_eq!(deleted.key, "local/state/b");
        assert_eq!(store.get("local/state/b"), Err(StorageError::NotFound));
    }
}
