
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_runtime_file(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dir");
        }
        fs::write(path, contents).expect("write file");
    }

    #[test]
    fn security_integrity_kernel_seal_and_verify_round_trip() {
        let temp = tempdir().expect("tempdir");
        let runtime_root = temp.path().join("client").join("runtime");
        fs::create_dir_all(&runtime_root).expect("runtime root");

        write_runtime_file(
            &runtime_root,
            "systems/security/guard.js",
            "module.exports = 1;\n",
        );
        write_runtime_file(
            &runtime_root,
            "config/directives/policy.yaml",
            "mode: strict\n",
        );

        let policy_path = runtime_root.join(DEFAULT_POLICY_REL);
        let payload = Map::new();

        let sealed = seal(&runtime_root, &policy_path, &payload).expect("seal");
        assert_eq!(sealed.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(sealed.get("sealed_files").and_then(Value::as_u64), Some(2));

        let verified = verify(&runtime_root, &policy_path, None).expect("verify");
        assert_eq!(verified.get("ok").and_then(Value::as_bool), Some(true));

        write_runtime_file(
            &runtime_root,
            "systems/security/guard.js",
            "module.exports = 2;\n",
        );
        let broken = verify(&runtime_root, &policy_path, None).expect("verify mismatch");
        assert_eq!(broken.get("ok").and_then(Value::as_bool), Some(false));
        let violations = broken
            .get("violations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(violations
            .iter()
            .any(|row| { row.get("type").and_then(Value::as_str) == Some("hash_mismatch") }));
    }

    #[test]
    fn security_integrity_kernel_appends_log_rows() {
        let temp = tempdir().expect("tempdir");
        let log_path = temp.path().join("integrity.jsonl");
        let mut payload = Map::new();
        payload.insert(
            "entry".to_string(),
            json!({
                "type": "hash_mismatch",
                "file": "systems/security/guard.js"
            }),
        );

        let result = append_event(&log_path, &payload).expect("append");
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(true));

        let lines = fs::read_to_string(&log_path).expect("read log");
        assert!(lines.contains("hash_mismatch"));
        assert!(lines.contains("guard.js"));
    }
}
