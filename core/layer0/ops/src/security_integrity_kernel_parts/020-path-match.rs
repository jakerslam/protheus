
fn path_match(rel: &str, rule: &str) -> bool {
    let normalized = rule.trim().replace('\\', "/");
    if normalized.is_empty() {
        return false;
    }
    if let Some(prefix) = normalized.strip_suffix("/**") {
        rel.starts_with(prefix)
    } else {
        rel == normalized
    }
}

fn is_excluded(rel: &str, policy: &IntegrityPolicy) -> bool {
    policy
        .exclude_paths
        .iter()
        .any(|rule| path_match(rel, rule))
}

fn has_allowed_extension(rel: &str, policy: &IntegrityPolicy) -> bool {
    if policy.target_extensions.is_empty() {
        return true;
    }
    let ext = Path::new(rel)
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| format!(".{}", v.to_ascii_lowercase()))
        .unwrap_or_default();
    policy
        .target_extensions
        .iter()
        .any(|allowed| allowed == &ext)
}

fn sorted_hashes(hashes: &BTreeMap<String, String>) -> Value {
    let mut map = Map::new();
    for (key, value) in hashes {
        map.insert(key.clone(), Value::String(value.clone()));
    }
    Value::Object(map)
}

fn normalize_policy(runtime_root: &Path, raw: &Value) -> IntegrityPolicy {
    let obj = raw.as_object().cloned().unwrap_or_default();
    let target_roots = {
        let raw_roots = as_string_vec(obj.get("target_roots"));
        if raw_roots.is_empty() {
            vec![
                "systems/security".to_string(),
                "config/directives".to_string(),
            ]
        } else {
            raw_roots
                .into_iter()
                .map(|v| rel_from_runtime(runtime_root, &v))
                .collect()
        }
    };
    let target_extensions = {
        let raw_exts = as_string_vec(obj.get("target_extensions"));
        if raw_exts.is_empty() {
            vec![".js".to_string(), ".yaml".to_string(), ".yml".to_string()]
        } else {
            raw_exts
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect()
        }
    };
    let protected_files = {
        let raw_files = as_string_vec(obj.get("protected_files"));
        if raw_files.is_empty() {
            vec!["lib/directive_resolver.ts".to_string()]
        } else {
            raw_files
                .into_iter()
                .map(|v| rel_from_runtime(runtime_root, &v))
                .collect()
        }
    };
    let exclude_paths = as_string_vec(obj.get("exclude_paths"))
        .into_iter()
        .map(|v| rel_from_runtime(runtime_root, &v))
        .collect::<Vec<_>>();

    let mut hashes = BTreeMap::new();
    if let Some(Value::Object(raw_hashes)) = obj.get("hashes") {
        for (key, value) in raw_hashes {
            let rel = rel_from_runtime(runtime_root, key);
            if rel.is_empty() || rel.starts_with("../") {
                continue;
            }
            let digest = as_string(Some(value)).to_ascii_lowercase();
            if digest.is_empty() {
                continue;
            }
            hashes.insert(rel, digest);
        }
    }

    IntegrityPolicy {
        version: clean_text(obj.get("version"), 64).if_empty_then("1.0"),
        target_roots,
        target_extensions,
        protected_files,
        exclude_paths,
        hashes,
        sealed_at: Some(clean_text(obj.get("sealed_at"), 120)).filter(|v| !v.is_empty()),
        sealed_by: Some(clean_text(obj.get("sealed_by"), 120)).filter(|v| !v.is_empty()),
        last_approval_note: Some(clean_text(obj.get("last_approval_note"), 240))
            .filter(|v| !v.is_empty()),
    }
}

fn load_policy(runtime_root: &Path, policy_path: &Path) -> IntegrityPolicy {
    let raw = read_json_or_default(policy_path, json!({}));
    normalize_policy(runtime_root, &raw)
}

fn resolve_policy(
    runtime_root: &Path,
    policy_path: &Path,
    payload: Option<&Map<String, Value>>,
) -> IntegrityPolicy {
    if let Some(raw_policy) = payload.and_then(|map| map.get("policy")) {
        return normalize_policy(runtime_root, raw_policy);
    }
    load_policy(runtime_root, policy_path)
}

fn policy_to_value(policy: &IntegrityPolicy) -> Value {
    json!({
        "version": policy.version,
        "target_roots": policy.target_roots,
        "target_extensions": policy.target_extensions,
        "protected_files": policy.protected_files,
        "exclude_paths": policy.exclude_paths,
        "hashes": sorted_hashes(&policy.hashes),
        "sealed_at": policy.sealed_at,
        "sealed_by": policy.sealed_by,
        "last_approval_note": policy.last_approval_note
    })
}

fn collect_present_files(runtime_root: &Path, policy: &IntegrityPolicy) -> Vec<String> {
    let mut out = Vec::new();
    for root_rel in &policy.target_roots {
        let abs_root = runtime_root.join(root_rel);
        if !abs_root.exists() {
            continue;
        }
        for entry in WalkDir::new(&abs_root)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(runtime_root)
                .map(|v| v.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if rel.is_empty() || rel.starts_with("../") {
                continue;
            }
            if is_excluded(&rel, policy) || !has_allowed_extension(&rel, policy) {
                continue;
            }
            if !out.iter().any(|existing| existing == &rel) {
                out.push(rel);
            }
        }
    }
    for rel in &policy.protected_files {
        if is_excluded(rel, policy) {
            continue;
        }
        let abs = runtime_root.join(rel);
        if abs.is_file() && !out.iter().any(|existing| existing == rel) {
            out.push(rel.clone());
        }
    }
    out.sort();
    out
}

fn summarize_violations(violations: &[Value]) -> Value {
    let mut counts = BTreeMap::new();
    for violation in violations {
        let key = violation
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(key).or_insert(0u64) += 1;
    }
    let mut map = Map::new();
    for (key, value) in counts {
        map.insert(key, Value::from(value));
    }
    Value::Object(map)
}
