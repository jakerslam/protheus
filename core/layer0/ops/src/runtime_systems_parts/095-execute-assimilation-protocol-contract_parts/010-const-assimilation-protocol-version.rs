const ASSIMILATION_PROTOCOL_VERSION: &str = "infring_assimilation_protocol_v1";

const ASSIMILATION_ATTESTATION_OPS: &[&str] = &["run", "attest", "verify"];
const ASSIMILATION_DISTILLER_OPS: &[&str] = &["run", "distill", "verify"];
const ASSIMILATION_FRESHNESS_OPS: &[&str] = &["run", "freshness", "verify"];
const ASSIMILATION_GENERIC_OPS: &[&str] = &["run", "verify"];
const ASSIMILATION_ATTESTATION_PHASES: &[&str] = &["attestation", "generic"];
const ASSIMILATION_DISTILLER_PHASES: &[&str] = &["distillation", "generic"];
const ASSIMILATION_FRESHNESS_PHASES: &[&str] = &["freshness", "generic"];
const ASSIMILATION_GENERIC_PHASES: &[&str] = &["generic"];

#[derive(Clone, Copy)]
struct AssimilationComponentProfile {
    default_phase: &'static str,
    allowed_ops: &'static [&'static str],
    allowed_phases: &'static [&'static str],
    surface_id: &'static str,
    substrate_surface: &'static str,
}

fn is_assimilation_system_id(system_id: &str) -> bool {
    system_id
        .trim()
        .to_ascii_uppercase()
        .starts_with("SYSTEMS-ASSIMILATION-")
}

fn assimilation_component(system_id: &str) -> &'static str {
    let id = system_id.trim().to_ascii_uppercase();
    if id.contains("SOURCE_ATTESTATION_EXTENSION") {
        "source_attestation_extension"
    } else if id.contains("TRAJECTORY_SKILL_DISTILLER") {
        "trajectory_skill_distiller"
    } else if id.contains("WORLD_MODEL_FRESHNESS") {
        "world_model_freshness"
    } else {
        "assimilation_generic"
    }
}

fn assimilation_component_profile(component: &str) -> AssimilationComponentProfile {
    match component {
        "source_attestation_extension" => AssimilationComponentProfile {
            default_phase: "attestation",
            allowed_ops: ASSIMILATION_ATTESTATION_OPS,
            allowed_phases: ASSIMILATION_ATTESTATION_PHASES,
            surface_id: "substrate://runtime-systems/source_attestation_extension",
            substrate_surface: "source_attestation_surface",
        },
        "trajectory_skill_distiller" => AssimilationComponentProfile {
            default_phase: "distillation",
            allowed_ops: ASSIMILATION_DISTILLER_OPS,
            allowed_phases: ASSIMILATION_DISTILLER_PHASES,
            surface_id: "substrate://runtime-systems/trajectory_skill_distiller",
            substrate_surface: "trajectory_distillation_surface",
        },
        "world_model_freshness" => AssimilationComponentProfile {
            default_phase: "freshness",
            allowed_ops: ASSIMILATION_FRESHNESS_OPS,
            allowed_phases: ASSIMILATION_FRESHNESS_PHASES,
            surface_id: "substrate://runtime-systems/world_model_freshness",
            substrate_surface: "world_model_freshness_surface",
        },
        _ => AssimilationComponentProfile {
            default_phase: "generic",
            allowed_ops: ASSIMILATION_GENERIC_OPS,
            allowed_phases: ASSIMILATION_GENERIC_PHASES,
            surface_id: "substrate://runtime-systems/assimilation_generic",
            substrate_surface: "assimilation_generic_surface",
        },
    }
}

fn assimilation_default_phase(component: &str) -> &'static str {
    assimilation_component_profile(component).default_phase
}

fn assimilation_allowed_ops(component: &str) -> &'static [&'static str] {
    assimilation_component_profile(component).allowed_ops
}

fn assimilation_allowed_phases(component: &str) -> &'static [&'static str] {
    assimilation_component_profile(component).allowed_phases
}

fn normalize_assimilation_operation(command: &str, args: &[String]) -> String {
    let op = lane_utils::parse_flag(args, "op", true).unwrap_or_else(|| command.to_string());
    lane_utils::clean_text(Some(op.as_str()), 64)
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn resolve_assimilation_phase(component: &str, payload: &Value, args: &[String]) -> String {
    let phase_flag = lane_utils::parse_flag(args, "phase", true);
    let phase_payload = payload.get("phase").and_then(Value::as_str);
    let cleaned = lane_utils::clean_text(phase_flag.as_deref().or(phase_payload), 64)
        .to_ascii_lowercase()
        .replace('_', "-");
    if cleaned.is_empty() {
        assimilation_default_phase(component).to_string()
    } else {
        cleaned
    }
}

fn assimilation_protocol_paths(root: &Path, system_id: &str) -> (PathBuf, PathBuf) {
    let canonical_id = lane_utils::clean_token(Some(system_id), "runtime-assimilation");
    let dir = systems_dir(root).join("_assimilation").join(canonical_id);
    (
        dir.join("protocol_state.json"),
        dir.join("protocol_history.jsonl"),
    )
}

fn assimilation_protocol_step_receipts_path(root: &Path, system_id: &str) -> PathBuf {
    let canonical_id = lane_utils::clean_token(Some(system_id), "runtime-assimilation");
    systems_dir(root)
        .join("_assimilation")
        .join(canonical_id)
        .join("protocol_step_receipts.jsonl")
}

fn string_from_payload(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|raw| lane_utils::clean_text(Some(raw), 96))
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

fn resolve_hard_selector(payload: &Value, args: &[String]) -> String {
    let selector = lane_utils::parse_flag(args, "hard-selector", true)
        .or_else(|| lane_utils::parse_flag(args, "selector", true))
        .or_else(|| lane_utils::parse_flag(args, "surface-id", true))
        .or_else(|| lane_utils::parse_flag(args, "core-domain", true))
        .or_else(|| string_from_payload(payload, "hard_selector"))
        .or_else(|| string_from_payload(payload, "selector"))
        .or_else(|| string_from_payload(payload, "surface_id"))
        .unwrap_or_default();
    lane_utils::clean_text(Some(selector.as_str()), 96)
        .trim()
        .to_ascii_lowercase()
}

fn selector_bypass_requested(payload: &Value, args: &[String]) -> bool {
    let from_flags = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "selector-bypass", true).as_deref(),
        false,
    ) || lane_utils::parse_bool(
        lane_utils::parse_flag(args, "bypass-selector", true).as_deref(),
        false,
    );
    let from_payload = payload
        .get("selector_bypass")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || payload
            .get("bypass_selector")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    from_flags || from_payload
}

fn assimilation_denial_priority(code: &str) -> usize {
    match code {
        "assimilation_selector_bypass_rejected" => 0,
        "assimilation_hard_selector_closure_reject" => 1,
        "assimilation_protocol_op_not_allowed" => 2,
        "assimilation_protocol_phase_mismatch" => 3,
        "assimilation_candidate_closure_incomplete" => 4,
        _ => 100,
    }
}

fn surface_matches_hard_selector(surface: &Value, hard_selector: &str) -> bool {
    if hard_selector.is_empty() {
        return true;
    }
    let selector = hard_selector.trim().to_ascii_lowercase();
    if selector.is_empty() {
        return true;
    }
    let candidates = [
        surface
            .get("surface_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("component")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("binding_system_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
    ];
    candidates
        .iter()
        .any(|candidate| !candidate.is_empty() && candidate.contains(&selector))
}

fn contains_string(values: &[String], candidate: &str) -> bool {
    let normalized = candidate.trim().to_ascii_lowercase();
    values.iter().any(|row| row == &normalized)
}

fn assimilation_recon_surfaces(component: &str, system_id: &str) -> Vec<Value> {
    let profile = assimilation_component_profile(component);
    vec![json!({
        "surface_id": profile.surface_id,
        "provider": "substrate_runtime_systems",
        "domain": "runtime-systems",
        "component": component,
        "binding_system_id": system_id,
        "substrate_surface": profile.substrate_surface,
        "supported_operations": profile.allowed_ops,
        "supported_phases": profile.allowed_phases
    })]
}
