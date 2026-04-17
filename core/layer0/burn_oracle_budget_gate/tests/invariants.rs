// SPDX-License-Identifier: Apache-2.0
use burn_oracle_budget_gate::{
    evaluate_burn_oracle_budget_gate, BurnOracleBudgetRequest, OracleStatus, CHECK_ID,
    MAX_BURN_UNITS_CAP, RECEIPT_SCHEMA_ID,
};

#[test]
fn budget_bounds_enforced_fail_closed() {
    let exceeds_policy = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 11,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 100,
        },
    });
    assert!(!exceeds_policy.ok);
    assert_eq!(exceeds_policy.code, "budget_bound_exceeded");

    let below_minimum = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 0,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 100,
        },
    });
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
    let decision = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 9,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 8,
        },
    });
    assert!(!decision.ok);
    assert_eq!(decision.code, "oracle_budget_exceeded");
}

#[test]
fn deterministic_receipts_are_stable_for_equal_inputs() {
    let request = BurnOracleBudgetRequest {
        requested_burn_units: 5,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 20,
        },
    };

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
    let decision = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 3,
        max_allowed_burn_units: 3,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 5,
        },
    });
    assert!(decision.ok);
    assert_eq!(decision.code, "ok");
    assert_eq!(decision.receipt.code, "ok");
    assert_eq!(decision.receipt.schema_id, RECEIPT_SCHEMA_ID);
    assert_eq!(decision.receipt.check_id, CHECK_ID);
}

#[test]
fn out_of_bounds_inputs_fail_closed_with_stable_codes() {
    let budget_oob = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: MAX_BURN_UNITS_CAP + 1,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 50,
        },
    });
    assert!(!budget_oob.ok);
    assert_eq!(budget_oob.code, "budget_value_out_of_bounds");

    let oracle_oob = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 5,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: MAX_BURN_UNITS_CAP + 1,
        },
    });
    assert!(!oracle_oob.ok);
    assert_eq!(oracle_oob.code, "oracle_value_out_of_bounds");
}

#[test]
fn deterministic_key_changes_when_inputs_change() {
    let a = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 4,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 20,
        },
    });
    let b = evaluate_burn_oracle_budget_gate(BurnOracleBudgetRequest {
        requested_burn_units: 5,
        max_allowed_burn_units: 10,
        oracle_status: OracleStatus::Available {
            remaining_burn_units: 20,
        },
    });
    assert_ne!(a.receipt.deterministic_key, b.receipt.deterministic_key);
}
