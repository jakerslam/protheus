
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_emits_scaled_metadata_and_local_flag() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 7, "specialty":"coding"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "not_set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 2);
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("openai/gpt-5") }));
        assert!(rows
            .iter()
            .all(|row| parse_i64(row.get("power_scale"), 0) >= 1
                && parse_i64(row.get("power_scale"), 0) <= 5));
        assert!(rows.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("ollama")
                && row.get("is_local").and_then(Value::as_bool) == Some(true)
        }));
        assert!(rows.iter().any(|row| {
            row.get("id").and_then(Value::as_str) == Some("ollama/qwen2.5-coder:7b")
                && row.get("available").and_then(Value::as_bool) == Some(true)
        }));
    }

    #[test]
    fn route_prefers_local_when_offline_required() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let decision = route_decision_payload(
            root.path(),
            &json!({"ok": true}),
            &json!({"offline_required": true, "task_type": "general"}),
        );
        assert_eq!(
            decision
                .get("selected")
                .and_then(|v| v.get("provider"))
                .and_then(Value::as_str),
            Some("ollama")
        );
        assert!(decision
            .pointer("/route/fallback_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(
            decision
                .pointer("/route/retry_policy/max_total_attempts")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn route_applies_session_analytics_tuning_budget_and_model_bias() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 7, "specialty":"coding"},
                            "smallthinker:4b": {"power_rating": 3, "cost_rating": 3, "param_count_billion": 4, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        write_json(
            &root.path().join(SESSION_ANALYTICS_TUNING_REL),
            &json!({
                "routing": {
                    "default_budget_mode": "cheap",
                    "model_bias": {
                        "ollama/qwen2.5-coder:7b": 1.2
                    }
                }
            }),
        );
        let decision = route_decision_payload(
            root.path(),
            &json!({"ok": true}),
            &json!({
                "task_type": "code",
                "budget_mode": "balanced",
                "offline_required": true,
                "prefer_local": true
            }),
        );
        assert_eq!(
            decision
                .pointer("/analytics_tuning/default_budget_override_applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision
                .pointer("/input/budget_mode")
                .and_then(Value::as_str),
            Some("cheap")
        );
        assert_eq!(
            decision
                .pointer("/analytics_tuning/model_bias_entries")
                .and_then(Value::as_u64),
            Some(1)
        );
        let qwen_row = decision
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 200)
                        == "ollama/qwen2.5-coder:7b"
                })
            })
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            qwen_row.get("route_bias").and_then(Value::as_f64),
            Some(1.2)
        );
    }

    #[test]
    fn hosted_download_stub_stays_unavailable_without_chat_backend() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "cohere": {
                        "id": "cohere",
                        "display_name": "Cohere",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "not_set",
                        "reachable": false,
                        "model_profiles": {
                            "command-r": {
                                "power_rating": 3,
                                "cost_rating": 3,
                                "deployment_kind": "api",
                                "local_download_path": "/tmp/cohere/command-r",
                                "download_available": true
                            }
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let row = catalog
            .get("models")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("id").and_then(Value::as_str) == Some("cohere/command-r"))
            })
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(row.get("is_local").and_then(Value::as_bool), Some(false));
        assert_eq!(
            row.get("supports_chat").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(row.get("available").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn catalog_filters_placeholder_model_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "configured",
                        "reachable": true,
                        "model_profiles": {
                            "model": {"power_rating": 1, "cost_rating": 1, "specialty":"general"},
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "specialty":"coding"}
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("ollama/qwen2.5-coder:7b")));
        assert!(!rows
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("ollama/model")));
    }

    #[test]
    fn catalog_default_seed_is_not_truncated_to_three_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            rows.len() >= 12,
            "catalog should expose broad provider/model surface, got {} rows",
            rows.len()
        );
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("ollama/qwen3:4b") }));
        assert!(rows.iter().any(|row| {
            row.get("id").and_then(Value::as_str) == Some("openrouter/google/gemini-2.5-flash")
        }));
    }
}
