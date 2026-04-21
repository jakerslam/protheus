
fn into_c_string_ptr(payload: String) -> *mut c_char {
    let sanitized = payload.replace('\0', "");
    match CString::new(sanitized) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::new("{\"ok\":false,\"error\":\"cstring_encode_failed\"}")
            .unwrap_or_else(|_| CString::new("{}").expect("literal CString should be valid"))
            .into_raw(),
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn run_chaos_resilience_ffi(request_json: *const c_char) -> *mut c_char {
    let payload =
        match c_str_to_string(request_json).and_then(|req| run_chaos_resilience_json(&req)) {
            Ok(v) => v,
            Err(err) => error_json(&err),
        };
    into_c_string_ptr(payload)
}

#[no_mangle]
pub extern "C" fn load_embedded_observability_profile_ffi() -> *mut c_char {
    let payload = match load_embedded_observability_profile_json() {
        Ok(v) => v,
        Err(err) => error_json(&err),
    };
    into_c_string_ptr(payload)
}

#[no_mangle]
pub extern "C" fn observability_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_chaos_resilience_wasm(request_json: &str) -> String {
    match run_chaos_resilience_json(request_json) {
        Ok(v) => v,
        Err(err) => error_json(&err),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn load_embedded_observability_profile_wasm() -> String {
    match load_embedded_observability_profile_json() {
        Ok(v) => v,
        Err(err) => error_json(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(id: &str, ts: u64, severity: &str, tag: &str) -> TraceEvent {
        TraceEvent {
            trace_id: id.to_string(),
            ts_millis: ts,
            source: "client/runtime/systems/observability".to_string(),
            operation: "trace.capture".to_string(),
            severity: severity.to_string(),
            tags: vec![tag.to_string()],
            payload_digest: format!("sha256:{}", id),
            signed: true,
        }
    }

    #[test]
    fn profile_loads() {
        let profile = load_embedded_observability_profile().expect("profile should load");
        assert_eq!(profile.profile_id, "observability_profile_primary");
        assert!(!profile.chaos_hooks.is_empty());
    }

    #[test]
    fn chaos_report_stable_for_low_risk_events() {
        let req = ChaosScenarioRequest {
            scenario_id: "stable_case".to_string(),
            events: vec![
                sample_event("e1", 1000, "low", "runtime.guardrails"),
                sample_event("e2", 1100, "medium", "lane.integrity"),
                sample_event("e3", 1200, "low", "chaos.replay"),
            ],
            cycles: 180000,
            inject_fault_every: 400,
            enforce_fail_closed: true,
        };
        let report = run_chaos_resilience(&req).expect("report should build");
        assert_eq!(report.sovereignty.fail_closed, false);
        assert!(report.resilient);
    }

    #[test]
    fn chaos_report_fail_closed_on_critical_tamper() {
        let mut tamper = sample_event("tamper", 1000, "critical", "tamper");
        tamper.signed = false;
        let req = ChaosScenarioRequest {
            scenario_id: "tamper_case".to_string(),
            events: vec![tamper],
            cycles: 250000,
            inject_fault_every: 2,
            enforce_fail_closed: true,
        };
        let report = run_chaos_resilience(&req).expect("report should build");
        assert!(report.sovereignty.fail_closed);
        assert!(!report.resilient);
    }
}
