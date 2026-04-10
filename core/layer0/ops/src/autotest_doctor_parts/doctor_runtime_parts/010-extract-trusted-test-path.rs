fn trusted_test_path(path: String) -> TrustedTestPath {
    TrustedTestPath {
        path: Some(path),
        trusted: true,
        reason: None,
    }
}

fn untrusted_test_path(reason: &str) -> TrustedTestPath {
    TrustedTestPath {
        path: None,
        trusted: false,
        reason: Some(reason.to_string()),
    }
}

fn has_shell_meta(command: &str) -> bool {
    const SHELL_META: [&str; 8] = ["|", "&&", ";", "$(", "`", ">", "<", "\n"];
    SHELL_META.iter().any(|token| command.contains(token))
}

fn extract_trusted_test_path(command: &str) -> TrustedTestPath {
    let cmd = command.trim();
    if cmd.is_empty() {
        return untrusted_test_path("missing_command");
    }
    if has_shell_meta(cmd) {
        return untrusted_test_path("shell_meta_detected");
    }

    let mut parts = cmd.split_whitespace();
    let head = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    if !head.eq_ignore_ascii_case("node") || !path.ends_with(".test.ts") {
        return untrusted_test_path("non_node_test_command");
    }
    let norm = path.trim_matches('"').trim_matches('\'').replace('\\', "/");
    if !norm.starts_with("tests/client-memory-tools/") {
        return untrusted_test_path("path_outside_allowlist");
    }
    if norm.contains("..") {
        return untrusted_test_path("path_traversal");
    }
    trusted_test_path(norm)
}

fn collect_failures(run_row: &Value) -> Vec<FailureSignature> {
    let mut out = Vec::new();
    let results = run_row
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for result in results {
        let failed = !result.get("ok").and_then(Value::as_bool).unwrap_or(false)
            || !result
                .get("guard_ok")
                .and_then(Value::as_bool)
                .unwrap_or(true);
        if !failed {
            continue;
        }

        let kind = classify_failure_kind(&result);
        let command = result
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let test_meta = extract_trusted_test_path(command);
        let guard_files = result
            .get("guard_files")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| clean_text(s, 260))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let seed = format!(
            "{}|{}|{}|{}|{}",
            result.get("id").and_then(Value::as_str).unwrap_or_default(),
            kind,
            test_meta.path.clone().unwrap_or_default(),
            result
                .get("guard_reason")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            result
                .get("exit_code")
                .and_then(Value::as_i64)
                .unwrap_or_default()
        );
        let signature_id = stable_id("sig", &seed);

        out.push(FailureSignature {
            signature_id,
            kind,
            test_id: result
                .get("id")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 120))
                .filter(|s| !s.is_empty()),
            command: Some(clean_text(command, 260)).filter(|s| !s.is_empty()),
            test_path: test_meta.path.clone(),
            trusted_test_command: test_meta.trusted,
            untrusted_reason: if test_meta.trusted {
                None
            } else {
                test_meta.reason.clone()
            },
            exit_code: result.get("exit_code").and_then(Value::as_i64),
            guard_ok: result
                .get("guard_ok")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            guard_reason: result
                .get("guard_reason")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 180))
                .filter(|s| !s.is_empty()),
            stderr_excerpt: result
                .get("stderr_excerpt")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 600))
                .filter(|s| !s.is_empty()),
            stdout_excerpt: result
                .get("stdout_excerpt")
                .and_then(Value::as_str)
                .map(|s| clean_text(s, 600))
                .filter(|s| !s.is_empty()),
            guard_files,
            flaky: result
                .get("flaky")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        });
    }

    out
}

fn load_latest_autotest_run(
    paths: &RuntimePaths,
    date_arg: &str,
) -> Option<(PathBuf, String, Value)> {
    let key = date_arg.trim().to_ascii_lowercase();
    if key == "latest" {
        let payload = read_json(&paths.autotest_latest_path);
        if payload.is_object() {
            return Some((
                paths.autotest_latest_path.clone(),
                now_iso()[..10].to_string(),
                payload,
            ));
        }
        return None;
    }

    let date = if key.len() == 10 {
        key
    } else {
        now_iso()[..10].to_string()
    };
    let file = paths.autotest_runs_dir.join(format!("{date}.jsonl"));
    if !file.exists() {
        return None;
    }
    let mut selected = None::<Value>;
    for row in read_jsonl(&file) {
        if row.get("type").and_then(Value::as_str).unwrap_or_default() == "autotest_run" {
            selected = Some(row);
        }
    }
    selected.map(|payload| (file, date, payload))
}

fn ensure_signature_state<'a>(
    state: &'a mut DoctorState,
    signature_id: &str,
) -> &'a mut SignatureState {
    state
        .signatures
        .entry(signature_id.to_string())
        .or_default()
}
