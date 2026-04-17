fn resolve_local_spec(from_file: &Path, spec: &str) -> Option<PathBuf> {
    let spec = spec
        .split(['?', '#'])
        .next()
        .map(str::trim)
        .unwrap_or("");
    if spec.is_empty() || spec.starts_with('/') || spec.starts_with("file:") {
        return None;
    }
    let base = from_file
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(spec);
    let mut stem_candidates = Vec::<PathBuf>::new();
    if let Some(raw_name) = base.file_name().and_then(|row| row.to_str()) {
        if let Some(stem) = raw_name.strip_suffix(".js") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".ts") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".mjs") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".cjs") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".mts") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".cts") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".tsx") {
            stem_candidates.push(base.with_file_name(stem));
        }
        if let Some(stem) = raw_name.strip_suffix(".jsx") {
            stem_candidates.push(base.with_file_name(stem));
        }
    }
    let mut candidates = vec![
        base.clone(),
        PathBuf::from(format!("{}.ts", base.display())),
        PathBuf::from(format!("{}.tsx", base.display())),
        PathBuf::from(format!("{}.mts", base.display())),
        PathBuf::from(format!("{}.cts", base.display())),
        PathBuf::from(format!("{}.js", base.display())),
        PathBuf::from(format!("{}.jsx", base.display())),
        PathBuf::from(format!("{}.mjs", base.display())),
        PathBuf::from(format!("{}.cjs", base.display())),
    ];
    for index_name in [
        "index.ts",
        "index.tsx",
        "index.mts",
        "index.cts",
        "index.js",
        "index.jsx",
        "index.mjs",
        "index.cjs",
    ] {
        candidates.push(base.join(index_name));
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    for stem in stem_candidates {
        let stem_checks = [
            stem.clone(),
            PathBuf::from(format!("{}.ts", stem.display())),
            PathBuf::from(format!("{}.tsx", stem.display())),
            PathBuf::from(format!("{}.mts", stem.display())),
            PathBuf::from(format!("{}.cts", stem.display())),
            PathBuf::from(format!("{}.js", stem.display())),
            PathBuf::from(format!("{}.jsx", stem.display())),
            PathBuf::from(format!("{}.mjs", stem.display())),
            PathBuf::from(format!("{}.cjs", stem.display())),
            stem.join("index.ts"),
            stem.join("index.tsx"),
            stem.join("index.mts"),
            stem.join("index.cts"),
            stem.join("index.js"),
            stem.join("index.jsx"),
            stem.join("index.mjs"),
            stem.join("index.cjs"),
        ];
        for candidate in stem_checks {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn parse_import_specs(source: &str) -> Vec<String> {
    let mut specs = Vec::<String>::new();
    for (needle, quote) in [
        ("from '", '\''),
        ("from \"", '"'),
        ("import('", '\''),
        ("import(\"", '"'),
        ("require('", '\''),
        ("require(\"", '"'),
    ] {
        let mut offset = 0usize;
        while let Some(idx) = source[offset..].find(needle) {
            let start = offset + idx + needle.len();
            let Some(end_rel) = source[start..].find(quote) else {
                break;
            };
            let spec = source[start..start + end_rel].trim();
            if !spec.is_empty() {
                specs.push(spec.to_string());
            }
            offset = start + end_rel + 1;
        }
    }
    specs
}

fn run_dependency_boundary_check_native(root: &Path, policy_path: &Path) -> Result<Value, String> {
    let policy = read_json(policy_path)?;
    let scan = policy.get("scan").cloned().unwrap_or_else(|| json!({}));
    let include_dirs = json_string_vec(scan.get("include_dirs"));
    let include_ext = json_string_vec(scan.get("include_ext"))
        .into_iter()
        .map(|v| v.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let exclude_contains = json_string_vec(scan.get("exclude_contains"));
    let layers = json_string_map(policy.get("layers"));
    let allow_imports = json_string_map(policy.get("allow_imports"));
    let enforce_layers = json_string_vec(policy.get("enforce_layers"))
        .into_iter()
        .collect::<BTreeSet<_>>();

    let conduit = policy
        .get("conduit_boundary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let conduit_dirs = json_string_vec(conduit.get("include_dirs"));
    let conduit_ext = json_string_vec(conduit.get("include_ext"))
        .into_iter()
        .map(|v| v.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let conduit_excludes = json_string_vec(conduit.get("exclude_contains"));
    let conduit_allow = json_string_vec(conduit.get("allowlisted_files"))
        .into_iter()
        .collect::<BTreeSet<_>>();
    let forbidden_patterns = json_string_vec(conduit.get("forbidden_patterns"));

    let files = list_boundary_files(root, &include_dirs, &include_ext, &exclude_contains)?;
    let conduit_roots = conduit_dirs
        .iter()
        .map(|v| normalize_rel(v).trim_end_matches('/').to_string())
        .collect::<Vec<_>>();

    let mut layer_violations = Vec::<Value>::new();
    let mut conduit_violations = Vec::<Value>::new();
    let mut missing_local_imports = Vec::<Value>::new();

    for file_path in &files {
        let rel_path = normalize_rel(
            &file_path
                .strip_prefix(root)
                .unwrap_or(file_path)
                .to_string_lossy(),
        );
        let source = fs::read_to_string(file_path).map_err(|err| {
            format!(
                "dependency_boundary_read_source_failed:{}:{err}",
                file_path.display()
            )
        })?;
        let source_layer = detect_layer(&rel_path, &layers);

        for spec in parse_import_specs(&source) {
            if !spec.starts_with('.') {
                continue;
            }
            let Some(resolved) = resolve_local_spec(file_path, &spec) else {
                missing_local_imports.push(json!({
                    "file": rel_path,
                    "spec": spec
                }));
                continue;
            };
            let target_rel = normalize_rel(
                &resolved
                    .strip_prefix(root)
                    .unwrap_or(&resolved)
                    .to_string_lossy(),
            );
            let target_layer = detect_layer(&target_rel, &layers);
            let Some(source_layer) = source_layer.clone() else {
                continue;
            };
            let Some(target_layer) = target_layer else {
                continue;
            };
            if !enforce_layers.contains(&source_layer) {
                continue;
            }
            let allowed = allow_imports
                .get(&source_layer)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeSet<_>>();
            if !allowed.contains(&target_layer) {
                layer_violations.push(json!({
                    "file": rel_path,
                    "source_layer": source_layer,
                    "spec": spec,
                    "resolved": target_rel,
                    "target_layer": target_layer
                }));
            }
        }

        let ext = file_path
            .extension()
            .map(|v| format!(".{}", v.to_string_lossy().to_ascii_lowercase()))
            .unwrap_or_default();
        let in_conduit_scope = conduit_roots
            .iter()
            .any(|root| rel_path == *root || rel_path.starts_with(&format!("{root}/")))
            && conduit_ext.contains(&ext)
            && !conduit_excludes
                .iter()
                .any(|token| rel_path.contains(token));

        if !in_conduit_scope || conduit_allow.contains(&rel_path) {
            continue;
        }
        for token in &forbidden_patterns {
            if !token.is_empty() && source.contains(token) {
                conduit_violations.push(json!({
                    "file": rel_path,
                    "forbidden_pattern": token
                }));
            }
        }
    }

    let ok = layer_violations.is_empty()
        && conduit_violations.is_empty()
        && missing_local_imports.is_empty();

    Ok(json!({
        "ok": ok,
        "type": "dependency_boundary_guard",
        "ts": now_iso(),
        "strict": true,
        "scanned_files": files.len(),
        "layer_violations": layer_violations,
        "conduit_violations": conduit_violations,
        "missing_local_imports": missing_local_imports
    }))
}

fn bool_from_path(value: &Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for key in path {
        let Some(next) = cursor.get(*key) else {
            return false;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(false)
}

fn string_from_path(value: &Value, path: &[&str]) -> Option<String> {
    let mut cursor = value;
    for key in path {
        let Some(next) = cursor.get(*key) else {
            return None;
        };
        cursor = next;
    }
    cursor
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn check_constitution_contract(root: &Path, policy: &OriginIntegrityPolicy) -> Value {
    let constitution_path = resolve_path(root, &policy.constitution.constitution_path);
    let guardian_policy_path = resolve_path(root, &policy.constitution.guardian_policy_path);
    let rsi_policy_path = resolve_path(root, &policy.constitution.rsi_bootstrap_policy_path);

    let constitution_exists = constitution_path.is_file();
    let constitution_hash = if constitution_exists {
        sha256_file(&constitution_path).ok()
    } else {
        None
    };

    let guardian = read_json(&guardian_policy_path).unwrap_or_else(|_| json!({}));
    let require_dual_approval = bool_from_path(&guardian, &["require_dual_approval"]);
    let require_emergency_approval =
        bool_from_path(&guardian, &["emergency_rollback_requires_approval"]);

    let rsi = read_json(&rsi_policy_path).unwrap_or_else(|_| json!({}));
    let require_constitution_status =
        bool_from_path(&rsi, &["gating", "require_constitution_status"]);
    let merkle_path = string_from_path(&rsi, &["paths", "merkle_path"]);
    let resurrection_script = string_from_path(&rsi, &["scripts", "continuity_resurrection"]);
    let resurrection_exists = resurrection_script
        .as_ref()
        .map(|rel| resolve_path(root, rel).exists())
        .unwrap_or(false);

    let ok = constitution_exists
        && require_dual_approval
        && require_emergency_approval
        && require_constitution_status
        && merkle_path.is_some()
        && resurrection_script.is_some()
        && resurrection_exists;

    json!({
        "ok": ok,
        "constitution_path": normalize_rel(&policy.constitution.constitution_path),
        "constitution_exists": constitution_exists,
        "constitution_hash": constitution_hash,
        "guardian_policy_path": normalize_rel(&policy.constitution.guardian_policy_path),
        "guardian_require_dual_approval": require_dual_approval,
        "guardian_emergency_rollback_requires_approval": require_emergency_approval,
        "rsi_bootstrap_policy_path": normalize_rel(&policy.constitution.rsi_bootstrap_policy_path),
        "rsi_require_constitution_status": require_constitution_status,
        "rsi_merkle_path": merkle_path,
        "rsi_resurrection_script": resurrection_script,
        "rsi_resurrection_script_exists": resurrection_exists
    })
}

fn evaluate_invariants(root: &Path, policy: &OriginIntegrityPolicy, command: &str) -> Value {
    let safety_plane = collect_safety_plane_state(root, policy);
    let conduit_only = run_dependency_boundary_check(root, policy);
    let constitution = check_constitution_contract(root, policy);

    let safety_hash = safety_plane
        .get("state_hash")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let binding_material = json!({
        "safety_plane_state_hash": safety_hash,
        "conduit_only_ok": conduit_only.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "constitution_ok": constitution.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "command": command
    });
    let state_binding_hash = deterministic_receipt_hash(&binding_material);

    let ok = conduit_only.get("ok").and_then(Value::as_bool) == Some(true)
        && constitution.get("ok").and_then(Value::as_bool) == Some(true)
        && safety_plane
            .get("missing_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0;

    let mut out = json!({
        "ok": ok,
        "type": "origin_integrity_enforcer",
        "ts": now_iso(),
        "command": command,
        "policy_version": policy.version,
        "checks": {
            "conduit_only": conduit_only,
            "constitution": constitution
        },
        "safety_plane": safety_plane,
        "state_binding": {
            "safety_plane_state_hash": safety_hash,
            "state_binding_hash": state_binding_hash
        },
        "claim_evidence": [
            {
                "id": "conduit_only_enforced",
                "claim": "conduit is the only allowed client-to-core path",
                "evidence": {
                    "check_ok": binding_material.get("conduit_only_ok").cloned().unwrap_or(Value::Bool(false))
                }
            },
            {
                "id": "constitution_non_weakening_guard",
                "claim": "constitution weakening requires guardian + resurrection lanes",
                "evidence": {
                    "check_ok": binding_material.get("constitution_ok").cloned().unwrap_or(Value::Bool(false))
                }
            },
            {
                "id": "receipt_state_binding",
                "claim": "receipt is cryptographically bound to safety-plane state",
                "evidence": binding_material
            }
        ],
        "persona_lenses": {
            "guardian": {
                "origin_integrity_ok": ok,
                "safety_plane_hash": safety_hash
            }
        }
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn build_certificate(root: &Path, policy: &OriginIntegrityPolicy, run_receipt: &Value) -> Value {
    let verify_script_path = resolve_path(root, &policy.verify_script_relpath);
    let verify_sha = if verify_script_path.is_file() {
        sha256_file(&verify_script_path).ok()
    } else {
        None
    };

    let safety_hash = run_receipt
        .get("state_binding")
        .and_then(|v| v.get("safety_plane_state_hash"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let mut out = json!({
        "ok": run_receipt.get("ok").and_then(Value::as_bool) == Some(true) && verify_sha.is_some(),
        "type": "origin_verify_certificate",
        "ts": now_iso(),
        "verify_script": normalize_rel(&policy.verify_script_relpath),
        "verify_script_sha256": verify_sha,
        "safety_plane_state_hash": safety_hash,
        "origin_integrity_receipt_hash": run_receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "policy_version": policy.version
    });
    out["certificate_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
