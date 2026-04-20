
fn parse_runtime_lane_argv(row: &Value, domain: &str) -> Vec<String> {
    if let Some(raw_argv) = row.get("argv").and_then(Value::as_array) {
        let mut argv = Vec::<String>::new();
        for token in raw_argv {
            let Some(text) = token.as_str() else {
                continue;
            };
            let clean = text.trim();
            if clean.is_empty() {
                continue;
            }
            argv.push(clean.to_string());
        }
        if !argv.is_empty() {
            return argv;
        }
    }

    let action = row
        .get("action")
        .or_else(|| row.get("op"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("status");
    vec![domain.to_string(), action.to_string()]
}

fn runtime_lane_targets(contract: &Value) -> Vec<DispatchTarget> {
    let mut targets = Vec::<DispatchTarget>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    let deliverables = contract
        .get("deliverables")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in &deliverables {
        let dtype = row.get("type").and_then(Value::as_str).unwrap_or_default();
        if !matches!(dtype, "runtime_lane" | "core_authority") {
            continue;
        }
        let Some(path) = row.get("path").and_then(Value::as_str) else {
            continue;
        };
        let Some(domain) = runtime_lane_to_domain(path) else {
            continue;
        };
        let argv = parse_runtime_lane_argv(row, domain);
        let dedupe_key = argv.join("\u{1f}");
        if !seen.insert(dedupe_key) {
            continue;
        }
        targets.push(DispatchTarget {
            plane: domain.to_string(),
            source_path: path.to_string(),
            argv,
        });
    }
    targets
}

fn parse_json_payload(raw: &[u8]) -> Option<Value> {
    let body = String::from_utf8_lossy(raw);
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    for line in trimmed.lines().rev() {
        let line_trim = line.trim();
        if line_trim.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line_trim) {
            return Some(value);
        }
    }
    None
}

fn run_dispatch_target(
    root: &Path,
    target: &DispatchTarget,
    dispatch_strict: bool,
    dispatch_bin: &str,
) -> Value {
    let mut command = Command::new(dispatch_bin);
    command.current_dir(root);
    for arg in &target.argv {
        command.arg(arg);
    }
    if dispatch_strict {
        command.arg("--strict=1");
    }
    match command.output() {
        Ok(output) => {
            let parsed = parse_json_payload(&output.stdout);
            let exit_code = output.status.code().unwrap_or(1);
            let parsed_ok = parsed
                .as_ref()
                .and_then(|value| value.get("ok"))
                .and_then(Value::as_bool)
                .unwrap_or(output.status.success());
            let mut stdout_hasher = Sha256::new();
            stdout_hasher.update(&output.stdout);
            let stdout_sha256 = format!("sha256:{}", hex::encode(stdout_hasher.finalize()));
            json!({
                "ok": output.status.success() && parsed_ok,
                "plane": target.plane,
                "source_path": target.source_path,
                "argv": target.argv,
                "dispatch_bin": dispatch_bin,
                "exit_code": exit_code,
                "stdout_sha256": stdout_sha256,
                "stderr": String::from_utf8_lossy(&output.stderr),
                "receipt": parsed.unwrap_or(Value::Null)
            })
        }
        Err(err) => json!({
            "ok": false,
            "plane": target.plane,
            "source_path": target.source_path,
            "argv": target.argv,
            "dispatch_bin": dispatch_bin,
            "error": format!("dispatch_spawn_failed:{err}")
        }),
    }
}

fn with_hash(mut payload: Value) -> Value {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

pub fn contract_exists(root: &Path, id: &str) -> bool {
    contract_path(root, &id.to_ascii_uppercase()).exists()
}

pub fn execute_contract(root: &Path, id: &str) -> Result<Value, String> {
    execute_contract_with_options(root, id, true, true)
}
