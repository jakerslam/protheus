// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const TERMINAL_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/terminal_broker.json";
const TERMINAL_PERMISSION_POLICY_REL: &str =
    "client/runtime/config/terminal_command_permission_policy.json";
const OUTPUT_MAX_BYTES: usize = 32 * 1024;
const OUTPUT_TRUNCATION_MARKER: &str = "\n... (output truncated) ...\n";

#[derive(Debug, Clone)]
pub struct CommandResolution {
    pub requested_command: String,
    pub resolved_command: String,
    pub translated: bool,
    pub translation_reason: String,
    pub suggestions: Vec<String>,
}

fn now_iso() -> String {
    crate::now_iso()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object shape")
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array shape")
}

fn state_path(root: &Path) -> PathBuf {
    root.join(TERMINAL_STATE_REL)
}

fn normalize_session_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 120).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn default_state() -> Value {
    json!({
        "type": "infring_dashboard_terminal_broker",
        "updated_at": now_iso(),
        "sessions": {},
        "history": []
    })
}

fn load_state(root: &Path) -> Value {
    let mut state = read_json(&state_path(root)).unwrap_or_else(default_state);
    if !state.is_object() {
        state = default_state();
    }
    let _ = as_object_mut(&mut state, "sessions");
    let _ = as_array_mut(&mut state, "history");
    state
}

fn save_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&state_path(root), &state);
}

fn resolve_cwd(root: &Path, requested: &str) -> PathBuf {
    let text = clean_text(requested, 240);
    if text.is_empty() {
        return root.to_path_buf();
    }
    if text == "/workspace" || text == "/workspace/" {
        return root.to_path_buf();
    }
    if let Some(rest) = text.strip_prefix("/workspace/") {
        let mut normalized = PathBuf::new();
        for component in Path::new(rest).components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    let _ = normalized.pop();
                }
                Component::Normal(part) => normalized.push(part),
                Component::Prefix(_) | Component::RootDir => {}
            }
        }
        return root.join(normalized);
    }
    let candidate = PathBuf::from(&text);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn cwd_allowed(root: &Path, cwd: &Path) -> bool {
    cwd.starts_with(root)
}

fn utf8_prefix_by_bytes(text: &str, max_bytes: usize) -> &str {
    if text.as_bytes().len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn utf8_suffix_by_bytes(text: &str, max_bytes: usize) -> &str {
    if text.as_bytes().len() <= max_bytes {
        return text;
    }
    let mut start = text.len().saturating_sub(max_bytes);
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

fn truncate_output(text: &str) -> String {
    if text.as_bytes().len() <= OUTPUT_MAX_BYTES {
        return text.to_string();
    }
    let marker = OUTPUT_TRUNCATION_MARKER;
    let marker_len = marker.as_bytes().len();
    if OUTPUT_MAX_BYTES <= marker_len + 8 {
        return utf8_suffix_by_bytes(text, OUTPUT_MAX_BYTES).to_string();
    }
    let budget = OUTPUT_MAX_BYTES - marker_len;
    let head_budget = budget / 2;
    let tail_budget = budget - head_budget;
    let head = utf8_prefix_by_bytes(text, head_budget);
    let tail = utf8_suffix_by_bytes(text, tail_budget);
    if head.len() + tail.len() >= text.len() {
        return text.to_string();
    }
    let mut truncated = String::with_capacity(OUTPUT_MAX_BYTES);
    truncated.push_str(head);
    truncated.push_str(marker);
    truncated.push_str(tail);
    if truncated.as_bytes().len() <= OUTPUT_MAX_BYTES {
        return truncated;
    }
    let strict_budget = OUTPUT_MAX_BYTES - marker_len;
    let strict_head = utf8_prefix_by_bytes(text, strict_budget / 2);
    let strict_tail = utf8_suffix_by_bytes(text, strict_budget - strict_head.as_bytes().len());
    format!("{strict_head}{marker}{strict_tail}")
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => matches!(
            clean_text(&raw, 40).to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => fallback,
    }
}

fn primitives_enabled() -> bool {
    bool_env("INFRING_TURN_LOOP_PRIMITIVES_ENABLED", true)
}

fn pre_tool_gate_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_PRE_GATE_ENABLED", true)
}

fn post_tool_filter_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_POST_FILTER_ENABLED", true)
}

fn tracking_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_TRACKING_ENABLED", true)
}

fn tool_summary_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_SUMMARY_ENABLED", true)
}

fn recovery_hints_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_RECOVERY_HINTS_ENABLED", true)
}

fn extract_rule(raw: &str) -> String {
    let cleaned = clean_text(raw, 320);
    if let Some(inner) = cleaned.strip_prefix("Bash(") {
        if let Some(pattern) = inner.strip_suffix(')') {
            return clean_text(pattern, 240);
        }
    }
    clean_text(&cleaned, 240)
}

fn rules_from_value(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(extract_rule))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn default_deny_rules() -> Vec<String> {
    vec![
        "rm -rf /".to_string(),
        "rm -rf /*".to_string(),
        "sudo rm -rf /".to_string(),
        "git reset --hard*".to_string(),
        "git checkout -- *".to_string(),
        "shutdown*".to_string(),
        "reboot*".to_string(),
    ]
}

fn default_ask_rules() -> Vec<String> {
    vec![
        "git push*".to_string(),
        "gh pr create*".to_string(),
        "gh repo create*".to_string(),
        "curl *".to_string(),
        "wget *".to_string(),
        "scp *".to_string(),
        "ssh *".to_string(),
    ]
}

fn load_permission_rules(root: &Path, request: &Value) -> (Vec<String>, Vec<String>) {
    let mut deny = default_deny_rules();
    let mut ask = default_ask_rules();
    if let Some(policy) = read_json(&root.join(TERMINAL_PERMISSION_POLICY_REL)) {
        deny.extend(rules_from_value(
            policy
                .get("deny_rules")
                .or_else(|| policy.pointer("/permissions/deny")),
        ));
        ask.extend(rules_from_value(
            policy
                .get("ask_rules")
                .or_else(|| policy.pointer("/permissions/ask")),
        ));
    }
    deny.extend(rules_from_value(
        request
            .get("deny_rules")
            .or_else(|| request.pointer("/permissions/deny")),
    ));
    ask.extend(rules_from_value(
        request
            .get("ask_rules")
            .or_else(|| request.pointer("/permissions/ask")),
    ));
    deny.sort();
    deny.dedup();
    ask.sort();
    ask.dedup();
    (deny, ask)
}

fn permission_gate_payload(root: &Path, request: &Value, command: &str) -> Value {
    let (deny_rules, ask_rules) = load_permission_rules(root, request);
    let (verdict, matched) =
        crate::command_permission_kernel::evaluate_command_permission_for_kernel(
            command,
            &deny_rules,
            &ask_rules,
        );
    json!({
        "verdict": verdict.as_str(),
        "matched": matched,
        "deny_rules_count": deny_rules.len(),
        "ask_rules_count": ask_rules.len()
    })
}

fn output_tokens_estimate(stdout: &str, stderr: &str) -> usize {
    (stdout.len() + stderr.len()) / 4
}

fn command_recovery_hints(command: &str, exit_code: i64, permission_verdict: &str) -> Vec<String> {
    let mut hints = Vec::<String>::new();
    if permission_verdict == "deny" {
        hints.push(
            "Blocked by command policy. Try a safer read-only command or explicit approval."
                .to_string(),
        );
    } else if permission_verdict == "ask" {
        hints.push("Confirmation required for this command before execution.".to_string());
    }
    let detail =
        crate::session_command_discovery_kernel::classify_command_detail_for_kernel(command);
    if detail
        .get("ignored")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        hints.push(
            "This command often returns little output. Add `&& ls` to verify context.".to_string(),
        );
    } else if !detail
        .get("supported")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let base = clean_text(
            detail
                .get("base_command")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        if !base.is_empty() {
            hints.push(format!(
                "Unsupported `{base}` flow detected; consider `infring {base}` or `/help`."
            ));
        }
    } else if let Some(canonical) = detail.get("canonical").and_then(Value::as_str) {
        let canonical_clean = clean_text(canonical, 120);
        if !canonical_clean.is_empty() {
            hints.push(format!(
                "Try `{canonical_clean}` for a deterministic wrapper lane."
            ));
        }
    }
    if exit_code != 0 {
        hints.push(
            "Command exited non-zero. Re-run with `--help` or inspect stderr details.".to_string(),
        );
    }
    hints.truncate(3);
    hints
}

fn apply_post_tool_output_filter(
    stdout: String,
    stderr: String,
) -> (String, String, Vec<String>, bool) {
    if !post_tool_filter_enabled() {
        return (stdout, stderr, Vec::new(), false);
    }
    let mut stdout_out = stdout;
    let mut stderr_out = stderr;
    let mut filter_events = Vec::<String>::new();
    let mut low_signal = false;

    let stdout_trimmed = clean_text(&stdout_out, 8_000);
    if crate::tool_output_match_filter::matches_ack_placeholder(&stdout_trimmed) {
        filter_events.push("stdout_ack_placeholder".to_string());
        stdout_out.clear();
        low_signal = true;
    } else if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&stdout_trimmed)
    {
        stdout_out = rewritten;
        filter_events.push(format!("stdout_rewrite:{rule_id}"));
    }

    let stderr_trimmed = clean_text(&stderr_out, 8_000);
    if crate::tool_output_match_filter::matches_ack_placeholder(&stderr_trimmed) {
        filter_events.push("stderr_ack_placeholder".to_string());
        stderr_out.clear();
        low_signal = true;
    } else if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&stderr_trimmed)
    {
        stderr_out = rewritten;
        filter_events.push(format!("stderr_rewrite:{rule_id}"));
    }

    if clean_text(&stdout_out, 200).is_empty() && clean_text(&stderr_out, 200).is_empty() {
        low_signal = true;
    }
    (stdout_out, stderr_out, filter_events, low_signal)
}

fn maybe_track_command(
    root: &Path,
    session_id: &str,
    command: &str,
    output_tokens: usize,
) -> Option<Value> {
    if !tracking_enabled() {
        return None;
    }
    let payload = json!({
        "session_id": clean_text(session_id, 120),
        "records": [
            {
                "session_id": clean_text(session_id, 120),
                "command": clean_text(command, 3000),
                "output_tokens": output_tokens as u64
            }
        ]
    });
    crate::session_command_tracking_kernel::record_batch_for_kernel(root, &payload).ok()
}

fn build_tool_summary(
    status: &str,
    cwd: &Path,
    requested_command: &str,
    executed_command: &str,
    command_translated: bool,
    translation_reason: &str,
    permission_gate: &Value,
    exit_code: i64,
    duration_ms: i64,
    stdout: &str,
    stderr: &str,
    filter_events: &[String],
    low_signal: bool,
    recovery_hints: &[String],
) -> Value {
    let found = match (
        !clean_text(stdout, 200).is_empty(),
        !clean_text(stderr, 200).is_empty(),
    ) {
        (true, true) => "stdout+stderr",
        (true, false) => "stdout",
        (false, true) => "stderr",
        (false, false) => "none",
    };
    let mut out = json!({
        "status": clean_text(status, 40),
        "cwd": cwd.to_string_lossy().to_string(),
        "requested_command": clean_text(requested_command, 4000),
        "executed_command": clean_text(executed_command, 4000),
        "command_translated": command_translated,
        "translation_reason": clean_text(translation_reason, 240),
        "permission_verdict": clean_text(
            permission_gate.get("verdict").and_then(Value::as_str).unwrap_or("allow"),
            40
        ),
        "permission_matches": permission_gate
            .get("matched")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "found": found,
        "low_signal": low_signal,
        "filter_events": filter_events,
        "recovery_hints": recovery_hints
    });
    if status == "blocked" {
        out["blocked"] = Value::Bool(true);
        out["blocked_reason"] = Value::String(clean_text(
            permission_gate
                .get("verdict")
                .and_then(Value::as_str)
                .unwrap_or("policy"),
            40,
        ));
    }
    out
}

fn memory_context_verify_command() -> String {
    [
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.1",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.2",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.3",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.4",
        "protheus-ops runtime-systems verify --system-id=V6-MEMORY-CONTEXT-001.5",
    ]
    .join(" && ")
}

fn default_router_suggestions() -> Vec<String> {
    vec![
        "protheus-ops daemon-control diagnostics".to_string(),
        "protheus-ops status --dashboard".to_string(),
        "protheus-ops attention-queue compact --retain=256".to_string(),
        memory_context_verify_command(),
    ]
}

pub fn resolve_operator_command(command: &str) -> Result<CommandResolution, Value> {
    let requested = clean_text(command, 4000);
    if requested.is_empty() {
        return Err(json!({"ok": false, "error": "command_required"}));
    }
    let lowered = requested.to_ascii_lowercase();

    if lowered.starts_with("protheus-ops diagnostic full-scan")
        || lowered.starts_with("protheus-ops diagnostic")
    {
        let resolved = "protheus-ops daemon-control diagnostics && protheus-ops status --dashboard"
            .to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_unsupported_diagnostic_surface_to_daemon_diagnostics"
                .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("protheus-ops queue optimize") {
        let retain = if lowered.contains("--strategy=aggressive") {
            128
        } else {
            256
        };
        let resolved =
            format!("protheus-ops attention-queue compact --retain={retain} && protheus-ops attention-queue status");
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason:
                "translated_unsupported_queue_optimize_surface_to_attention_queue_compact"
                    .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("infring memory-context validate")
        || lowered.starts_with("protheus-ops memory-context validate")
    {
        let resolved = memory_context_verify_command();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason:
                "translated_unsupported_memory_context_validate_surface_to_runtime_system_verify"
                    .to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered == "infring"
        || lowered == "infring help"
        || lowered == "infring --help"
        || lowered == "infring -h"
    {
        let resolved = "protheus-ops command-list-kernel --mode=help".to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_infring_help_surface_to_command_list_help".to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered == "protheus-ops help"
        || lowered == "protheus-ops --help"
        || lowered == "protheus-ops -h"
    {
        let resolved = "protheus-ops command-list-kernel --mode=help".to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason: "translated_protheus_help_surface_to_command_list_help".to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered.starts_with("infring ") {
        let suffix = requested
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or("");
        if suffix.is_empty() {
            return Err(json!({
                "ok": false,
                "error": "command_required",
                "message": "infring command requires a subcommand",
                "requested_command": requested,
                "suggestions": default_router_suggestions()
            }));
        }
        let translated = format!("protheus-ops {suffix}");
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: translated.clone(),
            translated: true,
            translation_reason: "translated_infring_cli_alias_to_protheus_ops".to_string(),
            suggestions: vec![translated],
        });
    }

    if lowered.starts_with("protheus-ops ") && lowered.contains("full-scan") {
        return Err(json!({
            "ok": false,
            "error": "unsupported_protheus_ops_command_variant",
            "requested_command": requested,
            "suggestions": default_router_suggestions()
        }));
    }

    Ok(CommandResolution {
        requested_command: requested.clone(),
        resolved_command: requested,
        translated: false,
        translation_reason: "passthrough_shell_command".to_string(),
        suggestions: Vec::new(),
    })
}

pub fn sessions_payload(root: &Path) -> Value {
    let state = load_state(root);
    let mut rows = state
        .get("sessions")
        .and_then(Value::as_object)
        .map(|obj| obj.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    json!({"ok": true, "sessions": rows})
}

pub fn create_session(root: &Path, request: &Value) -> Value {
    let requested_id = clean_text(request.get("id").and_then(Value::as_str).unwrap_or(""), 120);
    let mut session_id = if requested_id.is_empty() {
        format!(
            "term-{}",
            crate::deterministic_receipt_hash(&json!({"ts": now_iso()}))
                .chars()
                .take(12)
                .collect::<String>()
        )
    } else {
        normalize_session_id(&requested_id)
    };
    if session_id.is_empty() {
        session_id = "term-default".to_string();
    }
    let cwd = resolve_cwd(
        root,
        request.get("cwd").and_then(Value::as_str).unwrap_or(""),
    );
    if !cwd_allowed(root, &cwd) {
        return json!({"ok": false, "error": "cwd_outside_workspace"});
    }
    let mut state = load_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    sessions.insert(
        session_id.clone(),
        json!({
            "id": session_id,
            "cwd": cwd.to_string_lossy().to_string(),
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "last_exit_code": Value::Null,
            "last_output": ""
        }),
    );
    let out = sessions
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_create", "session": out})
}

pub fn close_session(root: &Path, session_id: &str) -> Value {
    let sid = normalize_session_id(session_id);
    if sid.is_empty() {
        return json!({"ok": false, "error": "session_id_required"});
    }
    let mut state = load_state(root);
    let removed = as_object_mut(&mut state, "sessions").remove(&sid).is_some();
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_close", "session_id": sid, "removed": removed})
}

pub fn exec_command(root: &Path, request: &Value) -> Value {
    let sid = normalize_session_id(
        request
            .get("session_id")
            .or_else(|| request.get("sessionId"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let command = request
        .get("command")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 4000))
        .unwrap_or_default();
    if sid.is_empty() || command.is_empty() {
        return json!({"ok": false, "error": "session_id_and_command_required"});
    }
    let resolution = match resolve_operator_command(&command) {
        Ok(resolution) => resolution,
        Err(mut err) => {
            err["session_id"] = Value::String(sid.clone());
            return err;
        }
    };
    let requested_command = resolution.requested_command.clone();
    let executed_command = resolution.resolved_command.clone();
    let command_translated = resolution.translated;
    let translation_reason = resolution.translation_reason.clone();
    let suggestions = resolution.suggestions.clone();

    let mut state = load_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    let Some(session) = sessions.get_mut(&sid) else {
        return json!({"ok": false, "error": "session_not_found", "session_id": sid});
    };
    let cwd = resolve_cwd(
        root,
        request
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or_else(|| session.get("cwd").and_then(Value::as_str).unwrap_or("")),
    );
    if !cwd_allowed(root, &cwd) {
        return json!({"ok": false, "error": "cwd_outside_workspace"});
    }
    let permission_gate = if pre_tool_gate_enabled() {
        permission_gate_payload(root, request, &executed_command)
    } else {
        json!({"verdict":"allow","matched":[],"deny_rules_count":0,"ask_rules_count":0})
    };
    let permission_verdict = clean_text(
        permission_gate
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("allow"),
        40,
    )
    .to_ascii_lowercase();
    let started = Instant::now();
    if pre_tool_gate_enabled() && (permission_verdict == "deny" || permission_verdict == "ask") {
        let blocked_error = if permission_verdict == "ask" {
            "permission_confirmation_required"
        } else {
            "permission_denied_by_policy"
        };
        let blocked_message = if permission_verdict == "ask" {
            "Command requires confirmation before execution."
        } else {
            "Command blocked by terminal command policy."
        };
        let recovery_hints = if recovery_hints_enabled() {
            command_recovery_hints(&executed_command, 126, &permission_verdict)
        } else {
            Vec::new()
        };
        let tracking = maybe_track_command(root, &sid, &executed_command, 0);
        let tool_summary = if tool_summary_enabled() {
            build_tool_summary(
                "blocked",
                &cwd,
                &requested_command,
                &executed_command,
                command_translated,
                &translation_reason,
                &permission_gate,
                126,
                started.elapsed().as_millis() as i64,
                "",
                "",
                &[],
                true,
                &recovery_hints,
            )
        } else {
            Value::Null
        };

        session["cwd"] = Value::String(cwd.to_string_lossy().to_string());
        session["updated_at"] = Value::String(now_iso());
        session["last_exit_code"] = json!(126);
        session["last_output"] = Value::String(String::new());
        session["last_error"] = Value::String(blocked_message.to_string());
        session["last_requested_command"] = Value::String(requested_command.clone());
        session["last_executed_command"] = Value::String(executed_command.clone());
        session["last_command_translated"] = Value::Bool(command_translated);
        session["last_translation_reason"] = Value::String(translation_reason.clone());
        session["last_permission_verdict"] = Value::String(permission_verdict.clone());

        let history = as_array_mut(&mut state, "history");
        history.push(json!({
            "session_id": sid,
            "ts": now_iso(),
            "command": requested_command,
            "requested_command": requested_command,
            "executed_command": executed_command,
            "translated": command_translated,
            "translation_reason": translation_reason,
            "permission_verdict": permission_verdict,
            "exit_code": 126,
            "ok": false,
            "blocked": true
        }));
        if history.len() > 500 {
            let drain = history.len() - 500;
            history.drain(0..drain);
        }
        save_state(root, state);
        return json!({
            "ok": false,
            "type": "dashboard_terminal_exec",
            "error": blocked_error,
            "message": blocked_message,
            "blocked": true,
            "session_id": request.get("session_id").or_else(|| request.get("sessionId")).cloned().unwrap_or_else(|| Value::String(String::new())),
            "exit_code": 126,
            "requested_command": requested_command,
            "executed_command": executed_command,
            "command_translated": command_translated,
            "translation_reason": translation_reason,
            "suggestions": suggestions,
            "stdout": "",
            "stderr": "",
            "permission_gate": permission_gate,
            "recovery_hints": recovery_hints,
            "tool_summary": tool_summary,
            "tracking": tracking.unwrap_or(Value::Null)
        });
    }

    let output = Command::new("zsh")
        .arg("-lc")
        .arg(&executed_command)
        .current_dir(&cwd)
        .output();

    let (ok, code, stdout, stderr) = match output {
        Ok(out) => (
            out.status.success(),
            out.status.code().unwrap_or(1),
            truncate_output(&String::from_utf8_lossy(&out.stdout)),
            truncate_output(&String::from_utf8_lossy(&out.stderr)),
        ),
        Err(err) => (
            false,
            127,
            String::new(),
            clean_text(&err.to_string(), 2000),
        ),
    };
    let (filtered_stdout, filtered_stderr, filter_events, mut low_signal) =
        apply_post_tool_output_filter(stdout, stderr);
    if clean_text(&filtered_stdout, 200).is_empty() && clean_text(&filtered_stderr, 200).is_empty()
    {
        low_signal = true;
    }
    let recovery_hints = if recovery_hints_enabled() && (low_signal || code != 0) {
        command_recovery_hints(&executed_command, code as i64, &permission_verdict)
    } else {
        Vec::new()
    };
    let tracking = maybe_track_command(
        root,
        &sid,
        &executed_command,
        output_tokens_estimate(&filtered_stdout, &filtered_stderr),
    );
    let tool_summary = if tool_summary_enabled() {
        build_tool_summary(
            if ok { "ok" } else { "error" },
            &cwd,
            &requested_command,
            &executed_command,
            command_translated,
            &translation_reason,
            &permission_gate,
            code as i64,
            started.elapsed().as_millis() as i64,
            &filtered_stdout,
            &filtered_stderr,
            &filter_events,
            low_signal,
            &recovery_hints,
        )
    } else {
        Value::Null
    };

    session["cwd"] = Value::String(cwd.to_string_lossy().to_string());
    session["updated_at"] = Value::String(now_iso());
    session["last_exit_code"] = json!(code);
    session["last_output"] = Value::String(filtered_stdout.clone());
    session["last_error"] = Value::String(filtered_stderr.clone());
    session["last_requested_command"] = Value::String(requested_command.clone());
    session["last_executed_command"] = Value::String(executed_command.clone());
    session["last_command_translated"] = Value::Bool(command_translated);
    session["last_translation_reason"] = Value::String(translation_reason.clone());
    session["last_permission_verdict"] = Value::String(permission_verdict.clone());
    session["last_filter_events"] = Value::Array(
        filter_events
            .iter()
            .map(|row| Value::String(clean_text(row, 120)))
            .collect::<Vec<_>>(),
    );

    let history = as_array_mut(&mut state, "history");
    history.push(json!({
        "session_id": sid,
        "ts": now_iso(),
        "command": requested_command,
        "requested_command": requested_command,
        "executed_command": executed_command,
        "translated": command_translated,
        "translation_reason": translation_reason,
        "permission_verdict": permission_verdict,
        "exit_code": code,
        "ok": ok,
        "low_signal": low_signal
    }));
    if history.len() > 500 {
        let drain = history.len() - 500;
        history.drain(0..drain);
    }
    save_state(root, state);
    json!({
        "ok": ok,
        "type": "dashboard_terminal_exec",
        "session_id": request.get("session_id").or_else(|| request.get("sessionId")).cloned().unwrap_or_else(|| Value::String(String::new())),
        "exit_code": code,
        "requested_command": requested_command,
        "executed_command": executed_command,
        "command_translated": command_translated,
        "translation_reason": translation_reason,
        "suggestions": suggestions,
        "stdout": filtered_stdout,
        "stderr": filtered_stderr,
        "permission_gate": permission_gate,
        "filter_events": filter_events,
        "low_signal_output": low_signal,
        "recovery_hints": recovery_hints,
        "tool_summary": tool_summary,
        "duration_ms": started.elapsed().as_millis() as i64,
        "cwd": cwd.to_string_lossy().to_string(),
        "tracking": tracking.unwrap_or(Value::Null)
    })
}

pub fn handle_http(root: &Path, method: &str, path: &str, body: &[u8]) -> Option<Value> {
    if method == "GET" && path == "/api/terminal/sessions" {
        return Some(sessions_payload(root));
    }
    if method == "POST" && path == "/api/terminal/sessions" {
        return Some(create_session(root, &parse_json(body)));
    }
    if method == "POST" && path == "/api/terminal/queue" {
        return Some(exec_command(root, &parse_json(body)));
    }
    if method == "DELETE" && path.starts_with("/api/terminal/sessions/") {
        let sid = path.trim_start_matches("/api/terminal/sessions/");
        return Some(close_session(root, sid));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_session_create_and_list() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = create_session(root.path(), &json!({"id":"term-a"}));
        assert_eq!(created.get("ok").and_then(Value::as_bool), Some(true));
        let rows = sessions_payload(root.path())
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn terminal_exec_returns_stdout() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"printf 'hello'"}),
        );
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(0));
        assert_eq!(out.get("stdout").and_then(Value::as_str), Some("hello"));
        assert_eq!(
            out.get("requested_command").and_then(Value::as_str),
            Some("printf 'hello'")
        );
        assert_eq!(
            out.get("executed_command").and_then(Value::as_str),
            Some("printf 'hello'")
        );
        assert_eq!(
            out.get("command_translated").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn terminal_exec_blocks_cwd_escape() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"pwd","cwd":"/"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("cwd_outside_workspace")
        );
    }

    #[test]
    fn terminal_exec_pre_tool_gate_blocks_denied_command() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"git reset --hard HEAD"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("permission_denied_by_policy")
        );
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(126));
        assert_eq!(
            out.pointer("/permission_gate/verdict")
                .and_then(Value::as_str),
            Some("deny")
        );
    }

    #[test]
    fn terminal_exec_post_tool_filter_suppresses_ack_placeholder() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"printf 'Web search completed.'"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("stdout").and_then(Value::as_str).unwrap_or(""), "");
        assert_eq!(
            out.get("low_signal_output").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn terminal_exec_accepts_workspace_virtual_cwd_alias() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a", "cwd": "/workspace"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"pwd","cwd":"/workspace"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(0));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            None,
            "workspace alias should not be rejected as outside the workspace root"
        );
    }

    #[test]
    fn command_router_translates_diagnostic_surface() {
        let out = resolve_operator_command(
            "protheus-ops diagnostic full-scan --priority=critical --output=telemetry-now",
        )
        .expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "protheus-ops daemon-control diagnostics && protheus-ops status --dashboard"
        );
        assert_eq!(
            out.translation_reason,
            "translated_unsupported_diagnostic_surface_to_daemon_diagnostics"
        );
    }

    #[test]
    fn command_router_translates_queue_optimize_aggressive() {
        let out = resolve_operator_command(
            "protheus-ops queue optimize --strategy=aggressive --clean-orphaned=true",
        )
        .expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "protheus-ops attention-queue compact --retain=128 && protheus-ops attention-queue status"
        );
    }

    #[test]
    fn command_router_translates_infring_alias_to_core_binary() {
        let out = resolve_operator_command("infring daemon ping").expect("translation");
        assert!(out.translated);
        assert_eq!(out.resolved_command, "protheus-ops daemon ping");
        assert_eq!(
            out.translation_reason,
            "translated_infring_cli_alias_to_protheus_ops"
        );
    }

    #[test]
    fn command_router_translates_infring_help_surface_to_usage() {
        let out = resolve_operator_command("infring --help").expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "protheus-ops command-list-kernel --mode=help"
        );
        assert_eq!(
            out.translation_reason,
            "translated_infring_help_surface_to_command_list_help"
        );
    }

    #[test]
    fn truncate_output_preserves_head_and_tail_context() {
        let text = format!(
            "head-marker:{}:{}tail-marker",
            "x".repeat(OUTPUT_MAX_BYTES),
            "y".repeat(OUTPUT_MAX_BYTES)
        );
        let out = truncate_output(&text);
        assert!(out.contains("head-marker"));
        assert!(out.contains("tail-marker"));
        assert!(out.contains("... (output truncated) ..."));
        assert!(out.as_bytes().len() <= OUTPUT_MAX_BYTES);
    }

    #[test]
    fn truncate_output_handles_utf8_boundaries() {
        let text = format!("前置{}后置", "界".repeat(OUTPUT_MAX_BYTES));
        let out = truncate_output(&text);
        assert!(out.contains("后置"));
        assert!(out.contains("... (output truncated) ..."));
        assert!(out.as_bytes().len() <= OUTPUT_MAX_BYTES);
    }
}
