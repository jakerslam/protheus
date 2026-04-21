
fn into_c_string_ptr(payload: String) -> *mut c_char {
    let sanitized = payload.replace('\0', "");
    match CString::new(sanitized) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::new("{\"ok\":false,\"error\":\"cstring_encode_failed\"}")
            .unwrap_or_else(|_| {
                CString::new("{}").expect("fallback CString literal should be valid")
            })
            .into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn security_check_ffi(request_json: *const c_char) -> *mut c_char {
    let payload = match c_str_to_string(request_json).and_then(|req| evaluate_operation_json(&req))
    {
        Ok(v) => v,
        Err(err) => serde_json::json!({
            "ok": false,
            "error": err.to_string()
        })
        .to_string(),
    };
    into_c_string_ptr(payload)
}

#[no_mangle]
pub extern "C" fn security_enforce_ffi(
    request_json: *const c_char,
    state_root: *const c_char,
) -> *mut c_char {
    let payload = match c_str_to_string(request_json).and_then(|req| {
        let root = c_str_to_string(state_root).unwrap_or_else(|_| ".".to_string());
        enforce_operation_json(&req, Path::new(&root))
    }) {
        Ok(v) => v,
        Err(err) => serde_json::json!({
            "ok": false,
            "error": err.to_string()
        })
        .to_string(),
    };
    into_c_string_ptr(payload)
}

#[no_mangle]
pub extern "C" fn security_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> SecurityOperationRequest {
        SecurityOperationRequest {
            operation_id: "op_test_1".to_string(),
            subsystem: "memory".to_string(),
            action: "recall".to_string(),
            actor: "unit_test".to_string(),
            risk_class: "normal".to_string(),
            payload_digest: Some("sha256:abcd".to_string()),
            tags: vec!["runtime.guardrails".to_string()],
            covenant_violation: false,
            tamper_signal: false,
            key_age_hours: 1,
            operator_quorum: 2,
            audit_receipt_nonce: Some("nonce-1".to_string()),
            zk_proof: Some("zk-proof".to_string()),
            ciphertext_digest: Some("sha256:cipher".to_string()),
        }
    }

    #[test]
    fn allow_clean_operation() {
        let req = base_request();
        let decision = evaluate_operation(&req).expect("decision should evaluate");
        assert!(decision.ok, "clean operation should pass security gate");
        assert!(
            !decision.fail_closed,
            "clean operation should not fail-close"
        );
    }

    #[test]
    fn fail_closed_on_covenant_violation() {
        let mut req = base_request();
        req.covenant_violation = true;
        let decision = evaluate_operation(&req).expect("decision should evaluate");
        assert!(!decision.ok, "covenant violation must deny");
        assert!(decision.fail_closed, "covenant violation must fail-close");
    }

    #[test]
    fn enforce_writes_shutdown_and_alert() {
        let mut req = base_request();
        req.tamper_signal = true;
        req.operator_quorum = 1;

        let temp_dir =
            std::env::temp_dir().join(format!("security_core_test_{}", std::process::id()));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).expect("temp dir should create");

        let decision = enforce_operation(&req, &temp_dir).expect("enforce should evaluate");
        assert!(decision.fail_closed, "tamper must fail-close");

        let shutdown_path = temp_dir.join("security/hard_shutdown.json");
        let alerts_path = temp_dir.join("security/human_alerts.jsonl");
        assert!(shutdown_path.exists(), "shutdown file should exist");
        assert!(alerts_path.exists(), "alerts file should exist");
    }
}

// Small std-only RFC3339 formatter helper to avoid chrono dependency.
mod chrono_stub {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub struct DateTime {
        secs: u64,
    }

    impl From<SystemTime> for DateTime {
        fn from(value: SystemTime) -> Self {
            let secs = value
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
            Self { secs }
        }
    }

    impl DateTime {
        pub fn to_rfc3339(&self) -> String {
            // Fallback ISO-like timestamp; sufficient for machine parsing in this project.
            format!("{}Z", self.secs)
        }
    }
}
