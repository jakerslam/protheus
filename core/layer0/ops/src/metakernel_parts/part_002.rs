fn run_budget_admission(root: &Path, strict: bool, manifest_rel: &str) -> Value {
    let policy_path = root.join(BUDGET_ADMISSION_POLICY_PATH);
    let policy = read_json(&policy_path).unwrap_or(Value::Null);
    let hard_limits = policy
        .get("hard_limits")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required = [
        "cpu_ms",
        "ram_mb",
        "storage_mb",
        "network_kb",
        "tokens",
        "power_mw",
        "privacy_points",
        "cognitive_load",
    ];
    let mut policy_missing = Vec::new();
    for field in required {
        if parse_nonneg_i64_field(&hard_limits, field).is_none() {
            policy_missing.push(field.to_string());
        }
    }
    let fail_closed = policy
        .get("fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let policy_ok = policy_missing.is_empty() && fail_closed;

    let manifest_path = root.join(manifest_rel);
    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    let budgets = manifest
        .get("budgets")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut reason_codes = Vec::new();
    for field in required {
        let actual = parse_nonneg_i64_field(&budgets, field).unwrap_or(-1);
        let limit = parse_nonneg_i64_field(&hard_limits, field).unwrap_or(-1);
        if actual < 0 {
            reason_codes.push(format!("budget_missing::{field}"));
            continue;
        }
        if limit >= 0 && actual > limit {
            reason_codes.push(format!("budget_exceeded::{field}"));
        }
    }
    let admitted = policy_ok && reason_codes.is_empty();
    json!({
        "ok": if strict { admitted } else { true },
        "strict": strict,
        "policy_path": BUDGET_ADMISSION_POLICY_PATH,
        "policy_ok": policy_ok,
        "fail_closed": fail_closed,
        "policy_missing_fields": policy_missing,
        "manifest_path": manifest_rel,
        "admitted": admitted,
        "reason_codes": reason_codes
    })
}

fn run_epistemic_object(root: &Path, strict: bool, object_rel: &str) -> Value {
    let schema = read_json(&root.join(EPISTEMIC_OBJECT_SCHEMA_PATH)).unwrap_or(Value::Null);
    let schema_ok = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|v| {
            let set: BTreeSet<String> = v
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect();
            let required = [
                "value",
                "schema",
                "provenance",
                "confidence",
                "policy",
                "retention",
                "export",
                "rollback",
            ];
            required.iter().all(|k| set.contains(*k))
        })
        .unwrap_or(false);

    let object = read_json(&root.join(object_rel)).unwrap_or(Value::Null);
    let mut missing = Vec::new();
    for k in [
        "value",
        "schema",
        "provenance",
        "confidence",
        "policy",
        "retention",
        "export",
        "rollback",
    ] {
        if object.get(k).is_none() {
            missing.push(k.to_string());
        }
    }
    let confidence_ok = object
        .get("confidence")
        .and_then(Value::as_f64)
        .map(|v| (0.0..=1.0).contains(&v))
        .unwrap_or(false);
    let object_ok = missing.is_empty() && confidence_ok;

    let ok = schema_ok && object_ok;
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "schema_path": EPISTEMIC_OBJECT_SCHEMA_PATH,
        "schema_ok": schema_ok,
        "object_path": object_rel,
        "object_ok": object_ok,
        "missing_fields": missing,
        "confidence_ok": confidence_ok
    })
}

fn run_effect_journal(root: &Path, strict: bool, journal_rel: &str) -> Value {
    let policy = read_json(&root.join(EFFECT_JOURNAL_POLICY_PATH)).unwrap_or(Value::Null);
    let policy_ok = policy
        .get("commit_before_actuate_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let payload = read_json(&root.join(journal_rel)).unwrap_or(Value::Null);
    let entries = payload
        .get("journal_entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let effects = payload
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut entry_ids = BTreeSet::new();
    let mut entry_ts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut entry_errors = Vec::new();
    for entry in entries {
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let kind = entry
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let ts = entry
            .get("ts")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() || kind.is_empty() || ts.is_empty() {
            entry_errors.push("journal_entry_missing_required_fields".to_string());
            continue;
        }
        if !entry_ids.insert(id.clone()) {
            entry_errors.push("journal_entry_duplicate_id".to_string());
            continue;
        }
        entry_ts.insert(id, ts);
    }
    let mut effect_errors = Vec::new();
    for effect in effects {
        let effect_type = effect
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let journal_ref = effect
            .get("journal_ref")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let commit_before_actuate = effect
            .get("commit_before_actuate")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let effect_ts = effect
            .get("ts")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if effect_type == "actuate" {
            if journal_ref.is_empty() {
                effect_errors.push("actuate_missing_journal_ref".to_string());
            } else if !entry_ids.contains(&journal_ref) {
                effect_errors.push("actuate_journal_ref_not_found".to_string());
            }
            if policy_ok && !commit_before_actuate {
                effect_errors.push("actuate_without_commit_before_actuate".to_string());
            }
            if effect_ts.is_empty() {
                effect_errors.push("actuate_missing_ts".to_string());
            } else if let Some(commit_ts) = entry_ts.get(&journal_ref) {
                if commit_ts > &effect_ts {
                    effect_errors.push("actuate_precedes_commit".to_string());
                }
            }
        }
    }
    let ok = policy_ok && entry_errors.is_empty() && effect_errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "policy_path": EFFECT_JOURNAL_POLICY_PATH,
        "policy_ok": policy_ok,
        "journal_path": journal_rel,
        "entry_errors": entry_errors,
        "effect_errors": effect_errors,
        "entry_count": entry_ids.len()
    })
}

fn run_substrate_registry(root: &Path, strict: bool) -> Value {
    let registry = read_json(&root.join(SUBSTRATE_REGISTRY_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if registry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("substrate_registry_version_must_be_v1".to_string());
    }
    if registry
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "substrate_descriptor_registry"
    {
        errors.push("substrate_registry_kind_must_be_substrate_descriptor_registry".to_string());
    }
    let descriptors = registry
        .get("descriptors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if descriptors.is_empty() {
        errors.push("substrate_registry_missing_descriptors".to_string());
    }
    let mut descriptor_ids = Vec::new();
    let mut descriptor_set = BTreeSet::new();
    for descriptor in descriptors {
        let id = descriptor
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() {
            errors.push("substrate_descriptor_id_required".to_string());
            continue;
        }
        if !descriptor_set.insert(id.clone()) {
            errors.push("substrate_descriptor_duplicate_id".to_string());
        }
        descriptor_ids.push(id.clone());
        let determinism = descriptor
            .get("determinism")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(determinism, "high" | "medium" | "low") {
            errors.push("substrate_descriptor_invalid_determinism".to_string());
        }
        let latency_ok = descriptor
            .get("latency_ms")
            .and_then(Value::as_i64)
            .map(|v| v >= 0)
            .unwrap_or(false);
        if !latency_ok {
            errors.push("substrate_descriptor_invalid_latency_ms".to_string());
        }
        let energy_ok = descriptor
            .get("energy_mw")
            .and_then(Value::as_i64)
            .map(|v| v >= 0)
            .unwrap_or(false);
        if !energy_ok {
            errors.push("substrate_descriptor_invalid_energy_mw".to_string());
        }
        for field in ["isolation", "observability", "privacy_locality"] {
            if descriptor
                .get(field)
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
                == false
            {
                errors.push(format!(
                    "substrate_descriptor_missing_or_invalid_field::{field}"
                ));
            }
        }
    }

    let degrade = registry
        .get("degrade_matrix")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for scenario in ["no-network", "no-ternary", "no-qpu", "neural-link-loss"] {
        let ok = degrade
            .get(scenario)
            .and_then(Value::as_str)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if !ok {
            errors.push(format!("substrate_missing_degrade_scenario::{scenario}"));
        }
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "registry_path": SUBSTRATE_REGISTRY_PATH,
        "descriptor_ids": descriptor_ids,
        "errors": errors
    })
}

fn run_radix_guard(root: &Path, strict: bool) -> Value {
    let policy = read_json(&root.join(RADIX_POLICY_GUARD_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("radix_guard_version_must_be_v1".to_string());
    }
    if policy
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "radix_policy_guard"
    {
        errors.push("radix_guard_kind_must_be_radix_policy_guard".to_string());
    }
    let binary_required = policy
        .get("binary_required_paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ternary_classes = policy
        .get("ternary_allow_classes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if binary_required.is_empty() {
        errors.push("radix_guard_binary_required_paths_empty".to_string());
    }
    let required = ["crypto", "policy", "capability", "attestation", "journal"];
    let set: BTreeSet<String> = binary_required
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_ascii_lowercase())
        .collect();
    if set.len() != binary_required.len() {
        errors.push("radix_guard_binary_required_paths_duplicate".to_string());
    }
    for path in required {
        if !set.contains(path) {
            errors.push(format!("binary_required_missing::{path}"));
        }
    }
    let ternary_set: BTreeSet<String> = ternary_classes
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_ascii_lowercase())
        .collect();
    if ternary_set.len() != ternary_classes.len() {
        errors.push("radix_guard_ternary_allow_classes_duplicate".to_string());
    }
    if ternary_set.iter().any(|v| !is_token_id(v)) {
        errors.push("radix_guard_ternary_allow_class_invalid".to_string());
    }
    let mut overlap = Vec::new();
    for id in &ternary_set {
        if set.contains(id.as_str()) {
            overlap.push(id.to_string());
        }
    }
    if !overlap.is_empty() {
        errors.push("ternary_class_overlaps_binary_required_paths".to_string());
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "policy_path": RADIX_POLICY_GUARD_PATH,
        "binary_required_count": set.len(),
        "ternary_allow_count": ternary_set.len(),
        "overlap": overlap,
        "errors": errors
    })
}

