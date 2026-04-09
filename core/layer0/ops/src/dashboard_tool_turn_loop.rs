// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use protheus_nexus_core_v1::registry::ModuleLifecycleState;
use protheus_nexus_core_v1::{
    DefaultNexusPolicy, DeliveryAuthorizationInput, LeaseIssueRequest, MainNexusControlPlane,
    ModuleKind, NexusFeatureFlags, SubNexusRegistration, TrustClass, VerityClass,
};

const TERMINAL_PERMISSION_POLICY_REL: &str =
    "client/runtime/config/terminal_command_permission_policy.json";
const CLIENT_INGRESS_SUB_NEXUS: &str = "client_ingress";
const CLIENT_INGRESS_BRIDGE_SUB_NEXUS: &str = "client_ingress_bridge";
const NEXUS_INGRESS_ISSUER: &str = "dashboard_tool_turn_loop";

#[derive(Clone, Copy)]
struct IngressRouteDescriptor {
    target: &'static str,
    schema_id: &'static str,
    verb: &'static str,
    required_verity: VerityClass,
    trust_class: TrustClass,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_tool_name(raw: &str) -> String {
    clean_text(raw, 80)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

fn tool_is_autonomous_spawn(normalized: &str) -> bool {
    matches!(
        normalized,
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn"
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|delta| delta.as_millis() as u64)
        .unwrap_or(0)
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => match clean_text(&raw, 40).to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        Err(_) => fallback,
    }
}

fn ingress_nexus_enabled() -> bool {
    bool_env("PROTHEUS_HIERARCHICAL_NEXUS_V1", true)
}

fn ingress_force_block_pair_enabled() -> bool {
    bool_env(
        "PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE",
        false,
    )
}

fn parse_module_lifecycle(raw: &str) -> Option<ModuleLifecycleState> {
    let lowered = clean_text(raw, 40).to_ascii_lowercase();
    match lowered.as_str() {
        "active" => Some(ModuleLifecycleState::Active),
        "quiesced" => Some(ModuleLifecycleState::Quiesced),
        "detached" => Some(ModuleLifecycleState::Detached),
        "maintenance" => Some(ModuleLifecycleState::Maintenance),
        "draining" => Some(ModuleLifecycleState::Draining {
            drain_deadline_ms: now_ms().saturating_add(30_000),
        }),
        _ => None,
    }
}

fn ingress_lifecycle_override_from_env() -> Option<ModuleLifecycleState> {
    std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE")
        .ok()
        .and_then(|raw| parse_module_lifecycle(raw.as_str()))
}

fn ingress_route_for_tool(tool_name: &str) -> IngressRouteDescriptor {
    let normalized = normalize_tool_name(tool_name);
    if matches!(
        normalized.as_str(),
        "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
            | "batch_query"
            | "file_read"
            | "file_read_many"
    ) {
        return IngressRouteDescriptor {
            target: "context_stacks",
            schema_id: "client_ingress.tool.retrieval",
            verb: "invoke",
            required_verity: VerityClass::High,
            trust_class: TrustClass::InterModuleData,
        };
    }
    if normalized.starts_with("stomach_") {
        return IngressRouteDescriptor {
            target: "stomach",
            schema_id: "client_ingress.tool.stomach",
            verb: "invoke",
            required_verity: VerityClass::High,
            trust_class: TrustClass::InterModuleData,
        };
    }
    IngressRouteDescriptor {
        target: CLIENT_INGRESS_BRIDGE_SUB_NEXUS,
        schema_id: "client_ingress.tool.execute",
        verb: "invoke",
        required_verity: VerityClass::Standard,
        trust_class: TrustClass::ClientIngressBoundary,
    }
}

fn terminal_ingress_route() -> IngressRouteDescriptor {
    IngressRouteDescriptor {
        target: CLIENT_INGRESS_BRIDGE_SUB_NEXUS,
        schema_id: "client_ingress.terminal.exec",
        verb: "execute",
        required_verity: VerityClass::Standard,
        trust_class: TrustClass::ClientIngressBoundary,
    }
}

fn ensure_sub_nexus_registered(
    nexus: &mut MainNexusControlPlane,
    sub_nexus_id: &str,
) -> Result<(), String> {
    if nexus.registry().contains(sub_nexus_id) {
        return Ok(());
    }
    let (module_kind, trust_class, verity_class) = match sub_nexus_id {
        "stomach" => (
            ModuleKind::Stomach,
            TrustClass::InterModuleData,
            VerityClass::High,
        ),
        "context_stacks" => (
            ModuleKind::ContextStacks,
            TrustClass::InterModuleData,
            VerityClass::High,
        ),
        CLIENT_INGRESS_SUB_NEXUS => (
            ModuleKind::ClientIngress,
            TrustClass::ClientIngressBoundary,
            VerityClass::Standard,
        ),
        _ => (
            ModuleKind::Other,
            TrustClass::ClientIngressBoundary,
            VerityClass::Standard,
        ),
    };
    let registration =
        SubNexusRegistration::new(sub_nexus_id, module_kind, trust_class, verity_class);
    let _ = nexus.register_sub_nexus(NEXUS_INGRESS_ISSUER, registration)?;
    Ok(())
}

fn authorize_client_ingress_route_with_nexus_inner(
    route_label: &str,
    route: IngressRouteDescriptor,
    force_block_pair: bool,
    source_lifecycle_override: Option<ModuleLifecycleState>,
) -> Result<Value, String> {
    let mut policy = DefaultNexusPolicy::default();
    if force_block_pair {
        policy.block_pair(CLIENT_INGRESS_SUB_NEXUS, route.target);
    }
    let mut nexus = MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        policy,
    );
    let _ = nexus.register_v1_adapters(NEXUS_INGRESS_ISSUER)?;
    ensure_sub_nexus_registered(&mut nexus, route.target)?;
    if let Some(next) = source_lifecycle_override {
        let _ = nexus.set_module_lifecycle(NEXUS_INGRESS_ISSUER, CLIENT_INGRESS_SUB_NEXUS, next)?;
    }

    let lease = nexus.issue_route_lease(
        NEXUS_INGRESS_ISSUER,
        LeaseIssueRequest {
            source: CLIENT_INGRESS_SUB_NEXUS.to_string(),
            target: route.target.to_string(),
            schema_ids: vec![route.schema_id.to_string()],
            verbs: vec![route.verb.to_string()],
            required_verity: route.required_verity,
            trust_class: route.trust_class,
            requested_ttl_ms: 45_000,
            template_id: None,
            template_version: None,
        },
    )?;
    let delivery = nexus.authorize_direct_delivery(
        NEXUS_INGRESS_ISSUER,
        DeliveryAuthorizationInput {
            source: CLIENT_INGRESS_SUB_NEXUS.to_string(),
            target: route.target.to_string(),
            schema_id: route.schema_id.to_string(),
            verb: route.verb.to_string(),
            offered_verity: route.required_verity,
            lease_id: Some(lease.lease_id.clone()),
            now_ms: None,
        },
    );
    if !delivery.allowed {
        return Err(format!(
            "client_ingress_nexus_delivery_denied:{}",
            delivery.reason
        ));
    }
    let receipt_ids = nexus
        .receipts()
        .iter()
        .map(|row| Value::String(row.receipt_id.clone()))
        .collect::<Vec<_>>();
    Ok(json!({
      "enabled": true,
      "source": CLIENT_INGRESS_SUB_NEXUS,
      "target": route.target,
      "schema_id": route.schema_id,
      "verb": route.verb,
      "route_label": clean_text(route_label, 200),
      "lease_id": lease.lease_id,
      "policy_decision_ref": lease.policy_decision_ref,
      "delivery": {"allowed": delivery.allowed, "reason": delivery.reason, "local_resolution": delivery.local_resolution, "conduit_link_id": delivery.conduit_link_id},
      "metrics": nexus.metrics(),
      "receipt_ids": receipt_ids
    }))
}

pub(crate) fn authorize_ingress_tool_call_with_nexus(
    tool_name: &str,
) -> Result<Option<Value>, String> {
    if !ingress_nexus_enabled() {
        return Ok(None);
    }
    let route = ingress_route_for_tool(tool_name);
    let connection = authorize_client_ingress_route_with_nexus_inner(
        &format!("tool:{tool_name}"),
        route,
        ingress_force_block_pair_enabled(),
        ingress_lifecycle_override_from_env(),
    )?;
    Ok(Some(connection))
}

pub(crate) fn authorize_ingress_terminal_command_with_nexus(
    command: &str,
) -> Result<Option<Value>, String> {
    if !ingress_nexus_enabled() {
        return Ok(None);
    }
    let connection = authorize_client_ingress_route_with_nexus_inner(
        &format!("terminal:{}", clean_text(command, 220)),
        terminal_ingress_route(),
        ingress_force_block_pair_enabled(),
        ingress_lifecycle_override_from_env(),
    )?;
    Ok(Some(connection))
}

fn load_permission_rules(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut deny = Vec::<String>::new();
    let mut ask = Vec::<String>::new();
    let path = root.join(TERMINAL_PERMISSION_POLICY_REL);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            let (policy_deny, policy_ask) =
                crate::command_permission_kernel::collect_permission_rules_for_kernel(Some(&value));
            deny.extend(policy_deny);
            ask.extend(policy_ask);
        }
    }
    deny.sort();
    deny.dedup();
    ask.sort();
    ask.dedup();
    (deny, ask)
}

fn input_confirmed(input: &Value) -> bool {
    input
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || !clean_text(
            input
                .get("approval_note")
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        )
        .is_empty()
}

fn tool_command_signature(tool_name: &str, input: &Value) -> String {
    let normalized = normalize_tool_name(tool_name);
    match normalized.as_str() {
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => clean_text(
            input
                .get("command")
                .or_else(|| input.get("cmd"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            3000,
        ),
        "manage_agent" | "agent_action" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let agent = clean_text(
                input
                    .get("agent_id")
                    .or_else(|| input.get("target_agent_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            clean_text(format!("manage_agent {action} {agent}").trim(), 400)
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let count = input
                .get("count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            clean_text(
                format!(
                    "spawn_subagents count={} objective={}",
                    count,
                    clean_text(
                        input
                            .get("objective")
                            .or_else(|| input.get("task"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        120
                    )
                )
                .trim(),
                500,
            )
        }
        _ => String::new(),
    }
}

pub(crate) fn pre_tool_permission_gate(
    root: &Path,
    tool_name: &str,
    input: &Value,
) -> Option<Value> {
    let normalized = normalize_tool_name(tool_name);
    let command = tool_command_signature(&normalized, input);
    if command.is_empty() {
        return None;
    }
    let (deny_rules, ask_rules) = load_permission_rules(root);
    let (verdict, matched) =
        crate::command_permission_kernel::evaluate_command_permission_for_kernel(
            &command,
            &deny_rules,
            &ask_rules,
        );
    let verdict_str = verdict.as_str().to_string();
    if verdict_str == "allow" {
        return None;
    }
    if verdict_str == "ask"
        && (input_confirmed(input) || tool_is_autonomous_spawn(normalized.as_str()))
    {
        return None;
    }
    let error = if verdict_str == "deny" {
        "tool_permission_denied"
    } else {
        "tool_confirmation_required"
    };
    Some(json!({
        "ok": false,
        "error": error,
        "type": "tool_pre_gate_blocked",
        "tool": normalized,
        "fail_closed": true,
        "permission_gate": {
            "verdict": verdict_str,
            "matched": matched,
            "deny_rules_count": deny_rules.len(),
            "ask_rules_count": ask_rules.len(),
            "command_signature": command
        },
        "hint": if error == "tool_confirmation_required" {
            "Confirmation required before this tool can run."
        } else {
            "Tool blocked by command permission policy."
        }
    }))
}

fn rewrite_text_for_post_filter(value: &str) -> Option<(String, String)> {
    let cleaned = clean_text(value, 32_000);
    if cleaned.is_empty() {
        return None;
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_raw_payload_dump(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_unsynthesized_web_dump(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_repetitive_thinking_chatter(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&cleaned)
    {
        return Some((rewritten, format!("failure_placeholder_rewrite:{rule_id}")));
    }
    if crate::tool_output_match_filter::matches_ack_placeholder(&cleaned) {
        return Some((
            crate::tool_output_match_filter::no_findings_user_copy().to_string(),
            "ack_placeholder_suppressed".to_string(),
        ));
    }
    None
}

fn rewrite_object_key(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    events: &mut Vec<String>,
) {
    let original = obj
        .get(key)
        .and_then(Value::as_str)
        .map(|row| row.to_string())
        .unwrap_or_default();
    if original.is_empty() {
        return;
    }
    if let Some((rewritten, event)) = rewrite_text_for_post_filter(&original) {
        obj.insert(key.to_string(), Value::String(rewritten));
        events.push(format!("{key}:{event}"));
    }
}

pub(crate) fn apply_post_tool_output_filter(payload: &mut Value) -> Value {
    let mut events = Vec::<String>::new();
    if let Some(obj) = payload.as_object_mut() {
        for key in [
            "summary", "content", "result", "message", "error", "response", "details", "hint",
            "text",
        ] {
            rewrite_object_key(obj, key, &mut events);
        }
        if let Some(result_obj) = obj.get_mut("result").and_then(Value::as_object_mut) {
            for key in [
                "summary", "content", "result", "message", "error", "response", "details", "hint",
                "text",
            ] {
                rewrite_object_key(result_obj, key, &mut events);
            }
        }
        if let Some(receipt_obj) = obj.get_mut("receipt").and_then(Value::as_object_mut) {
            for key in [
                "summary", "content", "message", "error", "response", "details", "hint", "text",
            ] {
                rewrite_object_key(receipt_obj, key, &mut events);
            }
        }
        if let Some(finalization_obj) = obj
            .get_mut("response_finalization")
            .and_then(Value::as_object_mut)
        {
            for key in ["response", "message", "error", "details", "text"] {
                rewrite_object_key(finalization_obj, key, &mut events);
            }
        }
    }
    let report = json!({
        "applied": !events.is_empty(),
        "events": events
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("turn_loop_post_filter".to_string(), report.clone());
    }
    report
}

pub(crate) fn annotate_tool_payload_tracking(
    root: &Path,
    session_id: &str,
    tool_name: &str,
    payload: &mut Value,
) {
    let post_filter_report = apply_post_tool_output_filter(payload);
    let tracking = record_tool_turn_tracking(root, session_id, tool_name, payload);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("turn_loop_post_filter".to_string(), post_filter_report);
        obj.insert(
            "turn_loop_tracking".to_string(),
            tracking.unwrap_or(Value::Null),
        );
    }
}

fn output_tokens_estimate(payload: &Value) -> usize {
    let mut total = 0usize;
    for key in ["summary", "content", "result", "message", "error"] {
        total += payload
            .get(key)
            .and_then(Value::as_str)
            .map(|row| row.len())
            .unwrap_or(0);
    }
    if total == 0 {
        total = payload.to_string().len().min(32_000);
    }
    (total / 4).max(1)
}

pub(crate) fn record_tool_turn_tracking(
    root: &Path,
    session_id: &str,
    tool_name: &str,
    payload: &Value,
) -> Option<Value> {
    let clean_session = clean_text(session_id, 120);
    if clean_session.is_empty() {
        return None;
    }
    let command = format!("tool::{}", normalize_tool_name(tool_name));
    let batch = json!({
        "session_id": clean_session,
        "records": [
            {
                "session_id": clean_session,
                "command": command,
                "output_tokens": output_tokens_estimate(payload)
            }
        ]
    });
    crate::session_command_tracking_kernel::record_batch_for_kernel(root, &batch).ok()
}

pub(crate) fn turn_transaction_payload(
    hydrate: &str,
    tool_execute: &str,
    synthesize: &str,
    session_persist: &str,
) -> Value {
    json!({
        "hydrate": clean_text(hydrate, 60),
        "tool_execute": clean_text(tool_execute, 60),
        "synthesize": clean_text(synthesize, 60),
        "session_persist": clean_text(session_persist, 60)
    })
}

pub(crate) fn hydration_failed_payload(agent_id: &str) -> Value {
    json!({
        "ok": false,
        "error": "context_hydration_incomplete",
        "agent_id": clean_text(agent_id, 120),
        "message": "Conversation context hydration failed closed before model execution. Retry once; if it persists, run `infringctl doctor --json` and `/context`.",
        "turn_transaction": turn_transaction_payload("failed_closed", "skipped", "skipped", "skipped")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_filter_rewrites_ack_placeholder_copy() {
        let mut payload = json!({"ok": true, "summary": "Web search completed."});
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let lowered = payload
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(lowered.contains("usable tool findings"));
        assert!(!lowered.contains("web search completed"));
    }

    #[test]
    fn post_filter_rewrites_raw_payload_dump_summary() {
        let mut payload = json!({
            "ok": true,
            "summary": "{\"agent_id\":\"agent-83ed64e07515\",\"input_tokens\":33,\"output_tokens\":85,\"latent_tool_candidates\":[],\"nexus_connection\":{},\"turn_loop_tracking\":{},\"turn_transaction\":{},\"response_finalization\":{},\"tools\":[]}"
        });
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("suppressed raw runtime payload"));
    }

    #[test]
    fn post_filter_rewrites_unsynthesized_web_dump_to_actionable_copy() {
        let mut payload = json!({
            "ok": true,
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B]."
        });
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("source-backed answer"));
        assert!(!summary.to_ascii_lowercase().contains("bing.com"));
    }

    #[test]
    fn pre_gate_respects_confirm_for_ask_verdicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"ask_rules":["Bash(echo *)"]}"#).expect("write policy");
        let blocked = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello"}),
        )
        .expect("blocked");
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("tool_confirmation_required")
        );
        let allowed = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello","confirm":true}),
        );
        assert!(allowed.is_none());
    }

    #[test]
    fn pre_gate_allows_spawn_without_confirm_for_ask_verdicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"ask_rules":["spawn_subagents*"]}"#)
            .expect("write policy");
        let out = pre_tool_permission_gate(
            root.path(),
            "spawn_subagents",
            &json!({"count": 2, "objective": "parallelize"}),
        );
        assert!(out.is_none());
    }

    #[test]
    fn pre_gate_still_denies_spawn_when_policy_denies() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"deny_rules":["spawn_subagents*"]}"#)
            .expect("write policy");
        let blocked = pre_tool_permission_gate(
            root.path(),
            "spawn_subagents",
            &json!({"count": 2, "objective": "parallelize"}),
        )
        .expect("blocked");
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("tool_permission_denied")
        );
    }

    #[test]
    fn ingress_nexus_authorization_succeeds_for_web_search_tool_route() {
        let route = ingress_route_for_tool("web_search");
        let out =
            authorize_client_ingress_route_with_nexus_inner("tool:web_search", route, false, None)
                .expect("nexus route");
        assert_eq!(
            out.get("source").and_then(Value::as_str),
            Some(CLIENT_INGRESS_SUB_NEXUS)
        );
        assert_eq!(
            out.get("target").and_then(Value::as_str),
            Some("context_stacks")
        );
        assert_eq!(
            out.pointer("/delivery/allowed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn ingress_nexus_authorization_fails_closed_when_pair_blocked() {
        let route = ingress_route_for_tool("web_search");
        let err =
            authorize_client_ingress_route_with_nexus_inner("tool:web_search", route, true, None)
                .expect_err("blocked");
        assert!(err.contains("lease_denied"));
    }

    #[test]
    fn ingress_nexus_authorization_fails_when_client_ingress_quiesced() {
        let route = ingress_route_for_tool("file_read");
        let err = authorize_client_ingress_route_with_nexus_inner(
            "tool:file_read",
            route,
            false,
            Some(ModuleLifecycleState::Quiesced),
        )
        .expect_err("quiesced blocked");
        assert!(err.contains("lease_source_not_accepting_new_leases"));
    }

    #[test]
    fn ingress_route_descriptor_maps_batch_query_to_context_stacks() {
        let route = ingress_route_for_tool("batch_query");
        assert_eq!(route.target, "context_stacks");
        assert_eq!(route.schema_id, "client_ingress.tool.retrieval");
        assert_eq!(route.verb, "invoke");
    }

    #[test]
    fn ingress_route_descriptor_maps_file_read_many_to_context_stacks() {
        let route = ingress_route_for_tool("file_read_many");
        assert_eq!(route.target, "context_stacks");
        assert_eq!(route.schema_id, "client_ingress.tool.retrieval");
        assert_eq!(route.verb, "invoke");
    }
}
