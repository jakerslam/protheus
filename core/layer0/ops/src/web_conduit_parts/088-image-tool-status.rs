fn append_web_image_tool_entry(tool_catalog: &mut Value, root: &Path, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        let runtime = crate::web_conduit_provider_runtime::image_tool_runtime_resolution_snapshot(
            root,
            policy,
            &json!({}),
        );
        rows.push(json!({
            "tool": "web_image_tool",
            "label": "Web Image Tool",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": runtime.get("selected_provider").cloned().unwrap_or(Value::Null),
            "default_model": runtime.get("selected_model").cloned().unwrap_or(Value::Null),
            "request_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, policy),
            "runtime": runtime
        }));
    }
}

pub(crate) fn api_image_tool_status(root: &Path, request: &Value) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let runtime = crate::web_conduit_provider_runtime::image_tool_runtime_resolution_snapshot(
        root, &policy, request,
    );
    json!({
        "ok": true,
        "type": "web_image_tool_status",
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "request_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy),
        "runtime": runtime
    })
}
