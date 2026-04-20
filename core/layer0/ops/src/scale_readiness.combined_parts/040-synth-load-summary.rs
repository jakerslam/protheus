
fn synth_load_summary(stage: &str) -> Value {
    match stage {
        "10k" => {
            json!({"dau": 10_000, "peak_concurrency": 1200, "rps": 1900, "write_ratio": 0.2, "read_ratio": 0.8})
        }
        "100k" => {
            json!({"dau": 100_000, "peak_concurrency": 12_000, "rps": 16_000, "write_ratio": 0.21, "read_ratio": 0.79})
        }
        "1M" => {
            json!({"dau": 1_000_000, "peak_concurrency": 125_000, "rps": 170_000, "write_ratio": 0.22, "read_ratio": 0.78})
        }
        _ => {
            json!({"dau": 1000, "peak_concurrency": 140, "rps": 280, "write_ratio": 0.18, "read_ratio": 0.82})
        }
    }
}
