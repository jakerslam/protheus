
pub fn evaluate_operation(
    req: &SecurityOperationRequest,
) -> Result<SecurityDecision, SecurityError> {
    let vault_policy = load_embedded_vault_policy()
        .map_err(|err| SecurityError::VaultPolicyLoadFailed(err.to_string()))?;
    let vault_request = to_vault_request(req);
    let vault_decision = evaluate_vault_policy(&vault_policy, &vault_request);

    let observability_profile = load_embedded_observability_profile()
        .map_err(|err| SecurityError::ObservabilityProfileLoadFailed(err.to_string()))?;
    let (score, threshold) =
        compute_sovereignty_score(&observability_profile, req, &vault_decision);

    let mut reasons: Vec<String> = Vec::new();
    if req.covenant_violation {
        reasons.push("covenant_violation_detected".to_string());
    }
    if req.tamper_signal {
        reasons.push("tamper_signal_detected".to_string());
    }
    if score < f64::from(threshold) {
        reasons.push(format!(
            "sovereignty_score_below_threshold:{score:.2}<{}",
            threshold
        ));
    }
    reasons.extend(vault_decision.reasons.iter().cloned());

    let fail_closed = req.covenant_violation
        || req.tamper_signal
        || (score < f64::from(threshold))
        || (!vault_decision.allowed && vault_decision.fail_closed);

    if reasons.is_empty() {
        reasons.push("security_gate_allow".to_string());
    }

    let ok = !fail_closed && vault_decision.allowed;
    let digest = digest_for_decision(req, &reasons, score);

    Ok(SecurityDecision {
        ok,
        fail_closed,
        shutdown_required: fail_closed,
        human_alert_required: fail_closed,
        sovereignty_score_pct: score,
        sovereignty_threshold_pct: threshold,
        decision_digest: digest,
        reasons,
        vault_decision,
    })
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<(), SecurityError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| SecurityError::IoFailed(format!("mkdir_failed:{err}")))?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| SecurityError::EncodeFailed(err.to_string()))?;
    std::fs::write(&tmp, payload)
        .map_err(|err| SecurityError::IoFailed(format!("write_tmp_failed:{err}")))?;
    std::fs::rename(&tmp, path)
        .map_err(|err| SecurityError::IoFailed(format!("rename_failed:{err}")))?;
    Ok(())
}

fn append_jsonl(path: &Path, value: &serde_json::Value) -> Result<(), SecurityError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| SecurityError::IoFailed(format!("mkdir_failed:{err}")))?;
    }
    let mut line =
        serde_json::to_string(value).map_err(|err| SecurityError::EncodeFailed(err.to_string()))?;
    line.push('\n');
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| SecurityError::IoFailed(format!("open_append_failed:{err}")))?;
    file.write_all(line.as_bytes())
        .map_err(|err| SecurityError::IoFailed(format!("append_failed:{err}")))?;
    Ok(())
}

fn alert_for(req: &SecurityOperationRequest, decision: &SecurityDecision) -> SecurityAlert {
    let reason = decision
        .reasons
        .first()
        .cloned()
        .unwrap_or_else(|| "fail_closed_triggered".to_string());
    SecurityAlert {
        ts: now_iso(),
        operation_id: req.operation_id.clone(),
        subsystem: req.subsystem.clone(),
        action: req.action.clone(),
        actor: req.actor.clone(),
        severity: "critical".to_string(),
        reason,
    }
}

pub fn enforce_operation(
    req: &SecurityOperationRequest,
    state_root: &Path,
) -> Result<SecurityDecision, SecurityError> {
    let decision = evaluate_operation(req)?;

    if decision.fail_closed {
        let security_dir = state_root.join("security");
        let shutdown_path = security_dir.join("hard_shutdown.json");
        let alerts_path = security_dir.join("human_alerts.jsonl");

        write_json_atomic(
            &shutdown_path,
            &serde_json::json!({
                "ok": false,
                "fail_closed": true,
                "ts": now_iso(),
                "operation_id": req.operation_id,
                "subsystem": req.subsystem,
                "action": req.action,
                "actor": req.actor,
                "reason": decision.reasons.first().cloned().unwrap_or_else(|| "fail_closed".to_string()),
                "decision_digest": decision.decision_digest,
                "status": "shutdown"
            }),
        )?;

        let alert = alert_for(req, &decision);
        append_jsonl(
            &alerts_path,
            &serde_json::to_value(alert).unwrap_or_default(),
        )?;
    }

    Ok(decision)
}

pub fn evaluate_operation_json(request_json: &str) -> Result<String, SecurityError> {
    let req: SecurityOperationRequest = serde_json::from_str(request_json)
        .map_err(|err| SecurityError::RequestDecodeFailed(err.to_string()))?;
    let decision = evaluate_operation(&req)?;
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "decision": decision
    }))
    .map_err(|err| SecurityError::EncodeFailed(err.to_string()))
}

pub fn enforce_operation_json(
    request_json: &str,
    state_root: &Path,
) -> Result<String, SecurityError> {
    let req: SecurityOperationRequest = serde_json::from_str(request_json)
        .map_err(|err| SecurityError::RequestDecodeFailed(err.to_string()))?;
    let decision = enforce_operation(&req, state_root)?;
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "decision": decision
    }))
    .map_err(|err| SecurityError::EncodeFailed(err.to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultSealRequest {
    pub operation_id: String,
    pub key_id: String,
    pub data_base64: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub covenant_violation: bool,
    #[serde(default)]
    pub tamper_signal: bool,
    #[serde(default = "default_operator_quorum")]
    pub operator_quorum: u8,
    #[serde(default = "default_key_age_hours")]
    pub key_age_hours: u32,
    #[serde(default)]
    pub audit_receipt_nonce: Option<String>,
    #[serde(default)]
    pub zk_proof: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultRotateRequest {
    pub operation_id: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub key_ids: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub covenant_violation: bool,
    #[serde(default)]
    pub tamper_signal: bool,
    #[serde(default = "default_operator_quorum")]
    pub operator_quorum: u8,
    #[serde(default = "default_key_age_hours")]
    pub key_age_hours: u32,
    #[serde(default)]
    pub audit_receipt_nonce: Option<String>,
    #[serde(default)]
    pub zk_proof: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultAuditRequest {
    pub operation_id: String,
    #[serde(default)]
    pub actor: String,
}

fn digest_parts(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"|");
    }
    hex::encode(hasher.finalize())
}
