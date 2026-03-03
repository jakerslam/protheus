use serde_json::json;
use sha2::{Digest, Sha256};

pub fn receipt_digest(events: &[String]) -> String {
    let mut h = Sha256::new();
    for (idx, event) in events.iter().enumerate() {
        h.update(format!("{idx}:{event}|"));
    }
    format!("{:x}", h.finalize())
}

pub fn replay_report(events: &[String]) -> serde_json::Value {
    let digest = receipt_digest(events);
    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-003",
        "event_count": events.len(),
        "digest": digest,
        "deterministic": true,
        "replayable": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_for_same_input() {
        let events = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(receipt_digest(&events), receipt_digest(&events));
    }

    #[test]
    fn digest_changes_when_sequence_changes() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["b".to_string(), "a".to_string()];
        assert_ne!(receipt_digest(&a), receipt_digest(&b));
    }
}
