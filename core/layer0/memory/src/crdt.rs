// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrdtCell {
    pub value: String,
    pub clock: u64,
    pub node: String,
}

pub type CrdtMap = BTreeMap<String, CrdtCell>;

const MAX_KEY_CHARS: usize = 192;
const MAX_VALUE_CHARS: usize = 2048;
const MAX_NODE_CHARS: usize = 96;

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect::<String>()
}

fn normalize_key(raw: &str) -> Option<String> {
    let cleaned = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_KEY_CHARS).collect::<String>())
}

fn normalize_node(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= MAX_NODE_CHARS {
            break;
        }
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            ch.to_ascii_lowercase()
        } else if ch.is_whitespace() {
            '-'
        } else {
            continue;
        };
        if normalized == '-' && out.ends_with('-') {
            continue;
        }
        out.push(normalized);
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "unknown-node".to_string()
    } else {
        trimmed
    }
}

fn normalize_value(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_VALUE_CHARS)
        .collect::<String>()
}

fn normalize_cell(cell: &CrdtCell) -> CrdtCell {
    CrdtCell {
        value: normalize_value(&cell.value),
        clock: cell.clock,
        node: normalize_node(&cell.node),
    }
}

fn should_take_incoming(existing: &CrdtCell, incoming: &CrdtCell) -> bool {
    incoming.clock > existing.clock
        || (incoming.clock == existing.clock
            && (incoming.node > existing.node
                || (incoming.node == existing.node && incoming.value > existing.value)))
}

pub fn merge(left: &CrdtMap, right: &CrdtMap) -> CrdtMap {
    let mut out: CrdtMap = BTreeMap::new();
    for (key, cell) in left {
        if let Some(normalized_key) = normalize_key(key) {
            out.insert(normalized_key, normalize_cell(cell));
        }
    }
    for (key, incoming_raw) in right {
        let Some(normalized_key) = normalize_key(key) else {
            continue;
        };
        let incoming = normalize_cell(incoming_raw);
        match out.get(&normalized_key) {
            None => {
                out.insert(normalized_key, incoming);
            }
            Some(existing) => {
                if should_take_incoming(existing, &incoming) {
                    out.insert(normalized_key, incoming);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_higher_clock() {
        let mut a = CrdtMap::new();
        a.insert(
            "topic".into(),
            CrdtCell {
                value: "alpha".into(),
                clock: 1,
                node: "n1".into(),
            },
        );
        let mut b = CrdtMap::new();
        b.insert(
            "topic".into(),
            CrdtCell {
                value: "beta".into(),
                clock: 2,
                node: "n2".into(),
            },
        );
        let merged = merge(&a, &b);
        assert_eq!(merged.get("topic").map(|v| v.value.as_str()), Some("beta"));
    }

    #[test]
    fn merge_sanitizes_key_and_node() {
        let mut left = CrdtMap::new();
        left.insert(
            " bad\u{200B}-key ".into(),
            CrdtCell {
                value: "payload\u{0000}".into(),
                clock: 1,
                node: "Node / Unsafe".into(),
            },
        );
        let merged = merge(&left, &CrdtMap::new());
        let cell = merged.get("bad-key").expect("normalized key exists");
        assert_eq!(cell.node, "node-unsafe");
        assert_eq!(cell.value, "payload");
    }

    #[test]
    fn merge_uses_value_tiebreak_when_clock_and_node_match() {
        let mut a = CrdtMap::new();
        a.insert(
            "topic".into(),
            CrdtCell {
                value: "alpha".into(),
                clock: 3,
                node: "n1".into(),
            },
        );
        let mut b = CrdtMap::new();
        b.insert(
            "topic".into(),
            CrdtCell {
                value: "beta".into(),
                clock: 3,
                node: "n1".into(),
            },
        );
        let merged_ab = merge(&a, &b);
        let merged_ba = merge(&b, &a);
        assert_eq!(merged_ab, merged_ba);
        assert_eq!(merged_ab.get("topic").map(|v| v.value.as_str()), Some("beta"));
    }
}
