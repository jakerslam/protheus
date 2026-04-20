
fn parse_structure_surface(root: &Path) -> Value {
    let mut extension_counts = BTreeMap::<String, u64>::new();
    let mut total_files: u64 = 0;
    let mut scanned: usize = 0;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if scanned >= RECON_MAX_FILES {
            break;
        }
        scanned += 1;
        if !entry.file_type().is_file() {
            continue;
        }
        total_files += 1;
        let ext = entry
            .path()
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let key = if ext.is_empty() {
            "no_ext".to_string()
        } else {
            ext
        };
        *extension_counts.entry(key).or_insert(0) += 1;
    }
    let top_extensions = extension_counts
        .iter()
        .map(|(ext, count)| json!({ "extension": ext, "count": count }))
        .collect::<Vec<_>>();
    json!({
        "total_files": total_files,
        "scanned_entries": scanned,
        "top_extensions": top_extensions
    })
}

fn build_dependency_closure(manifest_inventory: &[Value]) -> (Vec<String>, Vec<Value>, Value) {
    let mut dependency_hints = BTreeSet::<String>::new();
    let mut package_index = BTreeMap::<String, (String, String)>::new();
    for row in manifest_inventory {
        let package_name = row
            .get("package_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let path = row
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if !package_name.is_empty() && !path.is_empty() {
            package_index.insert(
                normalize_dependency_token(package_name),
                (package_name.to_string(), path),
            );
        }
        if let Some(hints) = row.get("dependency_hints").and_then(Value::as_array) {
            for hint in hints {
                if let Some(text) = hint.as_str() {
                    let cleaned = text.trim();
                    if !cleaned.is_empty() {
                        dependency_hints.insert(cleaned.to_string());
                    }
                }
            }
        }
    }
    let mut edges = Vec::<Value>::new();
    let mut edge_seen = BTreeSet::<String>::new();
    let mut internal_edge_count: usize = 0;
    let mut external_edge_count: usize = 0;
    for row in manifest_inventory {
        let source_path = row
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if source_path.is_empty() {
            continue;
        }
        let source_kind = row
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let source_package = row
            .get("package_name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let Some(hints) = row.get("dependency_hints").and_then(Value::as_array) else {
            continue;
        };
        for hint in hints {
            let Some(dep_raw) = hint.as_str() else {
                continue;
            };
            let dep = dep_raw.trim();
            if dep.is_empty() {
                continue;
            }
            let key = format!("{source_path}::{dep}");
            if !edge_seen.insert(key) {
                continue;
            }
            let token = normalize_dependency_token(dep);
            let target = package_index.get(&token);
            let relation = if target.is_some() {
                internal_edge_count += 1;
                "internal_manifest_dependency"
            } else {
                external_edge_count += 1;
                "external_dependency_hint"
            };
            let target_package = target.map(|row| row.0.clone());
            let target_manifest = target.map(|row| row.1.clone());
            edges.push(json!({
                "id": dep,
                "source": "manifest_hint",
                "source_manifest": source_path,
                "source_kind": source_kind,
                "source_package": source_package,
                "relation": relation,
                "target_package": target_package,
                "target_manifest": target_manifest
            }));
        }
    }
    if edges.is_empty() {
        edges = dependency_hints
            .iter()
            .take(200)
            .map(|hint| {
                json!({
                    "id": hint,
                    "source": "manifest_hint",
                    "relation": "unresolved_dependency_hint"
                })
            })
            .collect::<Vec<_>>();
    }
    let summary = json!({
        "manifest_node_count": manifest_inventory.len(),
        "package_node_count": package_index.len(),
        "edge_count": edges.len(),
        "internal_edge_count": internal_edge_count,
        "external_edge_count": external_edge_count
    });
    (
        dependency_hints.into_iter().collect::<Vec<_>>(),
        edges,
        summary,
    )
}

fn framework_targets_from_hints(hints: &[String]) -> Vec<String> {
    let mut out = BTreeSet::<String>::new();
    for hint in hints {
        let normalized = hint.to_ascii_lowercase();
        if normalized.contains("langgraph") {
            out.insert("workflow://langgraph".to_string());
        }
        if normalized.contains("openai-agents") || normalized.contains("openai_agents") {
            out.insert("workflow://openai-agents".to_string());
        }
        if normalized.contains("crewai") {
            out.insert("workflow://crewai".to_string());
        }
        if normalized.contains("mastra") {
            out.insert("workflow://mastra".to_string());
        }
        if normalized.contains("llamaindex") || normalized.contains("llama-index") {
            out.insert("workflow://llamaindex".to_string());
        }
        if normalized.contains("haystack") {
            out.insert("workflow://haystack".to_string());
        }
        if normalized.contains("dspy") {
            out.insert("workflow://dspy".to_string());
        }
        if normalized.contains("pydantic-ai") || normalized.contains("pydantic_ai") {
            out.insert("workflow://pydantic-ai".to_string());
        }
        if normalized.contains("camel-ai") || normalized == "camel" {
            out.insert("workflow://camel".to_string());
        }
        if normalized.contains("google-adk") || normalized.contains("google_adk") {
            out.insert("workflow://google-adk".to_string());
        }
    }
    out.into_iter().collect::<Vec<_>>()
}
