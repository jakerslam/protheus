use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DASHBOARD_STATE_ROOT_ENV: &str = "INFRING_TOOLING_DASHBOARD_STATE_ROOT";
const AGENT_SESSIONS_SUBDIR: &str = "agent_sessions";

#[derive(Clone, Debug, Default)]
struct SessionSnapshot {
    total_messages: usize,
    assistant_messages: usize,
}

pub(super) fn load_responses_by_case(path: &str) -> BTreeMap<String, Value> {
    let input = read_json(path);
    let rows = input
        .get("responses")
        .or_else(|| input.get("cases"))
        .or_else(|| input.get("turns"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut by_case = BTreeMap::new();
    for row in rows {
        let case_id = str_at(&row, &["case_id"], &str_at(&row, &["id"], ""));
        if case_id.is_empty() {
            continue;
        }
        let payload = row
            .get("response_payload")
            .or_else(|| row.get("payload"))
            .or_else(|| row.get("mock_response"))
            .cloned()
            .unwrap_or(row);
        by_case.insert(case_id, payload);
    }
    by_case
}

pub(super) fn response_sequence_payload(source: &Value, index: usize) -> Option<Value> {
    source
        .get("response_sequence")
        .or_else(|| source.get("__research_golden_response_sequence"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .cloned()
}

pub(super) fn payload_has_pending_tool_confirmation(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|status| status == "pending_confirmation")
            .unwrap_or(false)
    })
}

pub(super) fn create_live_agent(
    base_url: &str,
    case_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Option<String> {
    let name = format!("Research Golden {}", clean_text(case_id, 80));
    let payload = post_json(
        base_url,
        "/api/agents",
        &json!({
            "name": name,
            "role": "analyst"
        }),
        timeout_seconds,
    );
    str_opt(&payload, &["agent_id"])
        .or_else(|| str_opt(&payload, &["id"]))
        .map(normalize_agent_id)
        .filter(|agent_id| !agent_id.is_empty())
        .and_then(|agent_id| {
            if set_live_agent_model(base_url, &agent_id, model_ref, timeout_seconds) {
                Some(agent_id)
            } else {
                None
            }
        })
}

fn set_live_agent_model(
    base_url: &str,
    agent_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> bool {
    let Some(model_ref) = model_ref.map(|raw| clean_text(raw, 240)) else {
        return true;
    };
    if model_ref.is_empty() {
        return true;
    }
    let resolved_model_ref =
        resolve_live_agent_model_ref(base_url, &model_ref, timeout_seconds);
    let response = curl_json(
        "PUT",
        base_url,
        &format!("/api/agents/{agent_id}/model"),
        &json!({ "model": resolved_model_ref }),
        timeout_seconds,
    );
    response.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

fn resolve_live_agent_model_ref(base_url: &str, model_ref: &str, timeout_seconds: u64) -> String {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() || cleaned.contains('/') {
        return cleaned;
    }
    let catalog = curl_json("GET", base_url, "/api/models", &json!({}), timeout_seconds);
    resolve_live_agent_model_ref_from_catalog(&cleaned, &catalog)
}

fn resolve_live_agent_model_ref_from_catalog(model_ref: &str, catalog: &Value) -> String {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() || cleaned.contains('/') {
        return cleaned;
    }
    let Some(models) = catalog.get("models").and_then(Value::as_array) else {
        return cleaned;
    };

    let mut exact_available_non_auto = None::<String>;
    let mut exact_non_auto = None::<String>;
    for row in models {
        let provider = clean_text(
            row.get("provider")
                .or_else(|| row.get("runtime_provider"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model = clean_text(
            row.get("model")
                .or_else(|| row.get("runtime_model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let matches_requested = model == cleaned
            || clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 300) == cleaned;
        if !matches_requested || provider.is_empty() || provider.eq_ignore_ascii_case("auto") {
            continue;
        }
        let resolved = format!("{provider}/{model}");
        if row.get("available").and_then(Value::as_bool).unwrap_or(false) {
            exact_available_non_auto = Some(resolved);
            break;
        }
        if exact_non_auto.is_none() {
            exact_non_auto = Some(resolved);
        }
    }

    exact_available_non_auto.or(exact_non_auto).unwrap_or(cleaned)
}

pub(super) fn delete_live_agent(base_url: &str, agent_id: &str, timeout_seconds: u64) -> Value {
    curl_json(
        "DELETE",
        base_url,
        &format!("/api/agents/{agent_id}"),
        &json!({}),
        timeout_seconds,
    )
}

pub(super) fn post_agent_message(
    base_url: &str,
    agent_id: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let path = format!("/api/agents/{agent_id}/message");
    let timeout_recovery = dashboard_state_root().map(|root| {
        (
            root.clone(),
            session_snapshot_from_state_root(&root, agent_id),
        )
    });
    let response = post_json(base_url, &path, request, timeout_seconds);
    if is_retryable_curl_timeout(&response) {
        if let Some((dashboard_state_root, baseline_snapshot)) = timeout_recovery.as_ref() {
            if let Some(recovered) = recover_timed_out_response_from_state(
                dashboard_state_root,
                agent_id,
                baseline_snapshot,
                timeout_seconds,
            ) {
                return recovered;
            }
        }
        return structured_timeout_failure_payload(response, timeout_recovery.is_some());
    }
    response
}

fn post_json(base_url: &str, path: &str, request: &Value, timeout_seconds: u64) -> Value {
    curl_json("POST", base_url, path, request, timeout_seconds)
}

fn is_retryable_curl_timeout(payload: &Value) -> bool {
    payload.get("transport_error").and_then(Value::as_str) == Some("curl_failed")
        && payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|stderr| stderr.to_ascii_lowercase().contains("timed out"))
            .unwrap_or(false)
}

fn structured_timeout_failure_payload(response: Value, recovery_ready: bool) -> Value {
    let mut payload = if response.is_object() {
        response
    } else {
        json!({
            "ok": false,
            "raw_transport_payload": response
        })
    };
    let response_text = "The live dashboard request timed out before the workflow produced a final answer. This is a transport failure, not a research result.";
    if let Some(object) = payload.as_object_mut() {
        object.insert("ok".to_string(), Value::Bool(false));
        object.insert(
            "response".to_string(),
            Value::String(response_text.to_string()),
        );
        object.insert("timeout_recovery_attempted".to_string(), Value::Bool(true));
        object.insert(
            "timeout_recovery_source".to_string(),
            Value::String("agent_session_state".to_string()),
        );
        object.insert(
            "timeout_recovery_ready".to_string(),
            Value::Bool(recovery_ready),
        );
        object.insert(
            "response_finalization".to_string(),
            json!({
                "outcome": "structured_failure+transport_timeout+timeout_recovery_failed",
                "structured_failure": {
                    "kind": "transport_timeout",
                    "reason": "live_dashboard_request_timed_out_before_final_answer",
                    "retryable": true,
                    "source": "eval_transport"
                }
            }),
        );
        object.insert(
            "response_workflow".to_string(),
            json!({
                "final_llm_response": {
                    "status": "transport_timeout",
                    "attempted": false,
                    "used": false
                }
            }),
        );
    }
    payload
}

fn dashboard_state_root() -> Option<PathBuf> {
    if let Ok(raw) = env::var(DASHBOARD_STATE_ROOT_ENV) {
        let candidate = PathBuf::from(raw.trim());
        if !candidate.as_os_str().is_empty() && candidate.exists() {
            return Some(candidate);
        }
    }
    let candidate = repo_root().join("client/runtime/local/state/ui/infring_dashboard");
    candidate.exists().then_some(candidate)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn agent_sessions_dir(dashboard_state_root: &Path) -> PathBuf {
    if dashboard_state_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == AGENT_SESSIONS_SUBDIR)
        .unwrap_or(false)
    {
        return dashboard_state_root.to_path_buf();
    }
    dashboard_state_root.join(AGENT_SESSIONS_SUBDIR)
}

fn session_path_from_state_root(dashboard_state_root: &Path, agent_id: &str) -> PathBuf {
    agent_sessions_dir(dashboard_state_root).join(format!("{}.json", normalize_agent_id(agent_id)))
}

fn session_messages_from_state_root(dashboard_state_root: &Path, agent_id: &str) -> Vec<Value> {
    let state = read_json_path(&session_path_from_state_root(dashboard_state_root, agent_id));
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = sessions
        .iter()
        .find(|session| {
            clean_text(
                session
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            ) == active_id
        })
        .or_else(|| sessions.first());
    row.and_then(|session| session.get("messages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_snapshot_from_state_root(
    dashboard_state_root: &Path,
    agent_id: &str,
) -> SessionSnapshot {
    let messages = session_messages_from_state_root(dashboard_state_root, agent_id);
    let assistant_messages = messages
        .iter()
        .filter(|row| message_role(row).eq_ignore_ascii_case("assistant"))
        .count();
    SessionSnapshot {
        total_messages: messages.len(),
        assistant_messages,
    }
}

fn recover_timed_out_response_from_state(
    dashboard_state_root: &Path,
    agent_id: &str,
    baseline_snapshot: &SessionSnapshot,
    timeout_seconds: u64,
) -> Option<Value> {
    // Live research turns can keep running after the HTTP client gives up, so
    // give the persisted session state a wider window than the request budget.
    let recovery_budget_seconds = timeout_seconds
        .saturating_mul(2)
        .saturating_add(60)
        .clamp(30, 300);
    let deadline = Instant::now() + Duration::from_secs(recovery_budget_seconds);
    loop {
        if let Some(recovered) = recovered_payload_from_state(
            dashboard_state_root,
            agent_id,
            baseline_snapshot,
        ) {
            return Some(recovered);
        }
        if Instant::now() >= deadline {
            return None;
        }
        sleep(Duration::from_millis(1500));
    }
}

fn recovered_payload_from_state(
    dashboard_state_root: &Path,
    agent_id: &str,
    baseline_snapshot: &SessionSnapshot,
) -> Option<Value> {
    let messages = session_messages_from_state_root(dashboard_state_root, agent_id);
    if messages.len() <= baseline_snapshot.total_messages {
        return None;
    }
    let assistant_rows_seen = messages
        .iter()
        .filter(|row| message_role(row).eq_ignore_ascii_case("assistant"))
        .count();
    if assistant_rows_seen <= baseline_snapshot.assistant_messages {
        return None;
    }
    let assistant_row = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(idx, row)| {
            *idx >= baseline_snapshot.total_messages
                && message_role(row).eq_ignore_ascii_case("assistant")
                && !clean_text(
                    row.get("text")
                        .or_else(|| row.get("content"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    64_000,
                )
                .is_empty()
        })
        .map(|(_, row)| row.clone())?;
    assistant_row_to_payload(&assistant_row)
}

fn assistant_row_to_payload(row: &Value) -> Option<Value> {
    let response_text = row
        .get("text")
        .or_else(|| row.get("content"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 64_000))
        .filter(|raw| !raw.is_empty())?;
    let mut payload = json!({
        "ok": true,
        "response": response_text,
        "text": response_text,
        "message": response_text,
        "recovered_from_timeout": true,
        "recovery_source": "agent_session_state"
    });
    if let Some(object) = row.as_object() {
        for key in [
            "tools",
            "response_workflow",
            "response_finalization",
            "process_summary",
            "workflow_visibility",
            "turn_transaction",
            "terminal_transcript",
            "agent_health_snapshot",
            "live_eval_monitor",
            "dashboard_health_indicator",
        ] {
            if let Some(value) = object.get(key) {
                payload[key] = value.clone();
            }
        }
    }
    if let Some(pending_request) = row
        .pointer("/response_workflow/pending_tool_request")
        .or_else(|| row.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| row.pointer("/response_finalization/pending_tool_request"))
        .cloned()
    {
        payload["pending_tool_request"] = pending_request;
    }
    if let Some(provider) = row
        .pointer("/response_workflow/final_llm_response/provider")
        .and_then(Value::as_str)
    {
        payload["provider"] = Value::String(clean_text(provider, 160));
    }
    if let Some(model) = row
        .pointer("/response_workflow/final_llm_response/model")
        .and_then(Value::as_str)
    {
        payload["model"] = Value::String(clean_text(model, 240));
    }
    if let Some(runtime_model) = row
        .pointer("/response_workflow/final_llm_response/runtime_model")
        .and_then(Value::as_str)
    {
        payload["runtime_model"] = Value::String(clean_text(runtime_model, 240));
    }
    Some(payload)
}

fn read_json_path(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn message_role(row: &Value) -> String {
    let raw = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
        .to_ascii_lowercase();
    if raw.is_empty() {
        "assistant".to_string()
    } else {
        raw
    }
}

fn curl_json(
    method: &str,
    base_url: &str,
    path: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let Ok(body) = serde_json::to_string(request) else {
        return json!({"ok": false, "transport_error": "request_json_encode_failed"});
    };
    let mut child = match Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            &timeout_seconds.to_string(),
            "-H",
            "Content-Type: application/json",
            "-X",
            method,
            "--data-binary",
            "@-",
            &url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return json!({"ok": false, "transport_error": format!("curl_spawn_failed:{err}")})
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(
                |_| json!({"ok": false, "transport_error": "response_json_decode_failed"}),
            ),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 500),
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

pub(super) fn assistant_text(payload: &Value) -> String {
    for path in [
        &["response"][..],
        &["text"][..],
        &["message"][..],
        &["content"][..],
        &["assistant", "text"][..],
    ] {
        if let Some(raw) = str_opt(payload, path) {
            return clean_text(raw, 12_000);
        }
    }
    String::new()
}

pub(super) fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

pub(super) fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    parse_flag(args, key)
        .map(|raw| matches!(raw.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

pub(super) fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

pub(super) fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

pub(super) fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

pub(super) fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

pub(super) fn append_jsonl(path: &str, rows: &[Value]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

pub(super) fn str_opt<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

pub(super) fn str_at(value: &Value, path: &[&str], default: &str) -> String {
    str_opt(value, path).unwrap_or(default).to_string()
}

pub(super) fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 500))
        .collect()
}

pub(super) fn bool_at(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(super) fn u64_at(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_u64)
        .unwrap_or(default)
}

pub(super) fn f64_at(value: &Value, path: &[&str], default: f64) -> f64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_f64)
        .unwrap_or(default)
}

pub(super) fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(super) fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

pub(super) fn normalize_for_compare(raw: &str) -> String {
    clean_text(&raw.to_ascii_lowercase(), 4_000)
}

pub(super) fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

pub(super) fn is_local_dashboard_url(raw: &str) -> bool {
    let lower = raw.trim().to_ascii_lowercase();
    lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
}

pub(super) fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

pub(super) fn print_json_line(value: &Value) {
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(value).unwrap_or_default()
    );
}

#[cfg(test)]
mod eval_research_golden_utils_tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "eval-research-golden-utils-{}-{}",
            name,
            now_iso_like()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn retryable_curl_timeout_matches_timeout_stderr() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (28) Operation timed out after 45010 milliseconds with 0 bytes received"
        });
        assert!(is_retryable_curl_timeout(&payload));
    }

    #[test]
    fn retryable_curl_timeout_rejects_non_timeout_failures() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (7) Failed to connect to 127.0.0.1 port 4173"
        });
        assert!(!is_retryable_curl_timeout(&payload));
    }

    #[test]
    fn timeout_failure_payload_is_structured_terminal_artifact() {
        let payload = structured_timeout_failure_payload(
            json!({
                "ok": false,
                "transport_error": "curl_failed",
                "stderr": "curl: (28) Operation timed out after 60004 milliseconds with 0 bytes received"
            }),
            true,
        );
        let response = payload
            .get("response")
            .and_then(Value::as_str)
            .expect("response");
        assert!(!response.trim().is_empty());
        assert_eq!(
            payload.get("transport_error").and_then(Value::as_str),
            Some("curl_failed")
        );
        assert_eq!(
            payload
                .get("timeout_recovery_attempted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .get("timeout_recovery_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("structured_failure+transport_timeout+timeout_recovery_failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/structured_failure/kind")
                .and_then(Value::as_str),
            Some("transport_timeout")
        );
    }

    #[test]
    fn assistant_row_to_payload_recovers_workflow_metadata() {
        let row = json!({
            "role": "assistant",
            "text": "Recovered answer",
            "tools": [{"name": "web_search", "status": "ok"}],
            "response_workflow": {
                "final_llm_response": {
                    "status": "synthesized",
                    "provider": "ollama",
                    "model": "kimi-k2.6:cloud",
                    "runtime_model": "kimi-k2.6:cloud"
                },
                "pending_tool_request": {"status": "pending_confirmation", "tool_name": "web_search"}
            },
            "response_finalization": {
                "outcome": "workflow_authored+workflow:synthesized"
            },
            "process_summary": {"contract": "turn_process_summary_v1"},
            "workflow_visibility": {"current_stage_status": "synthesized"}
        });
        let payload = assistant_row_to_payload(&row).expect("payload");
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("Recovered answer")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
        assert_eq!(
            payload.get("runtime_model").and_then(Value::as_str),
            Some("kimi-k2.6:cloud")
        );
        assert_eq!(
            payload
                .pointer("/pending_tool_request/tool_name")
                .and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            payload.get("recovered_from_timeout").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn recovered_payload_uses_first_new_assistant_turn_after_baseline() {
        let root = temp_path("session-recovery");
        let sessions_dir = root.join(AGENT_SESSIONS_SUBDIR);
        fs::create_dir_all(&sessions_dir).expect("sessions dir");
        let agent_id = "agent-recovery";
        let session = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-05-08T00:00:00Z",
                "updated_at": "2026-05-08T00:00:00Z",
                "messages": [
                    {"role": "user", "text": "old question"},
                    {"role": "assistant", "text": "old answer", "response_workflow": {"final_llm_response": {"status": "synthesized"}}},
                    {"role": "user", "text": "new question"},
                    {"role": "assistant", "text": "new answer", "response_workflow": {"final_llm_response": {"status": "synthesized", "model": "kimi-k2.6:cloud"}}}
                ]
            }],
            "memory_kv": {}
        });
        fs::write(
            sessions_dir.join(format!("{}.json", agent_id)),
            format!("{}\n", serde_json::to_string_pretty(&session).expect("json")),
        )
        .expect("session write");
        let baseline = SessionSnapshot {
            total_messages: 2,
            assistant_messages: 1,
        };
        let payload =
            recovered_payload_from_state(&root, agent_id, &baseline).expect("recovered");
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("new answer")
        );
        assert_eq!(
            payload.get("model").and_then(Value::as_str),
            Some("kimi-k2.6:cloud")
        );
    }

    #[test]
    fn resolve_live_agent_model_ref_prefers_available_non_auto_provider_match() {
        let catalog = json!({
            "models": [
                {
                    "id": "auto/kimi-k2.6:cloud",
                    "provider": "auto",
                    "model": "kimi-k2.6:cloud",
                    "available": false
                },
                {
                    "id": "ollama/kimi-k2.6:cloud",
                    "provider": "ollama",
                    "model": "kimi-k2.6:cloud",
                    "available": true
                }
            ]
        });
        let resolved =
            resolve_live_agent_model_ref_from_catalog("kimi-k2.6:cloud", &catalog);
        assert_eq!(resolved, "ollama/kimi-k2.6:cloud");
    }

    #[test]
    fn resolve_live_agent_model_ref_keeps_explicit_provider_ref() {
        assert_eq!(
            resolve_live_agent_model_ref_from_catalog(
                "ollama/kimi-k2.6:cloud",
                &json!({"models": []})
            ),
            "ollama/kimi-k2.6:cloud"
        );
    }
}
