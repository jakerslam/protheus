use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Register {
    pub value: String,
    pub clock: u64,
    pub node: String,
}

pub type CrdtState = BTreeMap<String, Register>;

pub fn merge_state(left: &CrdtState, right: &CrdtState) -> CrdtState {
    let mut out = left.clone();
    for (key, incoming) in right {
        match out.get(key) {
            None => {
                out.insert(key.clone(), incoming.clone());
            }
            Some(existing) => {
                let take_incoming = incoming.clock > existing.clock
                    || (incoming.clock == existing.clock && incoming.node > existing.node);
                if take_incoming {
                    out.insert(key.clone(), incoming.clone());
                }
            }
        }
    }
    out
}

pub fn sample_report() -> serde_json::Value {
    let mut a = CrdtState::new();
    a.insert(
        "topic".into(),
        Register {
            value: "alpha".into(),
            clock: 3,
            node: "n1".into(),
        },
    );
    a.insert(
        "score".into(),
        Register {
            value: "7".into(),
            clock: 2,
            node: "n1".into(),
        },
    );

    let mut b = CrdtState::new();
    b.insert(
        "topic".into(),
        Register {
            value: "beta".into(),
            clock: 4,
            node: "n2".into(),
        },
    );
    b.insert(
        "flag".into(),
        Register {
            value: "on".into(),
            clock: 1,
            node: "n2".into(),
        },
    );

    let merged_ab = merge_state(&a, &b);
    let merged_ba = merge_state(&b, &a);

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-005",
        "convergent": merged_ab == merged_ba,
        "merged_keys": merged_ab.keys().cloned().collect::<Vec<String>>(),
        "state": merged_ab
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_is_convergent_for_sample() {
        let report = sample_report();
        assert_eq!(report.get("convergent").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn higher_clock_wins() {
        let mut l = CrdtState::new();
        l.insert(
            "k".into(),
            Register {
                value: "old".into(),
                clock: 1,
                node: "a".into(),
            },
        );
        let mut r = CrdtState::new();
        r.insert(
            "k".into(),
            Register {
                value: "new".into(),
                clock: 2,
                node: "b".into(),
            },
        );
        let merged = merge_state(&l, &r);
        assert_eq!(merged.get("k").map(|v| v.value.clone()), Some("new".into()));
    }
}
