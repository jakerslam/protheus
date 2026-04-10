#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn payload(result: &CommandResult) -> &Value {
        result.payload.get("payload").unwrap_or(&result.payload)
    }

    fn json_flag(name: &str, value: &Value) -> String {
        format!("--{name}={}", serde_json::to_string(value).unwrap())
    }

    fn sync_catalog(root: &Path, catalog: &Value) -> CommandResult {
        run_command(
            root,
            &[
                "sync".to_string(),
                json_flag("catalog-json", catalog),
                "--strict=1".to_string(),
            ],
        )
    }

    #[test]
    fn sync_search_integrate_roundtrip() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let catalog = json!({
            "actions": [
                {
                    "id": "slack.chat.post_message",
                    "platform": "slack",
                    "title": "Post message",
                    "method": "POST",
                    "url": "https://slack.com/api/chat.postMessage",
                    "parameters": {"required":["channel","text"]},
                    "updated_epoch_ms": now_epoch_ms()
                }
            ]
        });
        let sync = sync_catalog(root, &catalog);
        assert_eq!(sync.exit_code, 0);

        let search = run_command(
            root,
            &[
                "search".to_string(),
                "--query=slack message".to_string(),
                "--limit=5".to_string(),
            ],
        );
        assert_eq!(search.exit_code, 0);
        let results = payload(&search)
            .get("results")
            .and_then(Value::as_array)
            .expect("results");
        assert!(!results.is_empty());
        assert_eq!(
            results[0].get("platform").and_then(Value::as_str),
            Some("slack")
        );

        let integrate = run_command(
            root,
            &[
                "integrate".to_string(),
                "--action-id=slack.chat.post_message".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(integrate.exit_code, 0);
        let method = payload(&integrate)
            .pointer("/request_template/method")
            .and_then(Value::as_str);
        assert_eq!(method, Some("POST"));
    }

    #[test]
    fn connect_import_run_flow() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();

        let sync = run_command(root, &["sync".to_string()]);
        assert_eq!(sync.exit_code, 0);

        let connect = run_command(
            root,
            &[
                "connect".to_string(),
                "--platform=slack".to_string(),
                "--access-token=test-token".to_string(),
                "--oauth-passthrough=1".to_string(),
            ],
        );
        assert_eq!(connect.exit_code, 0);

        let flow = json!({
            "id": "notify_flow",
            "name": "Notify",
            "steps": [
                {
                    "id": "notify",
                    "action_id": "slack.chat.post_message",
                    "input": {"channel":"#ops","text":"done"}
                }
            ]
        });
        let import = run_command(
            root,
            &[
                "import-flow".to_string(),
                format!("--flow-json={}", serde_json::to_string(&flow).unwrap()),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(import.exit_code, 0);

        let run = run_command(
            root,
            &[
                "run-flow".to_string(),
                "--workflow-id=notify_flow".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(run.exit_code, 0);
        let completion = payload(&run)
            .get("completion_percent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(completion >= 99.9);
    }

    #[test]
    fn stale_action_rejected_in_strict_integrate() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let catalog = json!({
            "actions": [
                {
                    "id": "gmail.messages.send",
                    "platform": "gmail",
                    "title": "Send",
                    "method": "POST",
                    "url": "https://gmail.googleapis.com/gmail/v1/users/me/messages/send",
                    "updated_epoch_ms": 1
                }
            ]
        });
        let sync = sync_catalog(root, &catalog);
        assert_eq!(sync.exit_code, 0);

        let integrate = run_command(
            root,
            &[
                "integrate".to_string(),
                "--action-id=gmail.messages.send".to_string(),
                "--strict=1".to_string(),
                "--max-age-days=1".to_string(),
            ],
        );
        assert_ne!(integrate.exit_code, 0);
        let code = payload(&integrate).get("code").and_then(Value::as_str);
        assert_eq!(code, Some("action_schema_stale"));
    }
}
