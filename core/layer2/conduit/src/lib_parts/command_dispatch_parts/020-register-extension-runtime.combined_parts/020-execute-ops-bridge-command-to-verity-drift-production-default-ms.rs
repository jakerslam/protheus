
fn execute_ops_bridge_command(domain: &str, args: &[String], run_context: Option<&str>) -> Value {
    let root = repo_root_from_current_dir();
    let (command, mut command_args) = resolve_infring_ops_command(&root, domain);
    command_args.extend(args.iter().cloned());
    let timeout_ms = bridge_command_timeout_ms();

    let mut cmd = Command::new(&command);
    cmd.args(&command_args)
        .current_dir(&root)
        .env(
            "INFRING_NODE_BINARY",
            std::env::var("INFRING_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(context) = run_context {
        let trimmed = context.trim();
        if !trimmed.is_empty() {
            cmd.env("SPINE_RUN_CONTEXT", trimmed);
        }
    }

    match cmd.spawn() {
        Ok(mut child) => match child.wait_timeout(Duration::from_millis(timeout_ms)) {
            Ok(Some(status)) => {
                let (stdout, stderr) = collect_child_output(&mut child);
                let exit_code = status.code().unwrap_or(1);
                let spine_receipt = parse_json_payload(&stdout);
                let mut detail = serde_json::json!({
                    "ok": exit_code == 0,
                    "type": if exit_code == 0 {
                        format!("{domain}_bridge_ok")
                    } else {
                        format!("{domain}_bridge_error")
                    },
                    "exit_code": exit_code,
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": stdout,
                    "stderr": stderr,
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                if let Some(receipt) = spine_receipt {
                    detail["domain_receipt"] = receipt.clone();
                    if let Some(kind) = receipt.get("type").and_then(Value::as_str) {
                        detail["type"] = Value::String(kind.to_string());
                    }
                    if let Some(ok) = receipt.get("ok").and_then(Value::as_bool) {
                        detail["ok"] = Value::Bool(ok && exit_code == 0);
                    }
                    if let Some(reason) = receipt.get("reason").and_then(Value::as_str) {
                        detail["reason"] = Value::String(reason.to_string());
                    } else if let Some(reason) =
                        receipt.get("failure_reason").and_then(Value::as_str)
                    {
                        detail["reason"] = Value::String(reason.to_string());
                    }
                }
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let (stdout, stderr) = collect_child_output(&mut child);
                let mut detail = serde_json::json!({
                    "ok": false,
                    "type": format!("{domain}_bridge_timeout"),
                    "exit_code": 124,
                    "reason": format!("{domain}_bridge_timeout:{timeout_ms}"),
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": stdout,
                    "stderr": stderr,
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
            Err(err) => {
                let mut detail = serde_json::json!({
                    "ok": false,
                    "type": format!("{domain}_bridge_wait_error"),
                    "exit_code": 1,
                    "reason": format!("{domain}_bridge_wait_failed:{err}"),
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": "",
                    "stderr": "",
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
        },
        Err(err) => {
            let mut detail = serde_json::json!({
                "ok": false,
                "type": format!("{domain}_bridge_spawn_error"),
                "exit_code": 1,
                "reason": format!("{domain}_bridge_spawn_failed:{err}"),
                "command": command,
                "args": command_args,
                "run_context": run_context,
                "stdout": "",
                "stderr": "",
                "routed_via": "conduit",
                "domain": domain,
                "bridge_timeout_ms": timeout_ms
            });
            detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
            detail
        }
    }
}

fn execute_spine_bridge_command(args: &[String], run_context: Option<&str>) -> Value {
    let mut detail = execute_ops_bridge_command("spine", args, run_context);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["spine_receipt"] = receipt;
    }
    detail
}

fn execute_attention_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("attention-queue", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["attention_receipt"] = receipt;
    }
    detail
}

fn execute_persona_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("persona-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["persona_ambient_receipt"] = receipt;
    }
    detail
}

fn execute_dopamine_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("dopamine-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["dopamine_ambient_receipt"] = receipt;
    }
    detail
}

fn execute_memory_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("memory-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["memory_ambient_receipt"] = receipt;
    }
    detail
}

#[cfg(feature = "edge")]
fn edge_backend_label() -> &'static str {
    "picolm_static_stub"
}

#[cfg(not(feature = "edge"))]
fn edge_backend_label() -> &'static str {
    "edge_feature_disabled"
}

#[cfg(any(feature = "edge", test))]
fn normalize_edge_prompt(prompt: &str) -> String {
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "(empty_prompt)".to_string()
    } else {
        normalized
    }
}

#[cfg(any(feature = "edge", test))]
fn summarize_for_edge_backend(prompt: &str, token_cap: usize) -> String {
    let tokens = prompt.split_whitespace().collect::<Vec<_>>();
    if tokens.len() <= token_cap {
        return tokens.join(" ");
    }
    tokens
        .into_iter()
        .take(token_cap)
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_lane_id(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn deterministic_receipt_hash<T: Serialize>(value: &T) -> String {
    let canonical = canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

const VERITY_DRIFT_CONFIG_SCHEMA_ID: &str = "infring_verity_drift_policy";
const VERITY_DRIFT_CONFIG_SCHEMA_VERSION: u32 = 1;
const VERITY_DRIFT_CONFIG_POLICY_VERSION: u32 = 1;
const VERITY_DRIFT_MODE_PRODUCTION: &str = "production";
const VERITY_DRIFT_MODE_SIMULATION: &str = "simulation";
const VERITY_DRIFT_PRODUCTION_DEFAULT_MS: i64 = 500;
