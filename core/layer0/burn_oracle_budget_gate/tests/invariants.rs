// SPDX-License-Identifier: Apache-2.0
use burn_oracle_budget_gate::{
    evaluate_burn_oracle_budget_gate, BurnOracleBudgetRequest, OracleStatus, CHECK_ID,
    MAX_BURN_UNITS_CAP, RECEIPT_SCHEMA_ID,
};

fn available_request(
    requested_burn_units: u64,
    max_allowed_burn_units: u64,
    remaining_burn_units: u64,
) -> BurnOracleBudgetRequest {
    BurnOracleBudgetRequest {
        requested_burn_units,
        max_allowed_burn_units,
        oracle_status: OracleStatus::Available {
            remaining_burn_units,
        },
    }
}

#[test]
fn budget_bounds_enforced_fail_closed() {
    let exceeds_policy = evaluate_burn_oracle_budget_gate(available_request(11, 10, 100));
    assert!(!exceeds_policy.ok);
    assert_eq!(exceeds_policy.code, "budget_bound_exceeded");

    let below_minimum = evaluate_burn_oracle_budget_gate(available_request(0, 10, 100));
    assert!(!below_minimum.ok);
    assert_eq!(below_minimum.code, "request_below_minimum");
}

#[test]
fn oracle_unavailable_fails_closed() {
    let decision = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 4,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Unavailable,
    });
    assert!(!decision.ok);
    assert_eq!(decision.code, "oracle_unavailable_fail_closed");
}

#[test]
fn oracle_budget_limit_is_enforced() {
    let decision = evaluate_burn_oracle_budget_gate(available_request(9, 10, 8));
    assert!(!decision.ok);
    assert_eq!(decision.code, "oracle_budget_exceeded");
}

#[test]
fn deterministic_receipts_are_stable_for_equal_inputs() {
    let request = available_request(5, 10, 20);

    let first = evaluate_burn_oracle_budget_gate(request);
    let second = evaluate_burn_oracle_budget_gate(request);

    assert_eq!(first.receipt, second.receipt);
    assert_eq!(
        first.receipt.deterministic_key,
        second.receipt.deterministic_key
    );
}

#[test]
fn allowed_path_emits_ok_receipt_contract() {
    let decision = evaluate_burn_oracle_budget_gate(available_request(3, 3, 5));
    assert!(decision.ok);
    assert_eq!(decision.code, "ok");
    assert_eq!(decision.receipt.code, "ok");
    assert_eq!(decision.receipt.schema_id, RECEIPT_SCHEMA_ID);
    assert_eq!(decision.receipt.check_id, CHECK_ID);
}

#[test]
fn out_of_bounds_inputs_fail_closed_with_stable_codes() {
    let budget_oob =
        evaluate_burn_oracle_budget_gate(available_request(MAX_BURN_UNITS_CAP + 1, 10, 50));
    assert!(!budget_oob.ok);
    assert_eq!(budget_oob.code, "budget_value_out_of_bounds");

    let oracle_oob =
        evaluate_burn_oracle_budget_gate(available_request(5, 10, MAX_BURN_UNITS_CAP + 1));
    assert!(!oracle_oob.ok);
    assert_eq!(oracle_oob.code, "oracle_value_out_of_bounds");
}

#[test]
fn deterministic_key_changes_when_inputs_change() {
    let a = evaluate_burn_oracle_budget_gate(available_request(4, 10, 20));
    let b = evaluate_burn_oracle_budget_gate(available_request(5, 10, 20));
    assert_ne!(a.receipt.deterministic_key, b.receipt.deterministic_key);
}

#[test]
fn zero_policy_budget_is_denied_with_policy_invalid_code() {
    let decision = evaluate_burn_oracle_budget_gate(available_request(3, 0, 50));
    assert!(!decision.ok);
    assert_eq!(decision.code, "budget_policy_invalid");
}

#[test]
fn unavailable_oracle_receipt_uses_none_remaining_marker() {
    let decision = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 4,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Unavailable,
    });
    assert!(!decision.ok);
    assert_eq!(decision.code, "oracle_unavailable_fail_closed");
    assert_eq!(decision.receipt.oracle_remaining_burn_units, None);
    assert!(decision.receipt.deterministic_key.ends_with("|none"));
}
