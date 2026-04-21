include!("predictive_defrag_parts/010-types-and-policy.rs");
include!("predictive_defrag_parts/020-monitor.rs");
include!("predictive_defrag_parts/030-stress.rs");

const PREDICTIVE_DEFRAG_SOFT_TRIGGER_PCT: f64 = 45.0;
const PREDICTIVE_DEFRAG_HARD_TRIGGER_PCT: f64 = 65.0;
const PREDICTIVE_DEFRAG_MAX_BUDGET_PER_HOUR_PCT: f64 = 25.0;

pub fn normalize_predictive_defrag_budget_pct(raw_pct: f64) -> f64 {
    if raw_pct.is_finite() {
        raw_pct.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

pub fn normalize_predictive_defrag_sample_window(raw_window: usize) -> usize {
    raw_window.clamp(1, 10_000)
}

pub fn should_trigger_predictive_defrag(fragmentation_pct: f64, high_pressure: bool) -> bool {
    let pct = normalize_predictive_defrag_budget_pct(fragmentation_pct);
    pct >= PREDICTIVE_DEFRAG_HARD_TRIGGER_PCT
        || (high_pressure && pct >= PREDICTIVE_DEFRAG_SOFT_TRIGGER_PCT)
}

pub fn evaluate_predictive_defrag_contract(
    fragmentation_pct: f64,
    high_pressure: bool,
) -> (f64, bool, bool, &'static str) {
    let pct = normalize_predictive_defrag_budget_pct(fragmentation_pct);
    let should_warn = pct >= PREDICTIVE_DEFRAG_SOFT_TRIGGER_PCT;
    let should_block = pct >= PREDICTIVE_DEFRAG_HARD_TRIGGER_PCT;
    let reason = if should_block {
        "defrag_hard_trigger_reached"
    } else if should_warn && high_pressure {
        "defrag_soft_trigger_with_pressure"
    } else if should_warn {
        "defrag_soft_trigger_warn"
    } else {
        "defrag_contract_ok"
    };
    (pct, should_warn, should_block, reason)
}

pub fn normalize_predictive_defrag_budget_per_hour(raw_pct: f64) -> f64 {
    normalize_predictive_defrag_budget_pct(raw_pct).min(PREDICTIVE_DEFRAG_MAX_BUDGET_PER_HOUR_PCT)
}

pub fn evaluate_predictive_defrag_budget_contract(
    requested_budget_per_hour_pct: f64,
) -> (f64, bool, bool, &'static str) {
    let budget = normalize_predictive_defrag_budget_per_hour(requested_budget_per_hour_pct);
    let should_warn = budget >= 20.0;
    let should_block = budget > PREDICTIVE_DEFRAG_MAX_BUDGET_PER_HOUR_PCT;
    let reason = if should_block {
        "defrag_budget_exceeds_hard_cap"
    } else if should_warn {
        "defrag_budget_high_warn"
    } else {
        "defrag_budget_contract_ok"
    };
    (budget, should_warn, should_block, reason)
}

#[cfg(test)]
mod assim121_predictive_defrag_tests {
    use super::*;

    #[test]
    fn predictive_defrag_thresholds_respect_pressure() {
        assert!(should_trigger_predictive_defrag(70.0, false));
        assert!(should_trigger_predictive_defrag(50.0, true));
        assert!(!should_trigger_predictive_defrag(40.0, true));
    }

    #[test]
    fn predictive_defrag_inputs_are_bounded() {
        assert_eq!(normalize_predictive_defrag_budget_pct(f64::NAN), 0.0);
        assert_eq!(normalize_predictive_defrag_budget_pct(500.0), 100.0);
        assert_eq!(normalize_predictive_defrag_sample_window(0), 1);
        assert_eq!(normalize_predictive_defrag_sample_window(999_999), 10_000);
    }

    #[test]
    fn predictive_defrag_contract_reports_hard_trigger() {
        let (_pct, warn, block, reason) = evaluate_predictive_defrag_contract(90.0, true);
        assert!(warn);
        assert!(block);
        assert_eq!(reason, "defrag_hard_trigger_reached");
    }
}
