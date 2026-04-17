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

fn install_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let slug = normalize_name(request.get("slug").and_then(Value::as_str).unwrap_or(""));
    if slug.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "slug_required"}),
        };
    }
    let Some(skill) = marketplace_catalog()
        .into_iter()
        .find(|row| normalize_name(row.get("slug").and_then(Value::as_str).unwrap_or("")) == slug)
    else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "skill_not_found"}),
        };
    };

    let installed_already = merged_installed_rows(root).into_iter().any(|row| {
        normalize_name(row.get("name").and_then(Value::as_str).unwrap_or("")) == slug
            || row
                .get("source")
                .and_then(Value::as_object)
                .and_then(|src| src.get("slug"))
                .and_then(Value::as_str)
                .map(|v| normalize_name(v) == slug)
                .unwrap_or(false)
    });
    if installed_already {
        return CompatApiResponse {
            status: 409,
            payload: json!({"ok": false, "error": "already_installed"}),
        };
    }
    let installed_row = json!({
        "name": skill.get("name").cloned().unwrap_or_else(|| Value::String(slug.clone())),
        "description": skill.get("description").cloned().unwrap_or_else(|| Value::String(String::new())),
        "version": clean_text(skill.get("version").and_then(Value::as_str).unwrap_or("v1"), 40),
        "author": skill.get("author").cloned().unwrap_or_else(|| Value::String("Infring".to_string())),
        "runtime": skill.get("runtime").cloned().unwrap_or_else(|| Value::String("prompt_only".to_string())),
        "tools_count": 0,
        "tags": skill.get("tags").cloned().unwrap_or_else(|| Value::Array(default_tags())),
        "enabled": true,
        "has_prompt_context": true,
        "prompt_context": clean_text(skill.get("prompt_context").and_then(Value::as_str).unwrap_or(""), 4000),
        "source": {"type":"clawhub","slug": slug.clone()},
        "installed_at": crate::now_iso()
    });

    let mut state = load_dashboard_state(root);
    let installed = as_object_mut(&mut state, "installed");
    installed.insert(slug.clone(), installed_row.clone());
    save_dashboard_state(root, state);
    upsert_core_installed_skill(root, &slug, &installed_row);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "name": skill.get("name").cloned().unwrap_or_else(|| Value::String(slug)), "warnings": []}),
    }
}

fn uninstall_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let name = normalize_name(request.get("name").and_then(Value::as_str).unwrap_or(""));
    if name.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "name_required"}),
        };
    }
    let mut state = load_dashboard_state(root);
    {
        let installed = as_object_mut(&mut state, "installed");
        installed.remove(&name);
    }
    {
        let created = as_object_mut(&mut state, "created");
        created.remove(&name);
    }
    save_dashboard_state(root, state);
    remove_core_installed_skill(root, &name);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true}),
    }
}

fn create_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let name_raw = request.get("name").and_then(Value::as_str).unwrap_or("");
    let name = normalize_name(name_raw);
    if name.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "name_required"}),
        };
    }
    let created_row = json!({
        "name": name.clone(),
        "description": clean_text(request.get("description").and_then(Value::as_str).unwrap_or("User-created prompt skill"), 300),
        "version": "v1",
        "author": "User",
        "runtime": clean_text(request.get("runtime").and_then(Value::as_str).unwrap_or("prompt_only"), 40),
        "tools_count": 0,
        "tags": request.get("tags").cloned().filter(|v| v.is_array()).unwrap_or_else(|| Value::Array(default_tags())),
        "enabled": true,
        "has_prompt_context": true,
        "source": {"type":"local"},
        "prompt_context": clean_text(request.get("prompt_context").and_then(Value::as_str).unwrap_or(""), 4000),
        "created_at": crate::now_iso()
    });
    let mut state = load_dashboard_state(root);
    let created = as_object_mut(&mut state, "created");
    created.insert(name.clone(), created_row.clone());
    save_dashboard_state(root, state);
    upsert_core_installed_skill(root, &name, &created_row);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true}),
    }
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    snapshot: &Value,
    body: &[u8],
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    if method == "GET" {
        if path_only == "/api/skills" {
            return Some(CompatApiResponse {
                status: 200,
                payload: list_skills_payload(root),
            });
        }
        if path_only == "/api/mcp/servers" {
            return Some(CompatApiResponse {
                status: 200,
                payload: mcp_servers_payload(snapshot),
            });
        }
        if path_only == "/api/clawhub/browse" {
            return Some(CompatApiResponse {
                status: 200,
                payload: browse_payload(path),
            });
        }
        if path_only == "/api/clawhub/search" {
            return Some(CompatApiResponse {
                status: 200,
                payload: search_payload(path),
            });
        }
        if let Some(slug) = path_only.strip_prefix("/api/clawhub/skill/") {
            if let Some(clean_slug) = slug.strip_suffix("/code") {
                return Some(detail_code_payload(clean_slug));
            }
            return Some(detail_payload(root, slug));
        }
    }
    if method == "POST" {
        if path_only == "/api/clawhub/install" {
            return Some(install_payload(root, body));
        }
        if path_only == "/api/skills/uninstall" {
            return Some(uninstall_payload(root, body));
        }
        if path_only == "/api/skills/create" {
            return Some(create_payload(root, body));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_and_search_are_paginated() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(
            root.path(),
            "GET",
            "/api/clawhub/browse?sort=downloads&limit=5",
            &json!({}),
            &[],
        )
        .expect("browse");
        let rows = browse
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 5);
        let search = handle(
            root.path(),
            "GET",
            "/api/clawhub/search?q=router&limit=10",
            &json!({}),
            &[],
        )
        .expect("search");
        let search_rows = search
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(search_rows.iter().any(|row| {
            row.get("slug")
                .and_then(Value::as_str)
                .map(|v| v.contains("router"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn install_create_uninstall_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let installed = handle(
            root.path(),
            "POST",
            "/api/clawhub/install",
            &json!({}),
            br#"{"slug":"model-router-pro"}"#,
        )
        .expect("install");
        assert_eq!(installed.status, 200);
        let listed = handle(root.path(), "GET", "/api/skills", &json!({}), &[]).expect("skills");
        let rows = listed
            .payload
            .get("skills")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "model-router-pro")
                .unwrap_or(false)
        }));
        let core_registry = read_json(&root.path().join(CORE_SKILLS_REGISTRY_REL))
            .expect("core registry after install");
        let core_prompt = core_registry
            .get("installed")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get("model-router-pro"))
            .and_then(|row| row.get("prompt_context"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            !core_prompt.is_empty(),
            "core registry should persist prompt context"
        );

        let created = handle(
            root.path(),
            "POST",
            "/api/skills/create",
            &json!({}),
            br#"{"name":"my-demo-skill","description":"demo","runtime":"prompt_only","prompt_context":"ctx"}"#,
