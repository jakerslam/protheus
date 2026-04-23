fn bitnet_attest_receipt(root: &Path, args: &[String], strict: bool) -> Value {
    let model_digest = flag_value(args, "model-digest").unwrap_or_default();
    let provenance = flag_value(args, "provenance")
        .or_else(|| flag_value(args, "source-model"))
        .unwrap_or_default();
    let allowed_source = provenance.starts_with("hf://")
        || provenance.starts_with("local://")
        || provenance.starts_with("./")
        || provenance.starts_with('/');
    if strict && (model_digest.trim().is_empty() || !allowed_source) {
        let mut out = json!({
            "ok": false,
            "type": "model_router_bitnet_attest",
            "ts": now_iso(),
            "strict": strict,
            "errors": ["bitnet_provenance_attestation_failed"],
            "attestation": {
                "model_digest_present": !model_digest.trim().is_empty(),
                "allowed_source": allowed_source
            },
            "claim_evidence": [
                {
                    "id": "V6-MODEL-004.5",
                    "claim": "bitnet_inference_and_conversion_require_conduit_gating_and_provenance_attestation",
                    "evidence": {
                        "model_digest_present": !model_digest.trim().is_empty(),
                        "allowed_source": allowed_source
                    }
                }
            ]
        });
        finalize_model_router_receipt(&mut out);
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        return out;
    }
    let attestation = json!({
        "version": "v1",
        "model_digest": model_digest,
        "provenance": provenance,
        "attested_at": now_iso(),
        "attestation_digest": receipt_hash(&json!({
            "model_digest": model_digest,
            "provenance": provenance
        }))
    });
    write_json(&bitnet_attestation_path(root), &attestation);
    let mut out = json!({
        "ok": true,
        "type": "model_router_bitnet_attest",
        "ts": now_iso(),
        "strict": strict,
        "attestation": attestation,
        "attestation_state_path": bitnet_attestation_path(root).display().to_string(),
        "claim_evidence": [
            {
                "id": "V6-MODEL-004.5",
                "claim": "bitnet_inference_and_conversion_require_conduit_gating_and_provenance_attestation",
                "evidence": {
                    "model_digest": model_digest,
                    "provenance": provenance
                }
            }
        ]
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    let (latest_path, history_path) = model_router_state_paths(root);
    write_json(&latest_path, &out);
    append_jsonl(&history_path, &out);
    out
}

pub fn run(root: &Path, args: &[String]) -> i32 {
    let cmd = args
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  infring-ops model-router status");
        println!("  infring-ops model-router infer --intent=<text> --task=<text> [--risk=low|medium|high] [--complexity=low|medium|high]");
        println!("  infring-ops model-router optimize [minimax] [--compact-lines=24] [--target-cost=0.30] [--baseline-cost=5.0] [--quality-target-pct=95]");
        println!("  infring-ops model-router compact-context [--max-lines=24] [--source=soul,memory,task]");
        println!("  infring-ops model-router decompose-task [--task=<text>]");
        println!(
            "  infring-ops model-router adapt-repo [--repo=<url|path>] [--strategy=reuse-first]"
        );
        println!("  infring-ops model-router reset-agent [--preserve-identity=1|0] [--scope=routing+session-cache]");
        println!("  infring-ops model-router night-schedule [--start-hour=0] [--end-hour=6] [--timezone=America/Denver] [--cheap-model=minimax/m2.5]");
        println!("  infring-ops model-router bitnet-backend [--kernel=bitnet.cpp] [--model-format=bitnet-q3] [--strict=1|0]");
        println!("  infring-ops model-router bitnet-auto-route [--battery-pct=20] [--offline=1|0] [--edge=1|0]");
        println!("  infring-ops model-router bitnet-use [--source-model=hf://...] [--target-model=bitnet/local]");
        println!("  infring-ops model-router bitnet-telemetry [--throughput=<n>] [--energy-j=<n>] [--baseline-energy-j=<n>] [--memory-mb=<n>] [--hardware-class=<id>]");
        println!("  infring-ops model-router bitnet-attest [--model-digest=<hex>] [--provenance=<uri>] [--strict=1|0]");
        return 0;
    }

    let strict = parse_bool_flag(flag_value(args, "strict"), false);
    if !matches!(cmd.as_str(), "status" | "infer" | "run") {
        let conduit = model_router_conduit_enforcement(args, &cmd, strict);
        if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let mut out = json!({
                "ok": false,
                "type": "model_router_conduit_gate",
                "ts": now_iso(),
                "command": cmd,
                "strict": strict,
                "error": "conduit_bypass_rejected",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            });
            finalize_model_router_receipt(&mut out);
            out["receipt_hash"] = Value::String(receipt_hash(&out));
            print_json_line(&out);
            return 1;
        }
    }

    if matches!(
        cmd.as_str(),
        "optimize" | "optimize-cheap" | "optimize-minimax"
    ) {
        let out = optimize_cheapest_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "reset-agent" | "agent-reset") {
        let out = reset_agent_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "night-schedule" | "schedule-night") {
        let out = night_scheduler_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "compact-context" | "compact") {
        let out = compact_context_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "decompose-task" | "decompose") {
        let out = decompose_task_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "adapt-repo" | "repo-adapt") {
        let out = adapt_repo_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "bitnet-backend" | "backend-bitnet") {
        let out = bitnet_backend_receipt(root, args, strict);
        print_json_line(&out);
        return if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            0
        } else {
            1
        };
    }

    if matches!(cmd.as_str(), "bitnet-auto-route" | "auto-route-bitnet") {
        let out = bitnet_auto_route_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "bitnet-use" | "use-bitnet" | "convert-bitnet") {
        let out = bitnet_use_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "bitnet-telemetry" | "telemetry-bitnet") {
        let out = bitnet_telemetry_receipt(root, args);
        print_json_line(&out);
        return 0;
    }

    if matches!(cmd.as_str(), "bitnet-attest" | "attest-bitnet") {
        let out = bitnet_attest_receipt(root, args, strict);
        print_json_line(&out);
        return if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            0
        } else {
            1
        };
    }

    if !matches!(cmd.as_str(), "status" | "infer" | "run") {
        let mut out = json!({
            "ok": false,
            "type": "model_router_cli_error",
            "ts": now_iso(),
            "command": cmd,
            "argv": args,
            "error": "unknown_command",
            "exit_code": 2
        });
        finalize_model_router_receipt(&mut out);
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        print_json_line(&out);
        return 2;
    }

    let intent = flag_value(args, "intent").unwrap_or_default();
    let task = flag_value(args, "task").unwrap_or_else(|| {
        args.iter()
            .skip(1)
            .filter(|v| !v.starts_with("--"))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    });
    let risk = flag_value(args, "risk").unwrap_or_else(|| "low".to_string());
    let complexity = flag_value(args, "complexity").unwrap_or_else(|| "low".to_string());
    let role = infer_role(&intent, &task);
    let capability = infer_capability(&intent, &task, &role);
    let tier = infer_tier(&risk, &complexity);
    let provider_online = parse_bool_flag(flag_value(args, "provider-online"), true);
    let preferred_model = flag_value(args, "preferred-model")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "ollama/llama3.2:latest".to_string());
    let fallback_model = flag_value(args, "fallback-model")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "ollama/kimi-k2.5:cloud".to_string());
    let (selected_model, fallback_applied) =
        select_route_model(provider_online, &preferred_model, &fallback_model);

    let mut out = json!({
        "ok": true,
        "type": "model_router",
        "ts": now_iso(),
        "command": cmd,
        "argv": args,
        "root": root.to_string_lossy(),
        "intent": intent,
        "task": task,
        "risk": risk,
        "complexity": complexity,
        "role": role,
        "capability": capability,
        "tier": tier,
        "route_plan": {
            "provider_online": provider_online,
            "preferred_model": preferred_model,
            "fallback_model": fallback_model,
            "selected_model": selected_model,
            "fallback_applied": fallback_applied
        },
        "claim_evidence": [
            {
                "id": "native_model_router_lane",
                "claim": "model_router inference runs natively in rust",
                "evidence": {
                    "role": role,
                    "capability": capability,
                    "tier": tier
                }
            },
            {
                "id": "router_offline_fallback_contract",
                "claim": "router emits deterministic fallback model selection when provider degrades",
                "evidence": {
                    "provider_online": provider_online,
                    "fallback_applied": fallback_applied
                }
            }
        ],
        "persona_lenses": {
            "router": {
                "mode": cmd
            }
        }
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    print_json_line(&out);
    0
}

fn normalize_key(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

pub const ROUTER_MIN_REQUEST_TOKENS: i64 = 120;
pub const ROUTER_MAX_REQUEST_TOKENS: i64 = 12_000;
pub const ROUTER_PROBE_SUPPRESSION_TIMEOUT_STREAK_DEFAULT: i64 = 3;
pub const ROUTER_PROBE_SUPPRESSION_MINUTES_DEFAULT: i64 = 45;
pub const ROUTER_PROBE_REHAB_SUCCESS_THRESHOLD_DEFAULT: i64 = 2;
pub const ROUTER_BUDGET_DIR_DEFAULT: &str = "local/state/autonomy/daily_budget";
pub const ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT: &str =
    "local/state/ops/dynamic_burn_budget_oracle/latest.json";
pub const DEFAULT_FAST_PATH_DISALLOW_REGEXES: [&str; 5] = [
    "https?:\\/\\/",
    "(^|\\s)--?[a-z0-9][a-z0-9_-]*\\b",
    "\\b(node|npm|pnpm|yarn|git|curl|python|bash|zsh|ollama)\\b",
    "[`{}\\[\\]<>$;=]",
    "(^|\\s)(~\\/|\\.\\.?\\/|\\/users\\/|[a-z]:\\\\)",
];

pub fn is_local_ollama_model(model_id: &str) -> bool {
    let model = model_id.trim();
    !model.is_empty() && model.starts_with("ollama/") && !model.contains(":cloud")
}

pub fn is_cloud_model(model_id: &str) -> bool {
    let model = model_id.trim();
    !model.is_empty() && (model.contains(":cloud") || !model.starts_with("ollama/"))
}

pub fn ollama_model_name(model_id: &str) -> String {
    model_id.trim_start_matches("ollama/").to_string()
}

pub fn infer_tier(risk: &str, complexity: &str) -> u8 {
    let risk_norm = normalize_key(risk);
    let complexity_norm = normalize_key(complexity);
    if risk_norm == "high" || complexity_norm == "high" {
        return 3;
    }
    if risk_norm == "medium" || complexity_norm == "medium" {
        return 2;
    }
    1
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

fn has_any_exact(tokens: &HashSet<String>, words: &[&str]) -> bool {
    words
        .iter()
        .any(|w| tokens.contains(&w.to_ascii_lowercase()))
}

fn has_prefix(tokens: &HashSet<String>, prefix: &str) -> bool {
    let p = prefix.to_ascii_lowercase();
    tokens.iter().any(|t| t.starts_with(&p))
}

pub fn infer_role(intent: &str, task: &str) -> String {
    let combined = format!("{} {}", intent, task);
    let tokens = tokenize(&combined);

    if has_any_exact(
        &tokens,
        &[
            "code",
            "refactor",
            "patch",
            "bug",
            "test",
            "typescript",
            "javascript",
            "python",
            "node",
            "compile",
        ],
    ) {
        return "coding".to_string();
    }

    if has_any_exact(
        &tokens,
        &[
            "tool",
            "api",
            "curl",
            "exec",
            "command",
            "shell",
            "cli",
            "automation",
        ],
    ) || has_prefix(&tokens, "integrat")
    {
        return "tools".to_string();
    }

    let has_parallel_agent = tokens.contains("parallel") && tokens.contains("agent");
    if has_any_exact(&tokens, &["swarm", "multi-agent", "handoff", "delegate"])
        || has_parallel_agent
    {
        return "swarm".to_string();
    }

    if has_any_exact(&tokens, &["plan", "roadmap", "strategy", "backlog", "roi"])
        || has_prefix(&tokens, "priorit")
    {
        return "planning".to_string();
    }

    if has_any_exact(
        &tokens,
        &["prove", "formal", "derive", "reason", "logic", "constraint"],
    ) {
        return "logic".to_string();
    }

    if has_any_exact(
        &tokens,
        &[
            "chat", "reply", "post", "comment", "write", "summar", "explain",
        ],
    ) {
        return "chat".to_string();
    }

    "general".to_string()
}

