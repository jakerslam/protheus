fn parse_manifest_inventory(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 80 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(kind) = manifest_kind(path) else {
            continue;
        };
        let raw = fs::read(path).unwrap_or_default();
        let text = String::from_utf8_lossy(&raw).to_string();
        let package_name = parse_manifest_package_name(kind, &text);
        out.push(json!({
            "path": relative_or_absolute(root, path),
            "kind": kind,
            "content_hash": sha256_hex(&raw),
            "package_name": package_name,
            "dependency_hints": dependency_hints(kind, &text)
        }));
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

fn parse_license_surface(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut seen = BTreeSet::<String>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 24 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_license = name == "license"
            || name.starts_with("license.")
            || name == "copying"
            || name == "notice"
            || name == "security.md"
            || name == "license_scope.md";
        if !is_license {
            continue;
        }
        let path = relative_or_absolute(root, entry.path());
        if seen.insert(path.clone()) {
            out.push(json!({
                "path": path,
                "kind": "license_artifact"
            }));
        }
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

fn parse_test_surface(root: &Path) -> Value {
    let mut directories = BTreeSet::<String>::new();
    let mut sample_files = Vec::<String>::new();
    let mut test_file_count: u64 = 0;
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
        let path = entry.path();
        let rel = relative_or_absolute(root, path);
        if entry.file_type().is_dir() {
            let dir = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if dir == "tests" || dir == "__tests__" || dir == "spec" {
                directories.insert(rel);
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_test_file = name.ends_with("_test.rs")
            || name.ends_with("_test.py")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.tsx")
            || name.ends_with(".spec.js")
            || name.ends_with(".test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".test.js")
            || name.ends_with(".test.rs")
            || name.ends_with(".feature");
        if is_test_file {
            test_file_count += 1;
            if sample_files.len() < 20 {
                sample_files.push(rel);
            }
        }
    }
    sample_files.sort_unstable();
    json!({
        "directory_hints": directories.into_iter().collect::<Vec<_>>(),
        "test_file_count": test_file_count,
        "sample_files": sample_files,
        "scanned_entries": scanned
    })
}

fn parse_api_surface(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 30 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = if name.ends_with(".proto") {
            Some("proto")
        } else if name == "openapi.yaml" || name == "openapi.yml" || name == "openapi.json" {
            Some("openapi")
        } else if name.ends_with(".graphql") || name.ends_with(".gql") {
            Some("graphql")
        } else {
            None
        };
        if let Some(kind) = kind {
            out.push(json!({
                "path": relative_or_absolute(root, entry.path()),
                "kind": kind
            }));
        }
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

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

