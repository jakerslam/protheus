
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
