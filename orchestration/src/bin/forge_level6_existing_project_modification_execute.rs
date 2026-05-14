use infring_orchestration_v1::eval_forge_level6_existing_project_modification::{
    forge_level6_existing_project_fixture_report, forge_level6_existing_project_report_for_path,
};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root_arg = std::env::args().find_map(|arg| {
        arg.strip_prefix("--root=")
            .map(PathBuf::from)
            .or_else(|| arg.strip_prefix("--candidate-root=").map(PathBuf::from))
    });
    let report = match root_arg {
        Some(root) => forge_level6_existing_project_report_for_path(&root),
        None => forge_level6_existing_project_fixture_report(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
