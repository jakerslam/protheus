pub fn memory_matrix_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if action == "status" {
        matrix_status_payload(args)
    } else {
        build_matrix_payload(args)
    }
}

pub fn memory_auto_recall_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if action == "status" {
        auto_recall_status_payload(args)
    } else {
        auto_recall_filed_payload(args)
    }
}

pub fn dream_sequencer_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if action == "status" {
        dream_status_payload(args)
    } else {
        dream_run_payload(args)
    }
}

pub fn print_payload_and_exit_code(payload: Value) -> i32 {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>()
    }

    #[test]
    fn auto_recall_blocks_stale_matrix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let matrix_path = tmp.path().join("state/matrix.json");
        let policy_path = tmp.path().join("state/policy.json");
        fs::create_dir_all(matrix_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &matrix_path,
            serde_json::to_string_pretty(&json!({
                "generated_at": "2000-01-01T00:00:00Z",
                "tags": [{
                    "tag": "memory",
                    "nodes": [{
                        "node_id": "node.alpha",
                        "tags": ["memory"],
                        "priority_score": 10.0,
                        "recency_score": 1.0,
                        "dream_score": 0.0
                    }]
                }]
            }))
            .expect("encode"),
        )
        .expect("write matrix");
        fs::write(
            &policy_path,
            serde_json::to_string_pretty(&json!({
                "max_matrix_age_ms": 10
            }))
            .expect("encode"),
        )
        .expect("write policy");

        let root = tmp.path().to_string_lossy().to_string();
        let matrix = matrix_path.to_string_lossy().to_string();
        let policy = policy_path.to_string_lossy().to_string();
        let out = memory_auto_recall_payload(&map(&[
            ("root", root.as_str()),
            ("action", "filed"),
            ("node-id", "node.seed"),
            ("tags", "memory"),
            ("matrix-path", matrix.as_str()),
            ("policy-path", policy.as_str()),
        ]));
        assert_eq!(out["ok"], false);
        assert_eq!(out["reason"], "index_stale_blocked");
    }

    #[test]
    fn auto_recall_produces_sorted_matches_with_invariant_receipt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let matrix_path = tmp.path().join("state/matrix.json");
        fs::create_dir_all(matrix_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &matrix_path,
            serde_json::to_string_pretty(&json!({
                "generated_at": now_iso(),
                "tags": [{
                    "tag": "memory",
                    "nodes": [
                        {
                            "node_id": "node.low",
                            "tags": ["memory"],
                            "priority_score": 9.0,
                            "recency_score": 1.0,
                            "dream_score": 0.0
                        },
                        {
                            "node_id": "node.high",
                            "tags": ["memory"],
                            "priority_score": 20.0,
                            "recency_score": 1.0,
                            "dream_score": 0.0
                        }
                    ]
                }]
            }))
            .expect("encode"),
        )
        .expect("write matrix");

        let root = tmp.path().to_string_lossy().to_string();
        let matrix = matrix_path.to_string_lossy().to_string();
        let out = memory_auto_recall_payload(&map(&[
            ("root", root.as_str()),
            ("action", "filed"),
            ("node-id", "node.seed"),
            ("tags", "memory"),
            ("matrix-path", matrix.as_str()),
            ("allow-stale-matrix", "1"),
        ]));
        assert_eq!(out["ok"], true);
        assert_eq!(out["ranking_invariants"]["ok"], true);
        assert_eq!(out["matches"][0]["node_id"], "node.high");
    }
}

