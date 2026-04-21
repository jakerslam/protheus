fn run_action_family_app_arm_001(root: &Path, _normalized: &str, payload: &Value) -> LaneResult {
{
            let provider = payload
                .get("provider")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "openai".to_string());
            let model = payload
                .get("model")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 100))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gpt-5".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "switch-provider".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--provider={provider}"),
                    format!("--model={model}"),
                ],
            )
        }
}
