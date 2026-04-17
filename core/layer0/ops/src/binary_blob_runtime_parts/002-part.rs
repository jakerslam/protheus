fn emit(root: &Path, payload: Value) -> i32 {
    let mut normalized = payload;
    if normalized.get("lane").is_none() {
        normalized["lane"] = Value::String("core/layer0/ops".to_string());
    }
    if normalized.get("strict").is_none() {
        normalized["strict"] = Value::Bool(true);
    }
    if normalized.get("schema").is_none() {
        normalized["schema"] = Value::String("infring_layer1_security".to_string());
    }
    match write_receipt(root, STATE_ENV, STATE_SCOPE, normalized) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "binary_blob_runtime_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 240),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

fn verify_debug_token(root: &Path) -> Value {
    let (payload, code) = infring_layer1_security::run_soul_token_guard(
        root,
        &["verify".to_string(), "--strict=1".to_string()],
    );
    json!({"ok": code == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(false), "payload": payload, "code": code})
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    binary_blob_runtime_run::run(root, argv)
}

#[cfg(test)]
#[path = "binary_blob_runtime_tests.rs"]
mod tests;
