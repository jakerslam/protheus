fn run_execution_profiles(root: &Path, strict: bool) -> Value {
    let matrix = read_json(&root.join(EXECUTION_PROFILE_MATRIX_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if matrix
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("execution_profile_matrix_version_must_be_v1".to_string());
    }
    if matrix
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "execution_profile_matrix"
    {
        errors.push("execution_profile_matrix_kind_must_be_execution_profile_matrix".to_string());
    }
    let profiles = matrix
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut ids = BTreeSet::new();
    let mut profile_harnesses: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for profile in profiles {
        let id = profile
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() {
            errors.push("execution_profile_id_required".to_string());
            continue;
        }
        let id_norm = id.to_ascii_lowercase();
        if !is_token_id(&id_norm) {
            errors.push("execution_profile_id_invalid".to_string());
        }
        if !ids.insert(id_norm.clone()) {
            errors.push("execution_profile_duplicate_id".to_string());
        }
        let harness = profile
            .get("harness")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if harness.is_empty() {
            errors.push("execution_profile_harness_required".to_string());
        }
        if !harness.is_empty() && (!harness.starts_with("harness/") || harness.len() < 10) {
            errors.push("execution_profile_harness_invalid".to_string());
        }
        let determinism = profile
            .get("determinism")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(determinism, "high" | "medium" | "low") {
            errors.push("execution_profile_determinism_invalid".to_string());
        }
        profile_harnesses.insert(id_norm, harness);
    }
    for req in ["mcu", "edge", "cloud"] {
        if !ids.contains(req) {
            errors.push(format!("execution_profile_missing::{req}"));
        }
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "matrix_path": EXECUTION_PROFILE_MATRIX_PATH,
        "profile_harnesses": profile_harnesses,
        "errors": errors
    })
}

fn run_variant_profiles(root: &Path, strict: bool) -> Value {
    let mut errors = Vec::new();
    let mut profile_rows = Vec::new();
    let required_profiles = ["medical", "robotics", "ai_isolation", "riscv_sovereign"];

    for profile_id in required_profiles {
        let rel = format!("{VARIANT_PROFILE_DIR}/{profile_id}.json");
        let path = root.join(&rel);
        let payload = read_json(&path).unwrap_or(Value::Null);
        let mut profile_errors = Vec::new();

        if payload.is_null() {
            profile_errors.push("variant_profile_missing_or_invalid".to_string());
        }
        if payload
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "v1"
        {
            profile_errors.push("variant_profile_version_must_be_v1".to_string());
        }
        if payload
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "layer_minus_one_variant_profile"
        {
            profile_errors.push("variant_profile_kind_invalid".to_string());
        }
        if payload
            .get("profile_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != profile_id
        {
            profile_errors.push("variant_profile_id_mismatch".to_string());
        }
        let baseline_ref = payload
            .get("baseline_policy_ref")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if baseline_ref.is_empty() {
            profile_errors.push("variant_profile_baseline_policy_ref_required".to_string());
        }
        let no_privilege_widening = payload
            .get("no_privilege_widening")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !no_privilege_widening {
            profile_errors.push("variant_profile_no_privilege_widening_required".to_string());
        }

        let capability_delta = payload
            .get("capability_delta")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let grants: BTreeSet<String> = capability_delta
            .get("grant")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let revokes: BTreeSet<String> = capability_delta
            .get("revoke")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if grants.iter().any(|id| !is_token_id(id)) || revokes.iter().any(|id| !is_token_id(id)) {
            profile_errors.push("variant_profile_capability_delta_invalid_token".to_string());
        }
        let overlap: Vec<String> = grants.intersection(&revokes).cloned().collect();
        if !overlap.is_empty() {
            profile_errors.push("variant_profile_capability_delta_overlap".to_string());
        }

        let budget_delta = payload
            .get("budget_delta")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (k, v) in &budget_delta {
            if v.as_i64().is_none() {
                profile_errors.push(format!("variant_profile_budget_delta_invalid::{k}"));
            }
        }

        if !profile_errors.is_empty() {
            errors.extend(
                profile_errors
                    .iter()
                    .map(|err| format!("{profile_id}:{err}"))
                    .collect::<Vec<_>>(),
            );
        }

        profile_rows.push(json!({
            "profile_id": profile_id,
            "path": rel,
            "ok": profile_errors.is_empty(),
            "grants": grants.into_iter().collect::<Vec<_>>(),
            "revokes": revokes.into_iter().collect::<Vec<_>>(),
            "errors": profile_errors
        }));
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "variant_profile_dir": VARIANT_PROFILE_DIR,
        "required_profile_count": required_profiles.len(),
        "profiles": profile_rows,
        "errors": errors
    })
}

fn run_mpu_compartments(root: &Path, strict: bool) -> Value {
    let payload = read_json(&root.join(MPU_COMPARTMENT_PROFILE_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if payload
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mpu_compartment_profile_version_must_be_v1".to_string());
    }
    if payload
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mpu_compartment_profile"
    {
        errors.push("mpu_compartment_profile_kind_invalid".to_string());
    }

    let compartments = payload
        .get("compartments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if compartments.is_empty() {
        errors.push("mpu_compartments_required".to_string());
    }

    let mut ids = BTreeSet::new();
    let required_compartments: BTreeSet<String> = ["rtos_kernel", "conduit_io", "receipt_log"]
        .iter()
        .map(|v| v.to_string())
        .collect();
    let mut attenuation_hooks = Vec::new();
    let mut compartment_rows = Vec::new();
    for row in compartments {
        let id = row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let mut row_errors = Vec::new();
        if id.is_empty() {
            row_errors.push("mpu_compartment_id_required".to_string());
        } else {
            if !is_token_id(&id) {
                row_errors.push("mpu_compartment_id_invalid".to_string());
            }
            if !ids.insert(id.clone()) {
                row_errors.push("mpu_compartment_duplicate_id".to_string());
            }
        }
        let start_ok = row.get("region_start").and_then(Value::as_u64).unwrap_or(0) > 0;
        let size_ok = row.get("region_size").and_then(Value::as_u64).unwrap_or(0) > 0;
        if !start_ok || !size_ok {
            row_errors.push("mpu_compartment_region_invalid".to_string());
        }
        let access = row
            .get("access")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let read = access.get("read").and_then(Value::as_bool).unwrap_or(false);
        let write = access
            .get("write")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let execute = access
            .get("execute")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !(read || write || execute) {
            row_errors.push("mpu_compartment_access_empty".to_string());
        }
        if write && execute {
            row_errors.push("mpu_compartment_write_execute_forbidden".to_string());
        }
        let unprivileged = row
            .get("unprivileged")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !unprivileged {
            row_errors.push("mpu_compartment_unprivileged_required".to_string());
        }
        if !row_errors.is_empty() {
            errors.extend(
                row_errors
                    .iter()
                    .map(|err| format!("{id}:{err}"))
                    .collect::<Vec<_>>(),
            );
        }
        compartment_rows.push(json!({
            "id": id.clone(),
            "ok": row_errors.is_empty(),
            "read": read,
            "write": write,
            "execute": execute,
            "errors": row_errors
        }));
        if !id.is_empty() {
            let cpu_limit_pct = match id.as_str() {
                "rtos_kernel" => 60,
                "conduit_io" => 25,
                "receipt_log" => 15,
                _ => 20,
            };
            attenuation_hooks.push(json!({
                "compartment_id": id,
                "resource_limits": {
                    "cpu_limit_pct": cpu_limit_pct,
                    "memory_limit_bytes": row.get("region_size").and_then(Value::as_u64).unwrap_or(0),
                    "io_priority": if row.get("access").and_then(|v| v.get("write")).and_then(Value::as_bool).unwrap_or(false) { "normal" } else { "low" }
                },
                "hook": "metakernel_capability_attenuation"
            }));
        }
    }

    for required in &required_compartments {
        if !ids.contains(required) {
            errors.push(format!("mpu_compartment_required_missing::{required}"));
        }
    }

    let targets = payload
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if targets.is_empty() {
        errors.push("mpu_compartment_targets_required".to_string());
    }
    for target in targets {
        let target_id = target
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if target_id.is_empty() || !is_token_id(&target_id) {
            errors.push("mpu_compartment_target_id_invalid".to_string());
        }
        let target_compartments = target
            .get("compartments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if target_compartments.is_empty() {
            errors.push(format!("mpu_compartment_target_empty::{target_id}"));
            continue;
        }
        for comp in target_compartments {
            let id = comp
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if id.is_empty() || !ids.contains(&id) {
                errors.push(format!(
                    "mpu_compartment_target_unknown_compartment::{target_id}"
                ));
                break;
            }
        }
    }

    let top1_surface = "core/layer0/ops::metakernel";
    let top1_registry = read_json(&root.join(TOP1_SURFACE_REGISTRY_PATH)).unwrap_or(Value::Null);
    let top1_surface_present = top1_registry
        .get("surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(top1_surface));
    if !top1_surface_present {
        warnings.push("top1_surface_registry_missing::core/layer0/ops::metakernel".to_string());
    }

    let compartment_proof_links = required_compartments
        .iter()
        .map(|compartment| {
            json!({
                "compartment_id": compartment,
                "proof_surface_id": top1_surface,
                "registered": top1_surface_present,
                "present_in_profile": ids.contains(compartment)
            })
        })
        .collect::<Vec<_>>();

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "contract_path": MPU_COMPARTMENT_PROFILE_PATH,
        "compartment_count": ids.len(),
        "required_compartments": required_compartments.into_iter().collect::<Vec<_>>(),
        "compartments": compartment_rows,
        "capability_attenuation_hooks": attenuation_hooks,
        "top1_surface_registry": {
            "path": TOP1_SURFACE_REGISTRY_PATH,
            "surface_id": top1_surface,
            "surface_registered": top1_surface_present
        },
        "compartment_proof_links": compartment_proof_links,
        "errors": errors,
        "warnings": warnings
    })
}

fn run_manifest(root: &Path, strict: bool, manifest_rel: &str) -> Value {
    let registry = read_json(&root.join(REGISTRY_PATH)).unwrap_or(Value::Null);
    let primitives = gather_primitives_from_registry(&registry).unwrap_or_default();
    let valid: HashSet<String> = primitives.into_iter().collect();

    let schema_path = root.join(CELLBUNDLE_SCHEMA_PATH);
    let schema = read_json(&schema_path).unwrap_or(Value::Null);
    let schema_ok = schema
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .contains("CellBundle");

    let manifest_path = root.join(manifest_rel);
    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    let (manifest_ok, manifest_report) = validate_manifest_payload(&manifest, &valid, strict);

    let ok = schema_ok && manifest_ok;
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "schema_path": CELLBUNDLE_SCHEMA_PATH,
        "schema_ok": schema_ok,
        "manifest_path": manifest_rel,
        "manifest_ok": manifest_ok,
        "manifest_report": manifest_report
    })
}

