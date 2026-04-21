
fn to_security_request_for_seal(
    req: &VaultSealRequest,
) -> Result<SecurityOperationRequest, SecurityError> {
    if req.operation_id.trim().is_empty() {
        return Err(SecurityError::ValidationFailed(
            "operation_id_required".to_string(),
        ));
    }
    if req.key_id.trim().is_empty() {
        return Err(SecurityError::ValidationFailed(
            "key_id_required".to_string(),
        ));
    }
    if req.data_base64.trim().is_empty() {
        return Err(SecurityError::ValidationFailed(
            "data_base64_required".to_string(),
        ));
    }
    Ok(SecurityOperationRequest {
        operation_id: req.operation_id.clone(),
        subsystem: "vault".to_string(),
        action: "seal".to_string(),
        actor: if req.actor.trim().is_empty() {
            "operator".to_string()
        } else {
            req.actor.clone()
        },
        risk_class: "critical".to_string(),
        payload_digest: Some(format!(
            "sha256:{}",
            digest_parts(&[&req.key_id, &req.data_base64])
        )),
        tags: vec!["vault".to_string(), "seal".to_string()],
        covenant_violation: req.covenant_violation,
        tamper_signal: req.tamper_signal,
        key_age_hours: req.key_age_hours,
        operator_quorum: req.operator_quorum,
        audit_receipt_nonce: req.audit_receipt_nonce.clone(),
        zk_proof: req.zk_proof.clone(),
        ciphertext_digest: Some(format!(
            "sha256:{}",
            digest_parts(&[&req.key_id, &req.data_base64, "cipher"])
        )),
    })
}

fn to_security_request_for_rotate(
    req: &VaultRotateRequest,
) -> Result<SecurityOperationRequest, SecurityError> {
    if req.operation_id.trim().is_empty() {
        return Err(SecurityError::ValidationFailed(
            "operation_id_required".to_string(),
        ));
    }
    let key_ids = if req.key_ids.is_empty() {
        vec!["all_keys".to_string()]
    } else {
        req.key_ids.clone()
    };
    Ok(SecurityOperationRequest {
        operation_id: req.operation_id.clone(),
        subsystem: "vault".to_string(),
        action: "rotate".to_string(),
        actor: if req.actor.trim().is_empty() {
            "operator".to_string()
        } else {
            req.actor.clone()
        },
        risk_class: "critical".to_string(),
        payload_digest: Some(format!(
            "sha256:{}",
            digest_parts(&[
                &key_ids.join(","),
                req.reason.as_deref().unwrap_or("rotate_all")
            ])
        )),
        tags: vec!["vault".to_string(), "rotate".to_string()],
        covenant_violation: req.covenant_violation,
        tamper_signal: req.tamper_signal,
        key_age_hours: req.key_age_hours,
        operator_quorum: req.operator_quorum,
        audit_receipt_nonce: req.audit_receipt_nonce.clone(),
        zk_proof: req.zk_proof.clone(),
        ciphertext_digest: Some(format!(
            "sha256:{}",
            digest_parts(&[&key_ids.join(","), "rotate"])
        )),
    })
}

pub fn vault_load_policy_json() -> Result<String, SecurityError> {
    load_embedded_vault_policy_json()
        .map_err(|err| SecurityError::VaultPolicyLoadFailed(err.to_string()))
}

pub fn vault_evaluate_json(request_json: &str) -> Result<String, SecurityError> {
    evaluate_vault_policy_json(request_json)
        .map_err(|err| SecurityError::VaultPolicyLoadFailed(err.to_string()))
}

pub fn seal_json(request_json: &str, state_root: &Path) -> Result<String, SecurityError> {
    let req: VaultSealRequest = serde_json::from_str(request_json)
        .map_err(|err| SecurityError::RequestDecodeFailed(err.to_string()))?;
    let security_req = to_security_request_for_seal(&req)?;
    let decision = enforce_operation(&security_req, state_root)?;
    if decision.fail_closed || !decision.ok {
        return serde_json::to_string(&serde_json::json!({
            "ok": false,
            "status": "deny_fail_closed",
            "decision": decision
        }))
        .map_err(|err| SecurityError::EncodeFailed(err.to_string()));
    }

    let sealed_digest = format!(
        "sha256:{}",
        digest_parts(&[
            &req.key_id,
            &req.data_base64,
            &decision.decision_digest,
            req.audit_receipt_nonce.as_deref().unwrap_or("none")
        ])
    );
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "status": "sealed",
        "operation_id": req.operation_id,
        "key_id": req.key_id,
        "sealed_digest": sealed_digest,
        "decision": decision
    }))
    .map_err(|err| SecurityError::EncodeFailed(err.to_string()))
}

pub fn rotate_all_json(request_json: &str, state_root: &Path) -> Result<String, SecurityError> {
    let req: VaultRotateRequest = serde_json::from_str(request_json)
        .map_err(|err| SecurityError::RequestDecodeFailed(err.to_string()))?;
    let key_ids = if req.key_ids.is_empty() {
        vec!["all_keys".to_string()]
    } else {
        req.key_ids.clone()
    };
    let security_req = to_security_request_for_rotate(&req)?;
    let decision = enforce_operation(&security_req, state_root)?;
    if decision.fail_closed || !decision.ok {
        return serde_json::to_string(&serde_json::json!({
            "ok": false,
            "status": "deny_fail_closed",
            "decision": decision
        }))
        .map_err(|err| SecurityError::EncodeFailed(err.to_string()));
    }

    let receipts = key_ids
        .iter()
        .map(|key_id| {
            serde_json::json!({
                "key_id": key_id,
                "rotation_receipt": format!("sha256:{}", digest_parts(&[
                    key_id,
                    req.reason.as_deref().unwrap_or("rotate"),
                    &decision.decision_digest
                ]))
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "status": "rotated",
        "operation_id": req.operation_id,
        "reason": req.reason.unwrap_or_else(|| "rotate_all".to_string()),
        "rotated_keys": key_ids.len(),
        "receipts": receipts,
        "decision": decision
    }))
    .map_err(|err| SecurityError::EncodeFailed(err.to_string()))
}

pub fn audit_json(request_json: &str, _state_root: &Path) -> Result<String, SecurityError> {
    let req: VaultAuditRequest = serde_json::from_str(request_json)
        .map_err(|err| SecurityError::RequestDecodeFailed(err.to_string()))?;
    let policy = load_embedded_vault_policy()
        .map_err(|err| SecurityError::VaultPolicyLoadFailed(err.to_string()))?;
    let policy_json = load_embedded_vault_policy_json()
        .map_err(|err| SecurityError::VaultPolicyLoadFailed(err.to_string()))?;
    let policy_digest =
        digest_parts(&[&policy.policy_id, &policy.version.to_string(), &policy_json]);

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "status": "audited",
        "operation_id": req.operation_id,
        "actor": if req.actor.trim().is_empty() { "operator" } else { req.actor.as_str() },
        "policy_id": policy.policy_id,
        "policy_digest": format!("sha256:{policy_digest}"),
        "rules_count": policy.rules.len(),
        "auto_rotate_enabled": policy.auto_rotate.enabled,
        "fail_closed_rules": policy.rules.iter().filter(|r| r.fail_closed).count(),
        "ts": now_iso()
    }))
    .map_err(|err| SecurityError::EncodeFailed(err.to_string()))
}

fn c_str_to_string(ptr: *const c_char) -> Result<String, SecurityError> {
    if ptr.is_null() {
        return Err(SecurityError::RequestDecodeFailed(
            "null_pointer".to_string(),
        ));
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| SecurityError::RequestDecodeFailed("invalid_utf8".to_string()))?;
    Ok(s.to_string())
}
