
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
