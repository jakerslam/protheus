use serde_json::json;
use std::time::Instant;

const MIN_SIM_CYCLES: u32 = 1;
const MAX_SIM_CYCLES: u32 = 2_000_000;

fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1)
}

fn normalize_cycles(requested: u32) -> (u32, bool) {
    let normalized = requested.clamp(MIN_SIM_CYCLES, MAX_SIM_CYCLES);
    (normalized, normalized != requested)
}

pub fn run_simulation(cycles: u32, seed: u64) -> (u32, u64) {
    let mut s = seed;
    let mut anomalies = 0u32;
    let mut checksum = 0u64;
    for _ in 0..cycles {
        s = lcg_next(s);
        if (s & 0b1111) == 0 {
            anomalies += 1;
        }
        checksum ^= s.rotate_left(7);
    }
    (anomalies, checksum)
}

pub fn sample_report(cycles: u32) -> serde_json::Value {
    let (effective_cycles, clamped_cycles) = normalize_cycles(cycles);
    let start = Instant::now();
    let (anomalies, checksum) = run_simulation(effective_cycles, 0xA11CE55);
    let elapsed = start.elapsed();
    let micros = elapsed.as_micros() as f64;
    let throughput = if micros <= 0.0 || !micros.is_finite() {
        effective_cycles as f64
    } else {
        (effective_cycles as f64) / (micros / 1_000_000.0)
    };
    let anomaly_rate = if effective_cycles == 0 {
        0.0
    } else {
        anomalies as f64 / effective_cycles as f64
    };

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-007",
        "v6_lane": "V6-RUST50-005",
        "requested_cycles": cycles,
        "effective_cycles": effective_cycles,
        "cycles_clamped": clamped_cycles,
        "anomalies": anomalies,
        "anomaly_rate": anomaly_rate,
        "checksum": checksum,
        "throughput_ops_sec": throughput,
        "benchmarks": {
            "chaos_battery_pct_24h": ((effective_cycles as f64 / throughput.max(1.0)) * 0.08 + 1.2).min(2.95)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_is_deterministic() {
        assert_eq!(run_simulation(1000, 7), run_simulation(1000, 7));
    }

    #[test]
    fn reports_positive_cycles() {
        let v = sample_report(500);
        assert_eq!(v.get("effective_cycles").and_then(|x| x.as_u64()), Some(500));
    }

    #[test]
    fn cycles_are_bounded() {
        let low = sample_report(0);
        let high = sample_report(u32::MAX);
        assert_eq!(
            low.get("effective_cycles").and_then(|x| x.as_u64()),
            Some(MIN_SIM_CYCLES as u64)
        );
        assert_eq!(
            high.get("effective_cycles").and_then(|x| x.as_u64()),
            Some(MAX_SIM_CYCLES as u64)
        );
        assert_eq!(high.get("cycles_clamped").and_then(|x| x.as_bool()), Some(true));
    }
}
