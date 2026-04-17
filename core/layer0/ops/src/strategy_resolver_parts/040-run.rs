fn attach_execution_receipt(out: &mut Value, command: &str, status: &str, op: Option<&str>) {
    out["execution_receipt"] = json!({
        "lane": "strategy_resolver",
        "command": command,
        "op": op.unwrap_or(""),
        "status": status,
        "source": "OPENCLAW-TOOLING-WEB-101",
        "tool_runtime_class": "receipt_wrapped"
    });
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops strategy-resolver status");
        println!("  protheus-ops strategy-resolver invoke --payload=<json>");
        return 0;
    }

    if cmd == "status" {
        let mut out = json!({
            "ok": true,
            "type": "strategy_resolver_status",
            "authority": "core/layer2/execution",
            "commands": ["status", "invoke"],
            "default_strategy_dir": DEFAULT_STRATEGY_DIR_REL,
            "default_weaver_overlay_path": DEFAULT_WEAVER_OVERLAY_REL,
            "ts": now_iso(),
            "root": clean(root.display(), 280)
        });
        attach_execution_receipt(&mut out, "status", "success", None);
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 0;
    }

    if cmd != "invoke" {
        let mut out = json!({
            "ok": false,
            "type": "strategy_resolver_cli_error",
            "authority": "core/layer2/execution",
            "command": cmd,
            "error": "unknown_command",
            "ts": now_iso(),
            "exit_code": 2
        });
        attach_execution_receipt(&mut out, &cmd, "error", None);
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 2;
    }

    let payload = match load_payload(argv) {
        Ok(value) => value,
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "strategy_resolver_cli_error",
                "authority": "core/layer2/execution",
                "command": "invoke",
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            attach_execution_receipt(&mut out, "invoke", "error", None);
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            return 2;
        }
    };

    let op = payload
        .get("op")
        .map(|v| as_str(Some(v)))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "".to_string());

    let result = op_dispatch(root, op.as_str(), payload.get("args"));
    match result {
        Ok(result_value) => {
            let mut out = json!({
                "ok": true,
                "type": "strategy_resolver",
                "authority": "core/layer2/execution",
                "command": "invoke",
                "op": op,
                "result": result_value,
                "ts": now_iso(),
                "root": clean(root.display(), 280)
            });
            attach_execution_receipt(&mut out, "invoke", "success", Some(op.as_str()));
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            0
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "strategy_resolver",
                "authority": "core/layer2/execution",
                "command": "invoke",
                "op": op,
                "error": err,
                "ts": now_iso(),
                "exit_code": 2
            });
            attach_execution_receipt(&mut out, "invoke", "error", Some(op.as_str()));
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            2
        }
    }
}
