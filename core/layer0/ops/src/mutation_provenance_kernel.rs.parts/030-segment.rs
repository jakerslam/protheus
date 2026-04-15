#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn normalize_meta_prefers_runtime_relative_source() {
        let tmp = tempdir().expect("tempdir");
        std::env::set_var("PROTHEUS_WORKSPACE_ROOT", tmp.path());
        let runtime_source = tmp
            .path()
            .join("client")
            .join("runtime")
            .join("systems")
            .join("adaptive")
            .join("planner.ts");
        let meta = json!({ "source": runtime_source, "reason": "sync" });
        let normalized = normalize_meta_value(tmp.path(), as_object(Some(&meta)), "", "fallback");
        assert_eq!(
            normalized.get("source").and_then(Value::as_str),
            Some("systems/adaptive/planner.ts")
        );
    }

    #[test]
    fn strict_enforcement_blocks_violation() {
        let tmp = tempdir().expect("tempdir");
        std::env::set_var("PROTHEUS_WORKSPACE_ROOT", tmp.path());
        let policy_path = tmp
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("mutation_provenance_policy.json");
        ensure_parent(&policy_path).expect("policy dir");
        fs::write(
            &policy_path,
            serde_json::to_string_pretty(&json!({
                "version": "test",
                "channels": {
                    "adaptive": {
                        "allowed_source_prefixes": ["systems/adaptive/"],
                        "require_reason": true
                    }
                }
            }))
            .expect("encode"),
        )
        .expect("write policy");
        let payload = json!({
            "channel": "adaptive",
            "meta": { "source": tmp.path().join("bad.ts"), "reason": "" },
            "opts": { "strict": true }
        });
        let err = enforce_value(tmp.path(), payload_obj(&payload)).expect_err("strict block");
        assert!(err.starts_with("mutation_provenance_blocked:adaptive:"));
    }
}
