mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_policy(root: &Path, full_reload: bool) {
        let policy = json!({
            "enabled": true,
            "eyes": {
                "push_attention_queue": false
            },
            "personas": {
                "ambient_stance": true,
                "auto_apply": true,
                "full_reload": full_reload,
                "cache_path": "local/state/personas/ambient_stance/cache.json",
                "latest_path": "local/state/personas/ambient_stance/latest.json",
                "receipts_path": "local/state/personas/ambient_stance/receipts.jsonl",
                "max_personas": 8,
                "max_patch_bytes": 8192
            }
        });
        let path = root.join("config").join("mech_suit_mode_policy.json");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        write_json(&path, &policy);
    }

    #[test]
    fn incremental_apply_merges_without_full_reload() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), false);

        let first = json!({
            "risk_mode": "strict",
            "temperature": 0.2
        });
        let first_payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&first).expect("encode"));
        let code_a = run(
            dir.path(),
            &[
                "apply".to_string(),
                "--persona=guardian".to_string(),
                format!("--stance-json-base64={first_payload}"),
            ],
        );
        assert_eq!(code_a, 0);

        let second = json!({
            "temperature": 0.4,
            "risk_mode": Value::Null
        });
        let second_payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&second).expect("encode"));
        let code_b = run(
            dir.path(),
            &[
                "apply".to_string(),
                "--persona=guardian".to_string(),
                format!("--stance-json-base64={second_payload}"),
            ],
        );
        assert_eq!(code_b, 0);

        let cache = read_json(
            &dir.path()
                .join("local")
                .join("state")
                .join("personas")
                .join("ambient_stance")
                .join("cache.json"),
        )
        .expect("cache exists");
        let stance = cache
            .pointer("/personas/guardian/stance")
            .and_then(Value::as_object)
            .expect("stance object");
        assert_eq!(stance.get("temperature"), Some(&json!(0.4)));
        assert!(stance.get("risk_mode").is_none());
    }

    #[test]
    fn full_reload_is_blocked_when_policy_disallows_it() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), false);
        let payload = base64::engine::general_purpose::STANDARD.encode("{}");
        let code = run(
            dir.path(),
            &[
                "apply".to_string(),
                "--persona=guardian".to_string(),
                format!("--stance-json-base64={payload}"),
                "--full-reload=1".to_string(),
            ],
        );
        assert_eq!(code, 2);
    }
}
