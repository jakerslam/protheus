fn run_sleep_cleanup_command(root: &Path, argv: &[String]) -> i32 {
    let sub = argv
        .first()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let rest = if argv.is_empty() { &[][..] } else { &argv[1..] };

    match sub.as_str() {
        "status" => {
            let policy = load_sleep_cleanup_policy(root);
            let latest = read_json(&policy.state_path).unwrap_or_else(|| json!({}));
            let out = json!({
                "ok": true,
                "type": "spine_sleep_cleanup_status",
                "ts": now_iso(),
                "latest": latest
            });
            print_json_line(&out);
            0
        }
        "plan" => {
            let force = bool_from_flag(rest, "force", true);
            let (code, out) = execute_sleep_cleanup(root, false, force, "manual_plan");
            print_json_line(&out);
            code
        }
        "run" => {
            let apply = bool_from_flag(rest, "apply", true);
            let force = bool_from_flag(rest, "force", false);
            let (code, out) = execute_sleep_cleanup(root, apply, force, "manual_run");
            print_json_line(&out);
            code
        }
        "purge" => {
            let apply = bool_from_flag(rest, "apply", true);
            let force = bool_from_flag(rest, "force", true);
            let (code, out) = execute_sleep_cleanup_purge(root, apply, force, "manual_purge");
            print_json_line(&out);
            code
        }
        _ => {
            let out = cli_error_receipt(argv, "sleep_cleanup_invalid_args", 2);
            print_json_line(&out);
            2
        }
    }
}

fn load_mech_suit_policy(root: &Path) -> MechSuitPolicy {
    let default_path = {
        let candidate = root
            .join("client")
            .join("config")
            .join("mech_suit_mode_policy.json");
        if candidate.exists() {
            candidate
        } else {
            root.join("config").join("mech_suit_mode_policy.json")
        }
    };
    let policy_path = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or(default_path);
    let raw = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let enabled = bool_from_env("MECH_SUIT_MODE_FORCE")
        .unwrap_or_else(|| raw.get("enabled").and_then(Value::as_bool).unwrap_or(true));
    let state = raw.get("state");
    let spine = raw.get("spine");
    let eyes = raw.get("eyes");
    let attention_contract = eyes
        .and_then(|v| v.get("attention_contract"))
        .and_then(Value::as_object);
    let personas = raw.get("personas");
    let dopamine = raw.get("dopamine");

    MechSuitPolicy {
        enabled,
        heartbeat_hours: spine
            .and_then(|v| v.get("heartbeat_hours"))
            .and_then(Value::as_i64)
            .unwrap_or(4)
            .max(1),
        manual_triggers_allowed: spine
            .and_then(|v| v.get("manual_triggers_allowed"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        quiet_non_critical: spine
            .and_then(|v| v.get("quiet_non_critical"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        silent_subprocess_output: spine
            .and_then(|v| v.get("silent_subprocess_output"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        push_attention_queue: eyes
            .and_then(|v| v.get("push_attention_queue"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        attention_queue_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("attention_queue_path")),
            "client/runtime/local/state/attention/queue.jsonl",
        )
        .to_string_lossy()
        .to_string(),
        attention_receipts_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("receipts_path")),
            "client/runtime/local/state/attention/receipts.jsonl",
        )
        .to_string_lossy()
        .to_string(),
        attention_latest_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("latest_path")),
            "client/runtime/local/state/attention/latest.json",
        )
        .to_string_lossy()
        .to_string(),
        attention_max_queue_depth: attention_contract
            .and_then(|v| v.get("max_queue_depth"))
            .and_then(Value::as_i64)
            .unwrap_or(2048)
            .clamp(64, 200_000),
        attention_ttl_hours: attention_contract
            .and_then(|v| v.get("ttl_hours"))
            .and_then(Value::as_i64)
            .unwrap_or(48)
            .clamp(1, 24 * 90),
        attention_dedupe_window_hours: attention_contract
            .and_then(|v| v.get("dedupe_window_hours"))
            .and_then(Value::as_i64)
            .unwrap_or(24)
            .clamp(1, 24 * 90),
        attention_backpressure_drop_below: attention_contract
            .and_then(|v| v.get("backpressure_drop_below"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "critical".to_string()),
        attention_escalate_levels: attention_contract
            .and_then(|v| v.get("escalate_levels"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|row| row.trim().to_ascii_lowercase())
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|rows| !rows.is_empty())
            .unwrap_or_else(|| vec!["critical".to_string()]),
        ambient_stance: personas
            .and_then(|v| v.get("ambient_stance"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        dopamine_threshold_breach_only: dopamine
            .and_then(|v| v.get("threshold_breach_only"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        status_path: normalize_path(
            root,
            state.and_then(|v| v.get("status_path")),
            "client/runtime/local/state/ops/mech_suit_mode/latest.json",
        ),
        history_path: normalize_path(
            root,
            state.and_then(|v| v.get("history_path")),
            "client/runtime/local/state/ops/mech_suit_mode/history.jsonl",
        ),
        policy_path,
    }
}

fn update_mech_suit_status(root: &Path, policy: &MechSuitPolicy, component: &str, patch: Value) {
    let mut latest = read_json(&policy.status_path).unwrap_or_else(|| {
        json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        })
    });
    if !latest.is_object() {
        latest = json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        });
    }
    let rel_policy_path = policy
        .policy_path
        .strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| policy.policy_path.to_string_lossy().to_string());
    latest["ts"] = Value::String(now_iso());
    latest["active"] = Value::Bool(policy.enabled);
    latest["policy_path"] = Value::String(rel_policy_path);
    if !latest
        .get("components")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        latest["components"] = json!({});
    }
    latest["components"][component] = patch.clone();
    write_json_atomic(&policy.status_path, &latest);

    if let Some(parent) = policy.history_path.parent() {
        ensure_dir(parent);
    }
    let row = json!({
        "ts": now_iso(),
        "type": "mech_suit_status",
        "component": component,
        "active": policy.enabled,
        "patch": patch
    });
    if let Ok(payload) = serde_json::to_string(&row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&policy.history_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{payload}\n").as_bytes()));
    }
}

fn build_spine_status_receipt(_root: &Path, cli: &CliArgs, policy: &MechSuitPolicy) -> Value {
    let run_context = std::env::var("SPINE_RUN_CONTEXT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "manual".to_string());
    let attention_latest =
        read_json(Path::new(&policy.attention_latest_path)).unwrap_or_else(|| json!({}));
    let mut out = json!({
        "ok": true,
        "type": "spine_status",
        "ts": now_iso(),
        "command": cli.command,
        "mode": cli.mode,
        "date": cli.date,
        "ambient_mode_active": policy.enabled,
        "heartbeat_hours": policy.heartbeat_hours,
        "manual_triggers_allowed": policy.manual_triggers_allowed,
        "quiet_non_critical": policy.quiet_non_critical,
        "silent_subprocess_output": policy.silent_subprocess_output,
        "run_context": run_context,
        "attention_contract": {
            "event_owner": "eyes",
            "escalation_authority": "runtime_policy",
            "push_attention_queue": policy.push_attention_queue,
            "attention_queue_path": policy.attention_queue_path,
            "attention_receipts_path": policy.attention_receipts_path,
            "attention_latest_path": policy.attention_latest_path,
            "max_queue_depth": policy.attention_max_queue_depth,
            "ttl_hours": policy.attention_ttl_hours,
            "dedupe_window_hours": policy.attention_dedupe_window_hours,
            "backpressure_drop_below": policy.attention_backpressure_drop_below.clone(),
            "escalate_levels": policy.attention_escalate_levels.clone(),
            "latest": attention_latest
        },
        "personas": {
            "ambient_stance": policy.ambient_stance
        },
        "dopamine": {
            "threshold_breach_only": policy.dopamine_threshold_breach_only
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn emit_status(root: &Path, cli: &CliArgs, policy: &MechSuitPolicy) -> i32 {
    let receipt = build_spine_status_receipt(root, cli, policy);
    update_mech_suit_status(
        root,
        policy,
        "spine",
        json!({
            "ambient": policy.enabled,
            "heartbeat_hours": policy.heartbeat_hours,
            "manual_triggers_allowed": policy.manual_triggers_allowed,
            "quiet_non_critical": policy.quiet_non_critical,
            "silent_subprocess_output": policy.silent_subprocess_output,
            "attention_emission_owner": "eyes",
            "attention_escalation_authority": "runtime_policy",
            "last_result": "status",
            "last_mode": cli.mode,
            "last_date": cli.date
        }),
    );
    print_json_line(&receipt);
    0
}

fn ambient_gate_blocked_receipt(
    cli: &CliArgs,
    policy: &MechSuitPolicy,
    run_context: &str,
) -> Value {
    let mut out = json!({
        "ok": false,
        "blocked": true,
        "type": "spine_ambient_gate",
        "ts": now_iso(),
        "command": cli.command,
        "mode": cli.mode,
        "date": cli.date,
        "reason": "manual_trigger_blocked_mech_suit_mode",
        "ambient_mode_active": policy.enabled,
        "required_run_context": "heartbeat",
        "received_run_context": run_context,
        "heartbeat_hours": policy.heartbeat_hours,
        "manual_triggers_allowed": policy.manual_triggers_allowed,
        "quiet_non_critical": policy.quiet_non_critical,
        "silent_subprocess_output": policy.silent_subprocess_output,
        "attention_contract": {
            "event_owner": "eyes",
            "escalation_authority": "runtime_policy",
            "push_attention_queue": policy.push_attention_queue,
            "attention_queue_path": policy.attention_queue_path,
            "attention_receipts_path": policy.attention_receipts_path,
            "max_queue_depth": policy.attention_max_queue_depth,
            "ttl_hours": policy.attention_ttl_hours,
            "dedupe_window_hours": policy.attention_dedupe_window_hours,
            "backpressure_drop_below": policy.attention_backpressure_drop_below.clone(),
            "escalate_levels": policy.attention_escalate_levels.clone()
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

impl LedgerWriter {
    fn new(root: &Path, date: &str, run_id: &str) -> Self {
        Self {
            root: root.to_path_buf(),
            date: date.to_string(),
            run_id: run_id.to_string(),
            seq: 0,
            last_type: None,
        }
    }

    fn last_type(&self) -> Option<&str> {
        self.last_type.as_deref()
    }

    fn append(&mut self, mut evt: Value) {
        self.seq = self.seq.saturating_add(1);
        if let Some(map) = evt.as_object_mut() {
            let evt_type = map
                .get("type")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            map.insert("run_id".to_string(), Value::String(self.run_id.clone()));
            map.insert("ledger_seq".to_string(), Value::Number(self.seq.into()));
            if !map.contains_key("ts") {
                map.insert("ts".to_string(), Value::String(now_iso()));
            }
            if !map.contains_key("date") {
                map.insert("date".to_string(), Value::String(self.date.clone()));
            }
            if let Some(t) = evt_type {
                self.last_type = Some(t);
            }
        }

        let dir = spine_runs_dir(&self.root);
        ensure_dir(&dir);
        let file = dir.join(format!("{}.jsonl", self.date));
        {
            // Hot path lock: keep ledger append and latest marker write serialized with low-overhead mutex.
            let _guard = receipt_ledger_io_lock().lock();
            if let Ok(payload) = serde_json::to_string(&evt) {
                let _ = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .and_then(|mut f| {
                        std::io::Write::write_all(&mut f, format!("{payload}\n").as_bytes())
                    });
            }
            write_json_atomic(&dir.join("latest.json"), &evt);
        }
    }
}

fn constitution_hash(root: &Path) -> (bool, Option<String>, Option<String>) {
    let path = root.join("docs/workspace/AGENT-CONSTITUTION.md");
    match fs::read_to_string(&path) {
        Ok(raw) => {
            let digest = stable_hash(&raw, 64);
            let expected = std::env::var("PROTHEUS_CONSTITUTION_HASH").ok();
            if let Some(exp) = expected {
                (digest == exp, Some(digest), Some(exp))
            } else {
                (true, Some(digest), None)
            }
        }
        Err(_) => (false, None, None),
    }
}

fn compute_evidence_run_plan(
    configured_runs_raw: Option<i64>,
    budget: Option<&str>,
    projected: Option<&str>,
) -> Value {
    let configured_runs = configured_runs_raw.unwrap_or(2).clamp(0, 6);
    let normalize = |v: Option<&str>| -> String {
        match v.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
            "soft" => "soft".to_string(),
            "hard" => "hard".to_string(),
            _ => "none".to_string(),
        }
    };
    let budget_pressure = normalize(budget);
    let projected_pressure = normalize(projected);
    let pressure_throttle = budget_pressure != "none" || projected_pressure != "none";
    let evidence_runs = if pressure_throttle {
        configured_runs.min(1)
    } else {
        configured_runs
    };
    json!({
        "configured_runs": configured_runs,
        "budget_pressure": budget_pressure,
        "projected_pressure": projected_pressure,
        "pressure_throttle": pressure_throttle,
        "evidence_runs": evidence_runs
    })
}

fn default_evidence_plan() -> Value {
    json!({
        "configured_runs": 0,
        "budget_pressure": "none",
        "projected_pressure": "none",
        "pressure_throttle": false,
        "evidence_runs": 0
    })
}

