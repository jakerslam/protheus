
fn decode_base64_text(raw: Option<&String>, field: &str) -> Result<String, String> {
    let encoded = raw
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing_{field}"))?;
    let bytes = BASE64_STANDARD
        .decode(encoded.as_bytes())
        .map_err(|err| format!("base64_decode_failed:{field}:{err}"))?;
    String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{field}:{err}"))
}

fn decode_base64_json(raw: Option<&String>, field: &str) -> Result<Value, String> {
    let text = decode_base64_text(raw, field)?;
    serde_json::from_str(&text).map_err(|err| format!("json_decode_failed:{field}:{err}"))
}

fn decode_payload_or_emit(
    root: &Path,
    payload_base64: Option<&String>,
    receipt_type: &str,
) -> Result<Value, i32> {
    decode_base64_json(payload_base64, "payload_base64").map_err(|err| {
        emit_receipt(
            root,
            json!({
                "ok": false,
                "type": receipt_type,
                "lane": "core/layer0/ops",
                "error": err
            }),
        )
    })
}

pub(super) fn emit_receipt(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
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
                "type": "directive_kernel_error",
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
