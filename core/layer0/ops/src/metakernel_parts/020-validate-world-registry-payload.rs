fn validate_world_registry_payload(registry: &Value) -> (bool, Value) {
    let mut errors: Vec<String> = Vec::new();
    if registry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("world_registry_version_must_be_v1".to_string());
    }
    if registry
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "wit_world_registry"
    {
        errors.push("world_registry_kind_must_be_wit_world_registry".to_string());
    }
    let worlds = registry
        .get("worlds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if worlds.is_empty() {
        errors.push("world_registry_missing_worlds".to_string());
    }
    let mut world_ids = Vec::new();
    for world in worlds {
        let id = world
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let abi = world
            .get("abi_version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() {
            errors.push("world_registry_world_id_required".to_string());
            continue;
        }
        if abi.is_empty() {
            errors.push("world_registry_world_abi_required".to_string());
        } else if !is_semver_triplet(&abi) {
            errors.push("world_registry_world_abi_invalid_semver".to_string());
        }
        let supported = world
            .get("supported_capabilities")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if supported.is_empty() {
            errors.push("world_registry_supported_capabilities_required".to_string());
        }
        for cap in supported {
            let cid = cap.as_str().unwrap_or_default().trim().to_ascii_lowercase();
            if !EXPECTED_PRIMITIVES.iter().any(|p| *p == cid) {
                errors.push("world_registry_supported_capabilities_unknown_primitive".to_string());
            }
        }
        let targets = world
            .get("component_targets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if targets.is_empty() {
            errors.push("world_registry_component_targets_required".to_string());
        }
        world_ids.push(id);
    }
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    for id in &world_ids {
        if !seen.insert(id.clone()) {
            duplicates.push(id.clone());
        }
    }
    if !duplicates.is_empty() {
        errors.push("world_registry_duplicate_ids".to_string());
    }
    (
        errors.is_empty(),
        json!({
            "errors": errors,
            "world_ids": world_ids,
            "duplicate_ids": duplicates
        }),
    )
}

fn validate_capability_taxonomy_payload(taxonomy: &Value) -> (bool, Value) {
    let mut errors: Vec<String> = Vec::new();
    if taxonomy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("capability_taxonomy_version_must_be_v1".to_string());
    }
    let effects = taxonomy
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if effects.is_empty() {
        errors.push("capability_taxonomy_effects_required".to_string());
    }
    let required_effects = [
        "observe",
        "infer",
        "store",
        "communicate",
        "actuate",
        "train",
        "quantum",
        "admin",
    ];
    let mut effect_ids = BTreeSet::new();
    for effect in effects {
        let id = effect
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let risk = effect
            .get("risk_default")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        if id.is_empty() {
            errors.push("capability_taxonomy_effect_id_required".to_string());
            continue;
        }
        if !matches!(risk.as_str(), "R0" | "R1" | "R2" | "R3" | "R4") {
            errors.push("capability_taxonomy_invalid_risk_class".to_string());
        }
        effect_ids.insert(id);
    }
    let expected: BTreeSet<String> = required_effects.iter().map(|v| v.to_string()).collect();
    let missing_effects: Vec<String> = expected.difference(&effect_ids).cloned().collect();
    if !missing_effects.is_empty() {
        errors.push("capability_taxonomy_missing_required_effects".to_string());
    }

    let primitive_effects = taxonomy
        .get("primitive_effects")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let expected_primitives: BTreeSet<String> =
        EXPECTED_PRIMITIVES.iter().map(|v| v.to_string()).collect();
    for (primitive, effects) in primitive_effects {
        if !expected_primitives.contains(&primitive) {
            errors.push("capability_taxonomy_unknown_primitive_mapping".to_string());
        }
        for effect in effects.as_array().cloned().unwrap_or_default() {
            let id = effect
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if !expected.contains(&id) {
                errors.push("capability_taxonomy_unknown_effect_mapping".to_string());
            }
        }
    }
    (
        errors.is_empty(),
        json!({
            "errors": errors,
            "missing_required_effects": missing_effects
        }),
    )
}

fn parse_nonneg_i64_field(map: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key).and_then(Value::as_i64).filter(|v| *v >= 0)
}

fn run_registry(root: &Path, strict: bool) -> Value {
    let registry_path = root.join(REGISTRY_PATH);
    let registry = read_json(&registry_path).unwrap_or(Value::Null);
    let (registry_ok, registry_report) = validate_registry_payload(&registry);
    let primitives = gather_primitives_from_registry(&registry).unwrap_or_default();
    let valid: HashSet<String> = primitives.into_iter().collect();
    let unknown_usage = collect_unknown_primitive_usage(root, &valid);
    let unknown_usage_ok = unknown_usage.is_empty();
    let ok = registry_ok && unknown_usage_ok;

    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "registry_path": REGISTRY_PATH,
        "registry_ok": registry_ok,
        "registry_report": registry_report,
        "unknown_primitive_usage_count": unknown_usage.len(),
        "unknown_primitive_usage": unknown_usage
    })
}

fn run_worlds(root: &Path, strict: bool, manifest_rel: &str) -> Value {
    let registry_path = root.join(WIT_WORLD_REGISTRY_PATH);
    let registry = read_json(&registry_path).unwrap_or(Value::Null);
    let (registry_ok, registry_report) = validate_world_registry_payload(&registry);

    let worlds = registry
        .get("worlds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut world_table: serde_json::Map<String, Value> = serde_json::Map::new();
    for world in worlds {
        let id = world
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() {
            continue;
        }
        world_table.insert(id, world);
    }

    let manifest_path = root.join(manifest_rel);
    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    let world_id = manifest
        .get("world")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let world_entry = world_table.get(&world_id);
    let world_declared = !world_id.is_empty();
    let world_exists = world_entry.is_some();
    let manifest_abi = manifest
        .get("abi_version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let world_abi = world_entry
        .and_then(|w| w.get("abi_version"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let abi_declared = !manifest_abi.is_empty();
    let abi_semver_ok = !manifest_abi.is_empty() && is_semver_triplet(&manifest_abi);
    let abi_match = !manifest_abi.is_empty() && !world_abi.is_empty() && manifest_abi == world_abi;
    let manifest_component_target = manifest
        .get("component_target")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let target_declared = !manifest_component_target.is_empty();
    let target_allowed = world_entry
        .and_then(|w| w.get("component_targets"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|v| v == manifest_component_target);

    let manifest_caps: BTreeSet<String> = manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    let supported_caps: BTreeSet<String> = world_entry
        .and_then(|w| w.get("supported_capabilities"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    let unsupported_caps: Vec<String> =
        manifest_caps.difference(&supported_caps).cloned().collect();
    let compatibility_ok = if supported_caps.is_empty() {
        true
    } else {
        unsupported_caps.is_empty()
    };

    let ok = registry_ok
        && world_declared
        && world_exists
        && compatibility_ok
        && abi_declared
        && abi_semver_ok
        && abi_match
        && target_declared
        && target_allowed;
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "registry_path": WIT_WORLD_REGISTRY_PATH,
        "registry_ok": registry_ok,
        "registry_report": registry_report,
        "manifest_path": manifest_rel,
        "world_declared": world_declared,
        "world_id": world_id,
        "world_exists": world_exists,
        "manifest_abi_version": manifest_abi,
        "world_abi_version": world_abi,
        "abi_declared": abi_declared,
        "abi_semver_ok": abi_semver_ok,
        "abi_match": abi_match,
        "component_target_declared": target_declared,
        "component_target_allowed": target_allowed,
        "compatibility_ok": compatibility_ok,
        "unsupported_capabilities": unsupported_caps
    })
}

fn run_capability_taxonomy(root: &Path, strict: bool, manifest_rel: &str) -> Value {
    let taxonomy_path = root.join(CAPABILITY_TAXONOMY_PATH);
    let taxonomy = read_json(&taxonomy_path).unwrap_or(Value::Null);
    let (taxonomy_ok, taxonomy_report) = validate_capability_taxonomy_payload(&taxonomy);
    let effects = taxonomy
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut effect_risk = std::collections::HashMap::new();
    for effect in effects {
        let id = effect
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let risk = effect
            .get("risk_default")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        if !id.is_empty() && !risk.is_empty() {
            effect_risk.insert(id, risk);
        }
    }
    let primitive_effects = taxonomy
        .get("primitive_effects")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let manifest_path = root.join(manifest_rel);
    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    let manifest_caps = manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut derived_effects = BTreeSet::new();
    for cap in manifest_caps {
        let id = cap.as_str().unwrap_or_default().trim().to_ascii_lowercase();
        if id.is_empty() {
            continue;
        }
        for effect in primitive_effects
            .get(&id)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let eid = effect
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if !eid.is_empty() {
                derived_effects.insert(eid);
            }
        }
    }

    let mut highest_risk = "R0".to_string();
    let mut high_risk_effects = Vec::new();
    for effect in &derived_effects {
        let risk = effect_risk
            .get(effect)
            .cloned()
            .unwrap_or_else(|| "R4".to_string());
        if risk > highest_risk {
            highest_risk = risk.clone();
        }
        if matches!(risk.as_str(), "R3" | "R4") {
            high_risk_effects.push(effect.clone());
        }
    }
    let capability_gate = manifest
        .get("policy_checks")
        .and_then(Value::as_object)
        .and_then(|m| m.get("capability_gate"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let gate_required = taxonomy
        .get("high_risk_requires_policy_gate")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let policy_gate_ok = !gate_required || high_risk_effects.is_empty() || capability_gate;
    let ok = taxonomy_ok && policy_gate_ok;
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "taxonomy_path": CAPABILITY_TAXONOMY_PATH,
        "taxonomy_ok": taxonomy_ok,
        "taxonomy_report": taxonomy_report,
        "manifest_path": manifest_rel,
        "derived_effects": derived_effects.into_iter().collect::<Vec<_>>(),
        "highest_risk": highest_risk,
        "high_risk_effects": high_risk_effects,
        "gate_required": gate_required,
        "policy_gate_present": capability_gate,
        "policy_gate_ok": policy_gate_ok
    })
}

