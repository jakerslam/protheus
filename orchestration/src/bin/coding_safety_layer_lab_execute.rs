use infring_orchestration_v1::eval_coding_safety_layer::coding_safety_layer_lab_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = coding_safety_layer_lab_report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
