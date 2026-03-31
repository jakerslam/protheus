#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bootstraps_strict_production_policy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let payload = status_payload(temp.path(), &["status".to_string()]);
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("verity_plane_status")
        );
        assert_eq!(
            payload
                .get("policy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("mode"))
                .and_then(Value::as_str),
            Some("production")
        );
        assert_eq!(
            payload
                .get("ultimate_vector")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("id"))
                .and_then(Value::as_str),
            Some("ULTIMATE_VECTOR")
        );
        assert!(
            payload
                .get("receipt_hash")
                .and_then(Value::as_str)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn record_event_links_parent_receipt_hash() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = run_record_event(
            temp.path(),
            &[
                "record-event".to_string(),
                "--operation=first".to_string(),
                "--fidelity=0.99".to_string(),
                "--vector=0.98".to_string(),
            ],
        );
        let second = run_record_event(
            temp.path(),
            &[
                "record-event".to_string(),
                "--operation=second".to_string(),
                "--fidelity=0.98".to_string(),
                "--vector=0.97".to_string(),
            ],
        );
        let first_hash = first
            .get("receipt")
            .and_then(Value::as_object)
            .and_then(|receipt| receipt.get("receipt_hash"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let second_parent = second
            .get("receipt")
            .and_then(Value::as_object)
            .and_then(|receipt| receipt.get("parent_verity_hash"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert_eq!(first_hash, second_parent);
    }

    #[test]
    fn vector_check_emits_event_and_receipt() {
        let temp = tempfile::tempdir().expect("tempdir");
        let payload = run_vector_check(
            temp.path(),
            &["vector-check".to_string(), "--target=1.0".to_string()],
        );
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("verity_vector_check")
        );
        assert!(payload.get("event").is_some());
        assert!(payload.get("receipt").is_some());
    }

    #[test]
    fn tampered_policy_fails_closed_to_signed_default() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = verity_plane_config_path(temp.path());
        write_json(
            &cfg_path,
            &json!({
                "schema_id": "infring_verity_plane_policy",
                "schema_version": 1,
                "policy_version": 1,
                "mode": "simulation",
                "fidelity_warning_threshold": 0.40,
                "fidelity_lock_threshold": 0.20,
                "vector_warning_threshold": 0.10,
                "signature": "sig:tampered"
            }),
        );
        let payload = status_payload(temp.path(), &["status".to_string()]);
        assert_eq!(
            payload
                .get("policy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("signature_valid"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("policy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("mode"))
                .and_then(Value::as_str),
            Some("production")
        );
    }
}
