use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub priority: u32,
    pub eta_ms: u64,
}

pub fn schedule(mut tasks: Vec<Task>) -> Vec<Task> {
    tasks.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.eta_ms.cmp(&b.eta_ms))
            .then_with(|| a.id.cmp(&b.id))
    });
    tasks
}

pub fn rle_compress(input: &[u8]) -> Vec<(u8, u16)> {
    if input.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(u8, u16)> = Vec::new();
    let mut cur = input[0];
    let mut count: u16 = 1;
    for b in &input[1..] {
        if *b == cur && count < u16::MAX {
            count += 1;
            continue;
        }
        out.push((cur, count));
        cur = *b;
        count = 1;
    }
    out.push((cur, count));
    out
}

pub fn sqlite_hotpath_checksum(query: &str) -> String {
    let mut h = Sha256::new();
    h.update(query.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn sample_report() -> serde_json::Value {
    let scheduled = schedule(vec![
        Task { id: "compact".into(), priority: 3, eta_ms: 40 },
        Task { id: "recall".into(), priority: 5, eta_ms: 30 },
        Task { id: "index".into(), priority: 5, eta_ms: 20 },
        Task { id: "sync".into(), priority: 2, eta_ms: 80 },
    ]);

    let payload = b"aaaabbbbccccccdddddddddd";
    let compressed = rle_compress(payload);
    let compressed_bytes = compressed.len() * 3;
    let ratio = compressed_bytes as f64 / payload.len() as f64;

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-002",
        "scheduler_order": scheduled,
        "compression": {
            "input_bytes": payload.len(),
            "encoded_units": compressed.len(),
            "encoded_estimated_bytes": compressed_bytes,
            "ratio": ratio
        },
        "sqlite_checksum": sqlite_hotpath_checksum("SELECT node_id,summary FROM memory_index WHERE tag=? ORDER BY updated_at DESC LIMIT 50")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_prioritizes_higher_priority_then_eta() {
        let out = schedule(vec![
            Task { id: "a".into(), priority: 1, eta_ms: 10 },
            Task { id: "b".into(), priority: 3, eta_ms: 50 },
            Task { id: "c".into(), priority: 3, eta_ms: 5 },
        ]);
        assert_eq!(out[0].id, "c");
        assert_eq!(out[1].id, "b");
    }

    #[test]
    fn compression_reduces_runs() {
        let encoded = rle_compress(b"aaaaabbbb");
        assert_eq!(encoded.len(), 2);
        assert_eq!(encoded[0], (b'a', 5));
        assert_eq!(encoded[1], (b'b', 4));
    }
}
