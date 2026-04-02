// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const TERMINAL_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/terminal_broker.json";
const OUTPUT_MAX_BYTES: usize = 32 * 1024;

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

fn truncate_output(text: &str) -> String {
    let bytes = text.as_bytes();
    if bytes.len() <= OUTPUT_MAX_BYTES {
        return text.to_string();
    }
    let tail = &bytes[bytes.len() - OUTPUT_MAX_BYTES..];
    String::from_utf8_lossy(tail).to_string()
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
            translation_reason:
                "translated_infring_help_surface_to_command_list_help".to_string(),
            suggestions: vec![resolved],
        });
    }

    if lowered == "protheus-ops help" || lowered == "protheus-ops --help" || lowered == "protheus-ops -h" {
        let resolved = "protheus-ops command-list-kernel --mode=help".to_string();
        return Ok(CommandResolution {
            requested_command: requested,
            resolved_command: resolved.clone(),
            translated: true,
            translation_reason:
                "translated_protheus_help_surface_to_command_list_help".to_string(),
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

    session["cwd"] = Value::String(cwd.to_string_lossy().to_string());
    session["updated_at"] = Value::String(now_iso());
    session["last_exit_code"] = json!(code);
    session["last_output"] = Value::String(stdout.clone());
    session["last_error"] = Value::String(stderr.clone());
    session["last_requested_command"] = Value::String(requested_command.clone());
    session["last_executed_command"] = Value::String(executed_command.clone());
    session["last_command_translated"] = Value::Bool(command_translated);
    session["last_translation_reason"] = Value::String(translation_reason.clone());

    let history = as_array_mut(&mut state, "history");
    history.push(json!({
        "session_id": sid,
        "ts": now_iso(),
        "command": requested_command,
        "requested_command": requested_command,
        "executed_command": executed_command,
        "translated": command_translated,
        "translation_reason": translation_reason,
        "exit_code": code,
        "ok": ok
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
        "stdout": stdout,
        "stderr": stderr
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
}
