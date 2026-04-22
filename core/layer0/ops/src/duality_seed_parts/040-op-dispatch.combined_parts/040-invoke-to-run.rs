

pub fn invoke(root: &Path, op: &str, args: Option<&Value>) -> Result<Value, String> {
    op_dispatch(root, op, args)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops duality-seed status");
        println!("  protheus-ops duality-seed invoke --payload=<json>");
        return 0;
    }

    if cmd == "status" {
        let mut out = json!({
            "ok": true,
            "type": "duality_seed_status",
            "authority": "core/layer2/autonomy",
            "commands": ["status", "invoke"],
            "default_policy_path": DEFAULT_POLICY_REL,
            "default_codex_path": DEFAULT_CODEX_REL,
            "default_latest_state_path": DEFAULT_LATEST_REL,
            "default_history_path": DEFAULT_HISTORY_REL,
            "ts": now_iso(),
            "root": clean(root.display(), 280)
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 0;
    }

    if cmd != "invoke" {
        let mut out = json!({
            "ok": false,
            "type": "duality_seed_cli_error",
            "authority": "core/layer2/autonomy",
            "command": cmd,
            "error": "unknown_command",
            "ts": now_iso(),
            "exit_code": 2
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 2;
    }

    let payload = match load_payload(argv) {
        Ok(value) => value,
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "duality_seed_cli_error",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json_line(&out);
            return 2;
        }
    };

    let op = payload
        .get("op")
        .map(|v| as_str(Some(v)))
        .filter(|v| !v.is_empty())
        .unwrap_or_default();

    let result = op_dispatch(root, op.as_str(), payload.get("args"));
    match result {
        Ok(result_value) => {
            let mut out = json!({
                "ok": true,
                "type": "duality_seed",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "op": op,
                "result": result_value,
                "ts": now_iso(),
                "root": clean(root.display(), 280)
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json_line(&out);
            0
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "duality_seed",
                "authority": "core/layer2/autonomy",
                "command": "invoke",
                "op": op,
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json_line(&out);
            2
        }
    }
}
