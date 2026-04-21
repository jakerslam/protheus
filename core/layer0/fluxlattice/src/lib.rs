// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

const MAX_ID_LEN: usize = 96;
const MAX_TARGET_LEN: usize = 96;
const MAX_MORPHOLOGY_LEN: usize = 64;
const MAX_METADATA_KEY_LEN: usize = 64;
const MAX_METADATA_VALUE_LEN: usize = 256;
const MAX_METADATA_ENTRIES: usize = 128;

fn strip_invisible_unicode(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

fn normalize_token(input: &str, max_len: usize) -> String {
    strip_invisible_unicode(input)
        .trim()
        .chars()
        .take(max_len)
        .collect()
}

fn normalize_identifier(input: &str, max_len: usize, fallback: &str) -> String {
    let filtered: String = normalize_token(input, max_len)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
        .collect();
    if filtered.is_empty() {
        fallback.to_string()
    } else {
        filtered
    }
}

fn canonical_morphology(input: &str) -> String {
    let normalized = normalize_token(input, MAX_MORPHOLOGY_LEN).to_lowercase();
    match normalized.as_str() {
        "" => "coalesced".to_string(),
        "dynamic" | "dyn" => "dynamic".to_string(),
        "coalesce" | "coalesced" => "coalesced".to_string(),
        "static" | "stable" => "static".to_string(),
        _ => "coalesced".to_string(),
    }
}

fn upsert_meta(map: &mut BTreeMap<String, String>, key: &str, value: &str) {
    let normalized_key = normalize_token(key, MAX_METADATA_KEY_LEN).to_lowercase();
    if normalized_key.is_empty() {
        return;
    }
    if !map.contains_key(&normalized_key) && map.len() >= MAX_METADATA_ENTRIES {
        return;
    }
    let normalized_value = normalize_token(value, MAX_METADATA_VALUE_LEN);
    if normalized_value.is_empty() {
        return;
    }
    map.insert(normalized_key, normalized_value);
}

#[derive(Clone, Debug)]
pub struct FluxState {
    pub id: String,
    pub settled: bool,
    pub morphology: String,
    pub metadata: BTreeMap<String, String>,
}

impl FluxState {
    pub fn new(id: &str) -> Self {
        let normalized_id = normalize_identifier(id, MAX_ID_LEN, "unknown");
        Self {
            id: normalized_id,
            settled: false,
            morphology: "coalesced".to_string(),
            metadata: BTreeMap::new(),
        }
    }
}

pub fn init_state(id: &str) -> FluxState {
    let mut state = FluxState::new(id);
    upsert_meta(&mut state.metadata, "phase", "initialized");
    state
}

pub fn settle(mut state: FluxState, target: &str) -> FluxState {
    let normalized_target = normalize_identifier(target, MAX_TARGET_LEN, "");
    state.settled = true;
    if normalized_target.is_empty() {
        upsert_meta(&mut state.metadata, "target", "unspecified");
        upsert_meta(&mut state.metadata, "target_missing", "true");
    } else {
        upsert_meta(&mut state.metadata, "target", normalized_target.as_str());
    }
    upsert_meta(&mut state.metadata, "phase", "settled");
    state
}

pub fn morph(mut state: FluxState, morphology: &str) -> FluxState {
    state.morphology = canonical_morphology(morphology);
    upsert_meta(&mut state.metadata, "phase", "morphed");
    state
}

pub fn status_map(state: &FluxState) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("id".to_string(), state.id.clone());
    map.insert("settled".to_string(), state.settled.to_string());
    map.insert("morphology".to_string(), state.morphology.clone());
    for (k, v) in &state.metadata {
        let meta_key = normalize_token(k, MAX_METADATA_KEY_LEN);
        if meta_key.is_empty() {
            continue;
        }
        map.insert(
            format!("meta_{}", meta_key),
            normalize_token(v, MAX_METADATA_VALUE_LEN),
        );
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_settle_morph_roundtrip() {
        let state = init_state("test");
        let state = settle(state, "binary");
        let state = morph(state, "dyn");
        let status = status_map(&state);
        assert_eq!(status.get("settled").unwrap(), "true");
        assert_eq!(status.get("morphology").unwrap(), "dynamic");
    }

    #[test]
    fn normalizes_untrusted_inputs() {
        let state = init_state(" \u{200B}alpha\t");
        let state = settle(state, " \u{200C} ");
        let state = morph(state, "\u{FEFF}stable");
        let status = status_map(&state);
        assert_eq!(status.get("id").unwrap(), "alpha");
        assert_eq!(status.get("meta_target").unwrap(), "unspecified");
        assert_eq!(status.get("meta_target_missing").unwrap(), "true");
        assert_eq!(status.get("morphology").unwrap(), "static");
    }
}
