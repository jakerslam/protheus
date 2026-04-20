
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    match cmd.as_str() {
        "run-many" | "run-batch" => {
            let dispatch_enabled = parse_bool(parse_flag(argv, "dispatch"), true);
            let dispatch_strict = parse_bool(parse_flag(argv, "dispatch-strict"), true);
            let ids = match parse_id_list(root, argv) {
                Ok(rows) => rows,
                Err(code) => {
                    print_json_line(&with_hash(json!({
                        "ok": false,
                        "type": "srs_contract_runtime_error",
                        "code": code,
                        "message": "expected --ids=<ID1,ID2> or --ids-file=<path>"
                    })));
                    return 2;
                }
            };

            let mut results: Vec<Value> = Vec::new();
            let mut executed = 0usize;
            let mut failed = 0usize;
            for id in &ids {
                match execute_contract_with_options(root, id, dispatch_enabled, dispatch_strict) {
                    Ok(receipt) => {
                        let ok = receipt.get("ok").and_then(Value::as_bool).unwrap_or(false);
                        if ok {
                            executed += 1;
                        } else {
                            failed += 1;
                        }
                        results.push(json!({
                            "id": id,
                            "ok": ok,
                            "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null)
                        }));
                    }
                    Err(err) => {
                        failed += 1;
                        results.push(json!({
                            "id": id,
                            "ok": false,
                            "code": err
                        }));
                    }
                }
            }

            let out = with_hash(json!({
                "ok": failed == 0,
                "type": "srs_contract_runtime_batch_receipt",
                "lane": "srs_contract_runtime",
                "command": "run-many",
                "counts": {
                    "scanned": ids.len(),
                    "executed": executed,
                    "failed": failed
                },
                "results": results
            }));
            print_json_line(&out);
            if failed == 0 {
                0
            } else {
                1
            }
        }
        "run" => {
            let dispatch_enabled = parse_bool(parse_flag(argv, "dispatch"), true);
            let dispatch_strict = parse_bool(parse_flag(argv, "dispatch-strict"), true);
            let Some(id) = parse_id(argv) else {
                print_json_line(&with_hash(json!({
                    "ok": false,
                    "type": "srs_contract_runtime_error",
                    "code": "missing_id",
                    "message": "expected --id=<SRS-ID>"
                })));
                return 2;
            };
            match execute_contract_with_options(root, &id, dispatch_enabled, dispatch_strict) {
                Ok(out) => {
                    print_json_line(&out);
                    if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        0
                    } else {
                        1
                    }
                }
                Err(err) => {
                    print_json_line(&with_hash(json!({
                        "ok": false,
                        "type": "srs_contract_runtime_error",
                        "id": id,
                        "code": err
                    })));
                    1
                }
            }
        }
        "status" => {
            let Some(id) = parse_id(argv) else {
                print_json_line(&with_hash(json!({
                    "ok": false,
                    "type": "srs_contract_runtime_error",
                    "code": "missing_id",
                    "message": "expected --id=<SRS-ID>"
                })));
                return 2;
            };
            let latest = latest_path(root, &id);
            let out = if latest.exists() {
                read_json(&latest).unwrap_or_else(|_| {
                    with_hash(json!({
                        "ok": false,
                        "type": "srs_contract_runtime_error",
                        "id": id,
                        "code": "status_read_failed"
                    }))
                })
            } else {
                with_hash(json!({
                    "ok": false,
                    "type": "srs_contract_runtime_error",
                    "id": id,
                    "code": "status_not_found"
                }))
            };
            print_json_line(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        _ => {
            let id = parse_id(argv).unwrap_or_default();
            usage();
            print_json_line(&with_hash(json!({
                "ok": false,
                "type": "srs_contract_runtime_error",
                "id": id,
                "code": "unknown_command"
            })));
            2
        }
    }
}
