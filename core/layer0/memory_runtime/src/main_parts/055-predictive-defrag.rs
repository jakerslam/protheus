include!("predictive_defrag_parts/010-types-and-policy.rs");
include!("predictive_defrag_parts/020-monitor.rs");
include!("predictive_defrag_parts/030-stress.rs");

const PREDICTIVE_DEFRAG_SOFT_TRIGGER_PCT: f64 = 45.0;
const PREDICTIVE_DEFRAG_HARD_TRIGGER_PCT: f64 = 65.0;

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
}
