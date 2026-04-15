fn security_request(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let digest_seed = serde_json::to_string(&json!({
        "script": script_rel,
        "args": args
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(digest_seed.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let now_ms = chrono::Utc::now().timestamp_millis();

    json!({
        "operation_id": clean(format!("protheusctl_dispatch_{}_{}", now_ms, &digest[..10]), 160),
        "subsystem": "ops",
        "action": "cli_dispatch",
        "actor": "client/runtime/systems/ops/protheusctl",
        "risk_class": if bool_env("PROTHEUS_CTL_SECURITY_HIGH_RISK", false) { "high" } else { "normal" },
        "payload_digest": format!("sha256:{digest}"),
        "tags": ["protheusctl", "dispatch", "foundation_lock"],
        "covenant_violation": bool_env("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION", false),
        "tamper_signal": bool_env("PROTHEUS_CTL_SECURITY_TAMPER_SIGNAL", false),
        "key_age_hours": env::var("PROTHEUS_CTL_SECURITY_KEY_AGE_HOURS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(1),
        "operator_quorum": env::var("PROTHEUS_CTL_SECURITY_OPERATOR_QUORUM").ok().and_then(|v| v.parse::<u8>().ok()).unwrap_or(2),
        "audit_receipt_nonce": clean(format!("nonce-{}-{}", &digest[..12], now_ms), 120),
        "zk_proof": clean(env::var("PROTHEUS_CTL_SECURITY_ZK_PROOF").unwrap_or_else(|_| "zk-protheusctl-dispatch".to_string()), 220),
        "ciphertext_digest": clean(format!("sha256:{}", &digest[..32]), 220),
        "state_root": clean(client_state_root(root).to_string_lossy().to_string(), 500)
    })
}

fn evaluate_persona_dispatch_security(
    script_rel: &str,
    args: &[String],
    req: &Value,
) -> DispatchSecurity {
    let requested_lens = requested_lens_arg(args);
    let valid_lenses = csv_list_env(PERSONA_VALID_LENSES_ENV, PERSONA_VALID_LENSES_DEFAULT);
    let blocked_paths = csv_list_env(PERSONA_BLOCKED_PATHS_ENV, "");
    let valid_lens_refs = valid_lenses.iter().map(String::as_str).collect::<Vec<_>>();
    let blocked_path_refs = blocked_paths.iter().map(String::as_str).collect::<Vec<_>>();
    let covenant_violation = req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tamper_signal = req
        .get("tamper_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let decision = evaluate_persona_dispatch_gate(
        script_rel,
        requested_lens.as_deref(),
        &valid_lens_refs,
        &blocked_path_refs,
        covenant_violation,
        tamper_signal,
    );
    if !decision.ok {
        return DispatchSecurity {
            ok: false,
            reason: format!(
                "security_gate_blocked:{PERSONA_DISPATCH_SECURITY_GATE_CHECK_ID}:{}",
                decision.code
            ),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}

