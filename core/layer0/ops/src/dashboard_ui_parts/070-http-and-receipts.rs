
fn write_action_receipt(root: &Path, action: &str, payload: &Value, lane: &LaneResult) -> Value {
    let mut row = json!({
        "ok": lane.ok,
        "type": "infring_dashboard_action_receipt",
        "ts": now_iso(),
        "action": clean_text(action, 120),
        "payload": payload.clone(),
        "lane_status": lane.status,
        "lane_argv": lane.argv,
        "lane_receipt_hash": lane
            .payload
            .as_ref()
            .and_then(|v| v.get("receipt_hash"))
            .cloned()
            .unwrap_or(Value::Null)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    write_json(&root.join(ACTION_LATEST_REL), &row);
    append_jsonl(&root.join(ACTION_HISTORY_REL), &row);
    row
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_request(mut stream: &TcpStream) -> Result<HttpRequest, String> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2000)));
    let mut data = Vec::<u8>::new();
    let mut chunk = [0u8; 4096];
    let header_end;
    loop {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("request_read_failed:{err}"))?;
        if n == 0 {
            return Err("request_closed".to_string());
        }
        data.extend_from_slice(&chunk[..n]);
        if data.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large".to_string());
        }
        if let Some(pos) = find_bytes(&data, b"\r\n\r\n") {
            header_end = pos;
            break;
        }
    }
    let header_raw = String::from_utf8_lossy(&data[..header_end]).to_string();
    let mut lines = header_raw.lines();
    let Some(first_line) = lines.next() else {
        return Err("request_line_missing".to_string());
    };
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .map(|v| v.to_ascii_uppercase())
        .ok_or_else(|| "request_method_missing".to_string())?;
    let path = parts
        .next()
        .map(|v| v.to_string())
        .ok_or_else(|| "request_path_missing".to_string())?;

    let mut content_length = 0usize;
    let mut headers = Vec::<(String, String)>::new();
    for line in lines {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim().to_string();
        let value = v.trim().to_string();
        if !key.is_empty() {
            headers.push((key.clone(), value.clone()));
        }
        if key.eq_ignore_ascii_case("content-length") {
            content_length = value.parse::<usize>().unwrap_or(0);
        }
    }
    if content_length > MAX_REQUEST_BYTES {
        return Err("content_length_too_large".to_string());
    }

    let mut body = data[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("request_body_read_failed:{err}"))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > MAX_REQUEST_BYTES {
            return Err("request_body_too_large".to_string());
        }
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn write_response(
    mut stream: &TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        status,
        status_reason(status),
        content_type,
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|err| format!("response_head_write_failed:{err}"))?;
    stream
        .write_all(body)
        .map_err(|err| format!("response_body_write_failed:{err}"))?;
    stream
        .flush()
        .map_err(|err| format!("response_flush_failed:{err}"))
}

fn now_unix_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn maybe_schedule_snapshot_refresh(
    root: &Path,
    flags: &Flags,
    latest_snapshot: &Arc<Mutex<SnapshotState>>,
) {
    let now_ms = now_unix_ms();
    let mut should_spawn = false;
    if let Ok(mut guard) = latest_snapshot.lock() {
        let stale = now_ms.saturating_sub(guard.built_at_ms) >= flags.refresh_ms as i64;
        if stale && !guard.refresh_inflight {
            guard.refresh_inflight = true;
            should_spawn = true;
        }
    }
    if !should_spawn {
        return;
    }

    let root_owned = root.to_path_buf();
    let flags_owned = flags.clone();
    let state = Arc::clone(latest_snapshot);
    std::thread::spawn(move || {
        let snapshot = build_snapshot(&root_owned, &flags_owned);
        write_snapshot_receipt(&root_owned, &snapshot);
        if let Ok(mut guard) = state.lock() {
            guard.snapshot = snapshot;
            guard.built_at_ms = now_unix_ms();
            guard.refresh_inflight = false;
        }
    });
}

fn handle_request(
    root: &Path,
    flags: &Flags,
    latest_snapshot: &Arc<Mutex<SnapshotState>>,
    stream: &TcpStream,
) -> Result<(), String> {
    let req = parse_request(stream)?;
    if req.method == "GET" && (req.path == "/" || req.path == "/dashboard") {
        let out = json!({
            "ok": false,
            "type": "dashboard_api_only",
            "message": "This Rust dashboard lane serves APIs only. Open the unified dashboard host port for the browser UI.",
            "ui_entrypoint": "client/runtime/systems/ui/infring_dashboard.ts",
            "path": req.path
        });
        let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
        return write_response(
            stream,
            404,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.method == "GET" && req.path == "/api/dashboard/snapshot" {
        maybe_schedule_snapshot_refresh(root, flags, latest_snapshot);
        let snapshot = latest_snapshot
            .lock()
            .ok()
            .map(|state| state.snapshot.clone())
            .unwrap_or_else(|| build_snapshot(root, flags));
        let body = serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string());
        return write_response(
            stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.method == "POST" && req.path == "/api/dashboard/action" {
        let payload =
            parse_json_loose(&String::from_utf8_lossy(&req.body)).unwrap_or_else(|| json!({}));
        let action = payload
            .get("action")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 80))
            .unwrap_or_default();
        let action_payload = payload.get("payload").cloned().unwrap_or_else(|| json!({}));
        let lane = run_action(root, &action, &action_payload);
        let action_receipt = write_action_receipt(root, &action, &action_payload, &lane);
        let snapshot = build_snapshot(root, flags);
        write_snapshot_receipt(root, &snapshot);
        if let Ok(mut guard) = latest_snapshot.lock() {
            guard.snapshot = snapshot.clone();
            guard.built_at_ms = now_unix_ms();
            guard.refresh_inflight = false;
        }
        let out = json!({
            "ok": lane.ok,
            "type": "infring_dashboard_action_response",
            "action": action,
            "action_receipt": action_receipt,
            "lane": lane.payload.unwrap_or(Value::Null),
            "snapshot": snapshot
        });
        let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
        let status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            200
        } else {
            400
        };
        return write_response(
            stream,
            status,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.method == "GET" && req.path == "/healthz" {
        maybe_schedule_snapshot_refresh(root, flags, latest_snapshot);
        let hash = latest_snapshot
            .lock()
            .ok()
            .and_then(|s| s.snapshot.get("receipt_hash").cloned())
            .unwrap_or(Value::Null);
        let out = json!({
            "ok": true,
            "type": "infring_dashboard_healthz",
            "ts": now_iso(),
            "receipt_hash": hash
        });
        let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
        return write_response(
            stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.path.starts_with("/api/") {
        maybe_schedule_snapshot_refresh(root, flags, latest_snapshot);
        let snapshot = latest_snapshot
            .lock()
            .ok()
            .map(|v| v.snapshot.clone())
            .unwrap_or_else(|| build_snapshot(root, flags));
        let header_refs = req
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        if let Some(response) = dashboard_compat_api::handle_with_headers(
            root,
            &req.method,
            &req.path,
            &req.body,
            &header_refs,
            &snapshot,
        ) {
            let body = serde_json::to_string_pretty(&response.payload)
                .unwrap_or_else(|_| "{}".to_string());
            return write_response(
                stream,
                response.status,
                "application/json; charset=utf-8",
                body.as_bytes(),
            );
        }
    }

    let out = json!({
        "ok": false,
        "type": "infring_dashboard_not_found",
        "path": req.path
    });
    let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
    write_response(
        stream,
        404,
        "application/json; charset=utf-8",
        body.as_bytes(),
    )
}

fn run_serve(root: &Path, flags: &Flags) -> i32 {
    ensure_dir(&root.join(STATE_DIR_REL));
    ensure_dir(&root.join(ACTION_DIR_REL));

    let initial = build_snapshot(root, flags);
    write_snapshot_receipt(root, &initial);
    let latest_snapshot = Arc::new(Mutex::new(SnapshotState {
        snapshot: initial.clone(),
        built_at_ms: now_unix_ms(),
        refresh_inflight: false,
    }));
    let addr = format!("{}:{}", flags.host, flags.port);
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infring_dashboard_server_error",
                    "error": clean_text(&format!("bind_failed:{err}"), 220),
                    "host": flags.host,
                    "port": flags.port
                })
            );
            return 1;
        }
    };

    let url = format!("http://{}:{}", flags.host, flags.port);
    let status = json!({
        "ok": true,
        "type": "infring_dashboard_server",
        "ts": now_iso(),
        "url": url,
        "host": flags.host,
        "port": flags.port,
        "refresh_ms": flags.refresh_ms,
        "team": flags.team,
        "authority": "rust_core_api_only",
        "receipt_hash": initial.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "snapshot_path": SNAPSHOT_LATEST_REL,
        "action_path": ACTION_LATEST_REL
    });
    write_json(
        &root.join(STATE_DIR_REL).join("server_status.json"),
        &status,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&status).unwrap_or_else(|_| "{}".to_string())
    );
    println!("Dashboard API listening at {url}");

    let root_owned = root.to_path_buf();
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let latest_snapshot_ref = Arc::clone(&latest_snapshot);
        let root_ref = root_owned.clone();
        let flags_ref = flags.clone();
        std::thread::spawn(move || {
            if let Err(err) = handle_request(&root_ref, &flags_ref, &latest_snapshot_ref, &stream) {
                let out = json!({
                    "ok": false,
                    "type": "infring_dashboard_request_error",
                    "ts": now_iso(),
                    "error": clean_text(&err, 240)
                });
                let body = serde_json::to_string_pretty(&out)
                    .unwrap_or_else(|_| "{\"ok\":false}".to_string());
                let _ = write_response(
                    &stream,
                    500,
                    "application/json; charset=utf-8",
                    body.as_bytes(),
                );
            }
        });
    }
    0
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let flags = parse_flags(argv);
    match flags.mode.as_str() {
        "git-authority" | "git-authority-v1" => run_git_authority(root, &flags, argv),
        "runtime-sync" | "runtime" => {
            let sync = build_runtime_sync(root, &flags);
            write_json_stdout(&sync, flags.pretty);
            0
        }
        "snapshot" | "status" => {
            let snapshot = build_snapshot(root, &flags);
            write_snapshot_receipt(root, &snapshot);
            write_json_stdout(&snapshot, flags.pretty);
            0
        }
        "serve" | "web" => run_serve(root, &flags),
        _ => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infring_dashboard_cli_error",
                    "error": format!("unsupported_mode:{} (expected serve|snapshot|status|runtime-sync|git-authority)", flags.mode)
                })
            );
            2
        }
    }
}
