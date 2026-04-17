
#[cfg(test)]
mod abac_policy_plane_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn abac_denies_when_no_rule_matches() {
        let root = tempdir().expect("tempdir");
        let policy_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("abac_policy_plane.json");
        ensure_parent(&policy_path).expect("policy parent");
        write_json_atomic(
            &policy_path,
            &json!({
                "version": "v1",
                "kind": "abac_policy_plane",
                "default_effect": "deny",
                "rules": [],
                "flight_recorder": {
                    "immutable": true,
                    "hash_chain": true,
                    "redact_subject_fields": ["id"]
                }
            }),
        )
        .expect("write policy");

        let (out, code) = run_abac_policy_plane(
            root.path(),
            &[
                "evaluate".to_string(),
                "--action=write".to_string(),
                "--subject-role=observer".to_string(),
                "--subject-id=user-123".to_string(),
                "--object-classification=internal".to_string(),
                "--context-env=dev".to_string(),
            ],
        );
        assert_eq!(code, 1);
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("deny"));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("abac_policy_plane_evaluate")
        );

        let flight_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("security")
            .join("abac_flight_recorder.jsonl");
        let rows = read_jsonl(&flight_path);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]
                .get("subject")
                .and_then(|v| v.get("id"))
                .and_then(Value::as_str),
            Some("***")
        );
    }

    #[test]
    fn abac_allows_and_writes_hash_chain() {
        let root = tempdir().expect("tempdir");
        let policy_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("abac_policy_plane.json");
        ensure_parent(&policy_path).expect("policy parent");
        write_json_atomic(
            &policy_path,
            &json!({
                "version": "v1",
                "kind": "abac_policy_plane",
                "default_effect": "deny",
                "rules": [
                    {
                        "id": "allow_read_public",
                        "effect": "allow",
                        "action": ["read"],
                        "subject": {"role": ["operator"]},
                        "object": {"classification": ["public"]},
                        "context": {"env": ["prod"]}
                    }
                ],
                "flight_recorder": {
                    "immutable": true,
                    "hash_chain": true,
                    "redact_subject_fields": []
                }
            }),
        )
        .expect("write policy");

        let first = run_abac_policy_plane(
            root.path(),
            &[
                "evaluate".to_string(),
                "--action=read".to_string(),
                "--subject-role=operator".to_string(),
                "--subject-id=op-1".to_string(),
                "--object-classification=public".to_string(),
                "--context-env=prod".to_string(),
            ],
        );
        assert_eq!(first.1, 0);
        assert_eq!(
            first.0.get("decision").and_then(Value::as_str),
            Some("allow")
        );

        let second = run_abac_policy_plane(
            root.path(),
            &[
                "evaluate".to_string(),
                "--action=read".to_string(),
                "--subject-role=operator".to_string(),
                "--subject-id=op-2".to_string(),
                "--object-classification=public".to_string(),
                "--context-env=prod".to_string(),
            ],
        );
        assert_eq!(second.1, 0);

        let flight_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("security")
            .join("abac_flight_recorder.jsonl");
        let rows = read_jsonl(&flight_path);
        assert_eq!(rows.len(), 2);
        let first_hash = rows[0]
            .get("hash")
            .and_then(Value::as_str)
            .expect("first hash");
        assert_eq!(
            rows[1]
                .get("prev_hash")
                .and_then(Value::as_str)
                .unwrap_or(""),
            first_hash
        );
    }

    #[test]
    fn abac_redacts_subject_id_with_invisible_unicode_noise() {
        let root = tempdir().expect("tempdir");
        let policy_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("abac_policy_plane.json");
        ensure_parent(&policy_path).expect("policy parent");
        write_json_atomic(
            &policy_path,
            &json!({
                "version": "v1",
                "kind": "abac_policy_plane",
                "default_effect": "deny",
                "rules": [],
                "flight_recorder": {
                    "immutable": true,
                    "hash_chain": true,
                    "redact_subject_fields": ["id"]
                }
            }),
        )
        .expect("write policy");

        let (out, code) = run_abac_policy_plane(
            root.path(),
            &[
                "evaluate".to_string(),
                "--action=write".to_string(),
                "--subject-role=observer".to_string(),
                "--subject-id=user-\u{200B}42".to_string(),
                "--object-classification=internal".to_string(),
                "--context-env=dev".to_string(),
            ],
        );
        assert_eq!(code, 1);
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("deny"));

        let flight_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("security")
            .join("abac_flight_recorder.jsonl");
        let rows = read_jsonl(&flight_path);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]
                .get("subject")
                .and_then(|v| v.get("id"))
                .and_then(Value::as_str),
            Some("***")
        );
        assert!(rows[0].get("hash").and_then(Value::as_str).is_some());
    }
}
