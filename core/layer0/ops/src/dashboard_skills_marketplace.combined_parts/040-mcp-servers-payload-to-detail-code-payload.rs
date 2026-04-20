
fn mcp_servers_payload(snapshot: &Value) -> Value {
    let raw = snapshot
        .pointer("/skills/upstream/mcp_servers")
        .cloned()
        .unwrap_or_else(|| json!([]));
    if raw.get("configured").is_some() && raw.get("connected").is_some() {
        let configured = raw
            .get("configured")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let connected = raw
            .get("connected")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut servers = connected.clone();
        servers.extend(configured.clone());
        let total_configured = raw
            .get("total_configured")
            .and_then(Value::as_u64)
            .unwrap_or(configured.len() as u64);
        let total_connected = raw
            .get("total_connected")
            .and_then(Value::as_u64)
            .unwrap_or(connected.len() as u64);
        return json!({
            "configured": configured,
            "connected": connected,
            "servers": servers,
            "total_configured": total_configured,
            "total_connected": total_connected
        });
    }
    let rows = raw.as_array().cloned().unwrap_or_default();
    let connected = rows
        .iter()
        .filter(|row| {
            row.get("connected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || row
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case("connected"))
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let configured = rows
        .iter()
        .filter(|row| {
            !row.get("connected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|v| v.eq_ignore_ascii_case("connected"))
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "configured": configured,
        "connected": connected,
        "servers": rows,
        "total_configured": configured.len(),
        "total_connected": connected.len()
    })
}

fn browse_payload(path: &str) -> Value {
    let query = parse_query(path);
    let sort = clean_text(
        query
            .get("sort")
            .and_then(Value::as_str)
            .unwrap_or("trending"),
        40,
    )
    .to_lowercase();
    let mut rows = marketplace_catalog();
    rows.sort_by(|a, b| match sort.as_str() {
        "downloads" => parse_u64(b.get("downloads"), 0).cmp(&parse_u64(a.get("downloads"), 0)),
        "stars" => parse_u64(b.get("stars"), 0).cmp(&parse_u64(a.get("stars"), 0)),
        "updated" => clean_text(
            b.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            40,
        )
        .cmp(&clean_text(
            a.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            40,
        )),
        _ => {
            let score_a = parse_u64(a.get("downloads"), 0) + (parse_u64(a.get("stars"), 0) * 4);
            let score_b = parse_u64(b.get("downloads"), 0) + (parse_u64(b.get("stars"), 0) * 4);
            score_b.cmp(&score_a)
        }
    });
    paginate(rows, &query)
}

fn search_payload(path: &str) -> Value {
    let query = parse_query(path);
    let q = clean_text(query.get("q").and_then(Value::as_str).unwrap_or(""), 120).to_lowercase();
    let mut rows = marketplace_catalog();
    if !q.is_empty() {
        rows.retain(|row| {
            let tags = row
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(" ");
            let haystack = format!(
                "{} {} {} {}",
                row.get("slug").and_then(Value::as_str).unwrap_or(""),
                row.get("name").and_then(Value::as_str).unwrap_or(""),
                row.get("description").and_then(Value::as_str).unwrap_or(""),
                tags
            )
            .to_lowercase();
            haystack.contains(&q)
        });
    }
    paginate(rows, &query)
}

fn detail_payload(root: &Path, slug: &str) -> CompatApiResponse {
    let normalized = normalize_name(slug);
    let rows = marketplace_catalog();
    let Some(mut detail) = rows.into_iter().find(|row| {
        normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == normalized
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };
    let installed = merged_installed_rows(root).into_iter().any(|row| {
        row.get("source")
            .and_then(Value::as_object)
            .and_then(|src| src.get("slug"))
            .and_then(Value::as_str)
            .map(|v| normalize_name(v) == normalized)
            .unwrap_or(false)
            || normalize_name(row.get("name").and_then(Value::as_str).unwrap_or("")) == normalized
    });
    detail["installed"] = Value::Bool(installed);
    CompatApiResponse {
        status: 200,
        payload: detail,
    }
}

fn detail_code_payload(slug: &str) -> CompatApiResponse {
    let normalized = normalize_name(slug);
    let rows = marketplace_catalog();
    let Some(detail) = rows.into_iter().find(|row| {
        normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == normalized
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };
    let code = format!(
        "[skill]\nname = \"{}\"\nruntime = \"{}\"\ndescription = \"{}\"\n\n[prompt]\ncontext = \"{}\"\n",
        detail.get("name").and_then(Value::as_str).unwrap_or("unknown"),
        detail
            .get("runtime")
            .and_then(Value::as_str)
            .unwrap_or("prompt_only"),
        detail
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        detail
            .get("prompt_context")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "filename": format!("{}.toml", normalized),
            "code": code
        }),
    }
}
