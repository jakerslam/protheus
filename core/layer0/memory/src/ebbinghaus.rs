// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RetentionCurve {
    pub lambda: f64,
    pub age_days: f64,
    pub repetitions: u32,
    pub retention_score: f64,
}

const DEFAULT_LAMBDA: f64 = 0.02;
const MIN_LAMBDA: f64 = 0.0001;
const MAX_LAMBDA: f64 = 5.0;
const MAX_AGE_DAYS: f64 = 3650.0;
const MAX_REPETITIONS: u32 = 4096;

fn sanitize_scalar(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn normalized_lambda(lambda: f64) -> f64 {
    sanitize_scalar(lambda, DEFAULT_LAMBDA)
        .abs()
        .clamp(MIN_LAMBDA, MAX_LAMBDA)
}

fn normalized_age_days(age_days: f64) -> f64 {
    sanitize_scalar(age_days, 0.0).clamp(0.0, MAX_AGE_DAYS)
}

fn normalized_repetitions(repetitions: u32) -> u32 {
    repetitions.clamp(1, MAX_REPETITIONS)
}

fn normalized_threshold(threshold: f64) -> f64 {
    sanitize_scalar(threshold, 0.5).clamp(0.0, 1.0)
}

pub fn retention_score(age_days: f64, repetitions: u32, lambda: f64) -> f64 {
    let safe_age = normalized_age_days(age_days);
    let safe_reps = normalized_repetitions(repetitions) as f64;
    let safe_lambda = normalized_lambda(lambda);
    let repetition_boost = 1.0 + safe_reps.ln();
    let denom = (safe_lambda / repetition_boost).max(0.00001);
    (-denom * safe_age).exp().clamp(0.0, 1.0)
}

pub fn curve(age_days: f64, repetitions: u32, lambda: f64) -> RetentionCurve {
    let safe_lambda = normalized_lambda(lambda);
    let safe_age_days = normalized_age_days(age_days);
    let safe_repetitions = normalized_repetitions(repetitions);
    RetentionCurve {
        lambda: safe_lambda,
        age_days: safe_age_days,
        repetitions: safe_repetitions,
        retention_score: retention_score(safe_age_days, safe_repetitions, safe_lambda),
    }
}

#[allow(dead_code)]
pub fn should_retain(age_days: f64, repetitions: u32, lambda: f64, threshold: f64) -> bool {
    retention_score(age_days, repetitions, lambda) >= normalized_threshold(threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_decays_with_time() {
        let fresh = retention_score(0.1, 1, 0.02);
        let old = retention_score(30.0, 1, 0.02);
        assert!(fresh > old);
    }

    #[test]
    fn repetitions_improve_retention() {
        let low = retention_score(7.0, 1, 0.02);
        let high = retention_score(7.0, 5, 0.02);
        assert!(high > low);
    }

    #[test]
    fn non_finite_inputs_are_safely_normalized() {
        let score = retention_score(f64::NAN, 0, f64::INFINITY);
        assert!(score.is_finite());
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn threshold_is_clamped() {
        let score = retention_score(1.0, 1, 0.02);
        assert_eq!(should_retain(1.0, 1, 0.02, -10.0), score >= 0.0);
        assert_eq!(should_retain(1.0, 1, 0.02, 10.0), score >= 1.0);
    }
}
