
fn upload_create_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let bytes = upload_bytes_from_request(&request, body);
    if bytes.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "upload_empty"}),
        };
    }
    let filename = clean_text(
        request
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or("upload.bin"),
        200,
    );
    let upload_id = format!(
        "upl_{}",
        stable_hash(&format!("{}|{}|{}", filename, bytes.len(), now_ms()), 18)
    );
    let digest = stable_hash(&String::from_utf8_lossy(&bytes), 16);
    let uploads_dir = state_path(root, REFERENCE_UPLOADS_DIR_REL);
    let _ = fs::create_dir_all(&uploads_dir);
    let file_path = uploads_dir.join(format!("{upload_id}.bin"));
    let _ = fs::write(&file_path, &bytes);
    let mut state = load_parity_state(root);
    let uploads = as_array_mut(&mut state, "uploads");
    uploads.retain(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != upload_id
    });
    uploads.push(json!({
        "upload_id": upload_id,
        "filename": filename,
        "size_bytes": bytes.len(),
        "hash": digest,
        "path": clean_text(file_path.to_string_lossy().as_ref(), 500),
        "created_at": crate::now_iso(),
        "created_at_ms": now_ms()
    }));
    save_parity_state(root, state.clone());
    let upload = state
        .get("uploads")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
        .cloned()
        .unwrap_or_else(|| json!({}));
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": upload}),
    }
}

fn upload_detail_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let Some(row) = uploads_list(root).into_iter().find(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) == needle
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
        };
    };
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": row}),
    }
}

fn upload_delete_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let mut state = load_parity_state(root);
    let rows = as_array_mut(&mut state, "uploads");
    let mut removed = None;
    rows.retain(|row| {
        let keep = clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != needle;
        if !keep {
            removed = row
                .get("path")
                .and_then(Value::as_str)
                .map(|v| v.to_string());
        }
        keep
    });
    if let Some(path) = removed {
        let _ = fs::remove_file(path);
        save_parity_state(root, state);
        return CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "deleted": true, "upload_id": needle}),
        };
    }
    CompatApiResponse {
        status: 404,
        payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
    }
}

fn recent_log_events(root: &Path, limit: usize) -> Vec<Value> {
    let mut rows = fs::read_to_string(state_path(root, ACTION_HISTORY_REL))
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn stream_payload(root: &Path, stream: &str) -> CompatApiResponse {
    let events = recent_log_events(root, 60);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "stream": clean_text(stream, 80),
            "events": events,
            "count": events.len(),
            "cursor": stable_hash(&format!("{}|{}", stream, now_ms()), 12)
        }),
    }
}

fn handle_marketplace_aliases(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    snapshot: &Value,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "GET"
        && (path_only == "/api/marketplace" || path_only == "/api/marketplace/browse")
    {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("limit=25");
        let forwarded = format!("/api/clawhub/browse?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" && path_only == "/api/marketplace/search" {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("q=");
        let forwarded = format!("/api/clawhub/search?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" {
        if let Some(slug) = path_only.strip_prefix("/api/marketplace/skill/") {
            let forwarded = format!("/api/clawhub/skill/{}", clean_text(slug, 160));
            return super::dashboard_skills_marketplace::handle(
                root,
                "GET",
                &forwarded,
                snapshot,
                &[],
            );
        }
    }
    if method == "POST" && path_only == "/api/skills/install" {
        return super::dashboard_skills_marketplace::handle(
            root,
            "POST",
            "/api/clawhub/install",
            snapshot,
            body,
        );
    }
    if method == "POST" && path_only == "/api/skills/reload" {
        if let Some(listed) =
            super::dashboard_skills_marketplace::handle(root, "GET", "/api/skills", snapshot, &[])
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "reloaded": true, "skills": listed.payload.get("skills").cloned().unwrap_or_else(|| Value::Array(Vec::new()))}),
            });
        }
    }
    None
}
