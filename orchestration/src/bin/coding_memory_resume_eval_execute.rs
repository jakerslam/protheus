#[path = "../eval_coding_memory_resume.rs"]
mod eval_coding_memory_resume;

use eval_coding_memory_resume::coding_memory_resume_proof_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = coding_memory_resume_proof_report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
