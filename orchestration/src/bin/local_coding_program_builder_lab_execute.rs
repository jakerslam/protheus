use infring_orchestration_v1::eval_local_coding_program_builder::local_coding_program_builder_lab_file_execution_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = local_coding_program_builder_lab_file_execution_report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
