use serde_json::json;

const TARGET_READY_LANES: usize = 9;
const MAX_COMPLETED_LANES: usize = 64;

fn normalize_completed_lanes(completed_lanes: usize) -> (usize, bool) {
    let normalized = completed_lanes.min(MAX_COMPLETED_LANES);
    (normalized, normalized != completed_lanes)
}

fn evaluate_envelope_contract(
    within_target: bool,
    remaining_lanes: usize,
    clamped_completed_lanes: bool,
) -> (bool, bool, bool, &'static str) {
    let should_warn = !within_target || remaining_lanes > 0;
    let should_block = clamped_completed_lanes;
    let strict_ok = !should_block;
    let reason = if clamped_completed_lanes {
        "completed_lanes_clamped"
    } else if !within_target {
        "outside_target_band"
    } else if remaining_lanes > 0 {
        "target_lanes_incomplete"
    } else {
        "envelope_contract_ok"
    };
    (strict_ok, should_warn, should_block, reason)
}

pub fn build_envelope(within_target: bool, completed_lanes: usize) -> serde_json::Value {
    let (safe_completed_lanes, clamped_completed_lanes) = normalize_completed_lanes(completed_lanes);
    let status = if safe_completed_lanes >= TARGET_READY_LANES && within_target {
        "ready_for_guardrail_gate"
    } else if safe_completed_lanes >= TARGET_READY_LANES {
        "ready_but_out_of_target"
    } else {
        "incomplete"
    };
    let action = if safe_completed_lanes >= TARGET_READY_LANES && within_target {
        "freeze_share_and_optimize_hotpaths"
    } else if safe_completed_lanes >= TARGET_READY_LANES {
        "run_targeted_rust_decomposition"
    } else {
        "continue_incremental_rust_cutovers"
    };
    let remaining_lanes = TARGET_READY_LANES.saturating_sub(safe_completed_lanes);
    let (strict_ok, should_warn, should_block, contract_reason) =
        evaluate_envelope_contract(within_target, remaining_lanes, clamped_completed_lanes);

    json!({
        "ok": strict_ok,
        "lane": "V5-RUST-HYB-010",
        "completed_lanes": safe_completed_lanes,
        "completed_lanes_clamped": clamped_completed_lanes,
        "target_ready_lanes": TARGET_READY_LANES,
        "remaining_lanes": remaining_lanes,
        "within_target": within_target,
        "status": status,
        "action": action,
        "contract": {
            "strict_ok": strict_ok,
            "reason": contract_reason,
            "should_warn": should_warn,
            "should_block": should_block
        },
        "enforcement_mode": if within_target { "strict" } else { "degraded" },
        "guardrails": [
            "keep_ts_for_operator_surfaces",
            "restrict_rust_to_hotpaths_and_safety_critical_lanes",
            "require_canary_and_rollback_receipts"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_includes_guardrails() {
        let v = build_envelope(false, 9);
        assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(true));
        assert!(v
            .get("guardrails")
            .and_then(|x| x.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn completed_lane_count_is_bounded() {
        let v = build_envelope(true, usize::MAX);
        assert_eq!(
            v.get("completed_lanes").and_then(|x| x.as_u64()),
            Some(MAX_COMPLETED_LANES as u64)
        );
        assert_eq!(
            v.get("completed_lanes_clamped").and_then(|x| x.as_bool()),
            Some(true)
        );
        assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(false));
    }
}
