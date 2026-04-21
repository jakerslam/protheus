
fn run_ipc_daemon(root: &Path, argv: &[String]) -> Result<(), String> {
    let queue_dir = queue_dir_from_argv(root, argv);
    let poll_ms = poll_ms_from_argv(argv);
    let requests_dir = queue_dir.join("requests");
    let responses_dir = queue_dir.join("responses");
    let heartbeat_path = queue_dir.join("daemon.heartbeat.json");
    fs::create_dir_all(&requests_dir)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_requests_dir_failed:{err}"))?;
    fs::create_dir_all(&responses_dir)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_responses_dir_failed:{err}"))?;
    let _ = write_ipc_heartbeat(&heartbeat_path, poll_ms);
    let heartbeat_ticks = ((250 + poll_ms.saturating_sub(1)) / poll_ms.max(1)).max(1);
    let mut tick: u64 = 0;

    loop {
        if tick % heartbeat_ticks == 0 {
            let _ = write_ipc_heartbeat(&heartbeat_path, poll_ms);
        }
        tick = tick.wrapping_add(1);
        let mut request_files = fs::read_dir(&requests_dir)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_read_dir_failed:{err}"))?
            .filter_map(|entry| entry.ok().map(|row| row.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        request_files.sort();

        for request_path in request_files {
            let raw = fs::read_to_string(&request_path).unwrap_or_default();
            let request = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
            let request_id = clean_text(request.get("id"), 120);
            let domain = clean_text(request.get("domain"), 120);
            let args = request
                .get("args")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
                .unwrap_or_default();

            let response = if request_id.is_empty() {
                json!({
                    "ok": false,
                    "status": 2,
                    "payload": {
                        "ok": false,
                        "type": "ops_domain_ipc_request_invalid",
                        "reason": "missing_request_id"
                    }
                })
            } else if domain.is_empty() {
                json!({
                    "ok": false,
                    "status": 2,
                    "payload": {
                        "ok": false,
                        "type": "ops_domain_ipc_request_invalid",
                        "reason": "missing_domain"
                    }
                })
            } else {
                match run_domain_once(root, &domain, &args) {
                    Ok((status, payload)) => json!({
                        "ok": status == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
                        "status": status,
                        "payload": payload
                    }),
                    Err(err) => json!({
                        "ok": false,
                        "status": 1,
                        "payload": {
                            "ok": false,
                            "type": "ops_domain_conduit_bridge_error",
                            "reason": err
                        }
                    }),
                }
            };

            if !request_id.is_empty() {
                let response_path = responses_dir.join(format!("{request_id}.json"));
                let envelope = json!({
                    "ok": response.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    "request_id": request_id,
                    "response": response
                });
                let _ = write_json_atomic(&response_path, &envelope);
            }
            let _ = fs::remove_file(&request_path);
        }

        thread::sleep(Duration::from_millis(poll_ms));
    }
}

pub fn run(root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error(
                "ops_domain_conduit_runner_kernel_error",
                err.as_str(),
            ));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let result = match command.as_str() {
        "parse-argv" => Ok(run_parse_argv(payload)),
        "build-pass-args" => Ok(run_build_pass_args(payload)),
        "build-run-options" => Ok(run_build_run_options(payload)),
        "prepare-run" => Ok(run_prepare_run(payload)),
        "run" => Ok(run_execute(root, payload)),
        "ipc-daemon" => match run_ipc_daemon(root, argv) {
            Ok(()) => {
                Ok(json!({"ok": true, "type": "ops_domain_conduit_runner_kernel_ipc_daemon"}))
            }
            Err(err) => Err(err),
        },
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err(format!(
            "ops_domain_conduit_runner_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("ops_domain_conduit_runner_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                "ops_domain_conduit_runner_kernel_error",
                err.as_str(),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pass_args_respects_flag_domain() {
        let parsed = parse_args_map(&[
            "--domain".to_string(),
            "legacy-retired-lane".to_string(),
            "build".to_string(),
            "--lane-id=FOO-1".to_string(),
        ]);
        let args = build_pass_args_vec(&parsed);
        assert_eq!(
            args,
            vec!["build".to_string(), "--lane-id=FOO-1".to_string()]
        );
    }

    #[test]
    fn build_pass_args_strips_positional_domain() {
        let parsed = parse_args_map(&[
            "legacy-retired-lane".to_string(),
            "build".to_string(),
            "--lane-id=FOO-2".to_string(),
        ]);
        let args = build_pass_args_vec(&parsed);
        assert_eq!(
            args,
            vec!["build".to_string(), "--lane-id=FOO-2".to_string()]
        );
    }

    #[test]
    fn run_execute_missing_domain_returns_status_2() {
        let root = std::path::Path::new(".");
        let payload = Map::new();
        let out = run_execute(root, &payload);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("status").and_then(Value::as_i64), Some(2));
        let reason = out
            .get("payload")
            .and_then(Value::as_object)
            .and_then(|value| value.get("reason"))
            .and_then(Value::as_str);
        assert_eq!(reason, Some("missing_domain"));
    }
}
